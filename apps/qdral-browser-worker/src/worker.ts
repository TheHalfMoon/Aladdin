// SG-000108 worker state machine. Transport-free: frames arrive through
// `receive`, replies leave through `send`, and the engine is behind an
// injected `Launcher` so the protocol is testable without a browser.
//
// awaiting_hello --hello--> ready --launch--> launched --shutdown--> closed
// Any violation, refused launch, engine exit, or end of input closes the
// engine and ends the worker. At most one engine per worker lifetime.
//
// Slice 3a: in `launched`, the host may send `navigate`. The navigation runs
// outside the frame queue (so ping and shutdown stay responsive) and ends
// with `navigated` or `navigation_failed`. Every engine request is described
// to the host as a `route_request`; the host's `route_decision` is applied
// immediately (decisions bypass the queue, or a navigation waiting for them
// would deadlock). A request the host does not answer in time is refused.

import {
  launchRefusal,
  MAX_ORIGIN_BYTES,
  MAX_URL_BYTES,
  parseHostFrame,
  ProtocolViolation,
  WORKER_IDENTITY,
  WORKER_PROTOCOL_GENERATION,
  type ErrorCode,
  type HostFrame,
  type NavigationFailure,
  type WorkerFrame
} from "./protocol.js";
import type { RouteQuery } from "./routing.js";
import type { BoundedSnapshot } from "./snapshot.js";

export type LaunchSpec = Extract<HostFrame, { frame: "launch" }>;

/// How long the worker waits for a host route decision before refusing.
export const ROUTE_DECISION_TIMEOUT_MS = 10_000;

/// The host's answer to a described request.
export interface RoutePolicy {
  decide(query: RouteQuery): Promise<boolean>;
}

export class NavigationError extends Error {
  constructor(readonly reason: NavigationFailure) {
    super(reason);
  }
}

export interface EngineSession {
  close(): Promise<void>;
  /// Navigate the primary page; rejects with NavigationError.
  navigate(url: string): Promise<{ url: string; status: number }>;
  /// A bounded, value-free accessibility snapshot of the primary page.
  observe(): Promise<{ url: string; snapshot: BoundedSnapshot }>;
  /// Click the element with this snapshot ref and wait for the resulting
  /// navigation to load; rejects with NavigationError.
  activate(ref: string): Promise<{ url: string; status: number }>;
  /// Click the element with this snapshot ref, expect a download, and save
  /// it to `path` (which must not exist); rejects with NavigationError.
  download(ref: string, path: string): Promise<{ url: string; name: string; media: string; bytes: number }>;
  /// Called once if the engine goes away without `close`.
  onUnexpectedExit(listener: () => void): void;
}

export interface Launcher {
  launch(spec: LaunchSpec, policy: RoutePolicy): Promise<EngineSession>;
}

export type WorkerState = "awaiting_hello" | "ready" | "launched" | "closed";

export class Worker {
  private state: WorkerState = "awaiting_hello";
  private session: EngineSession | null = null;
  private launchedOnce = false;
  private queue: Promise<void> = Promise.resolve();
  private navigating = false;
  private nextRequestId = 0;
  private readonly pending = new Map<number, { resolve: (allow: boolean) => void; timer: NodeJS.Timeout }>();
  private readonly policy: RoutePolicy = { decide: (query) => this.ask(query) };

  constructor(
    private readonly launcher: Launcher,
    private readonly send: (frame: WorkerFrame) => void,
    private readonly finish: (exitCode: number) => void
  ) {}

  /// Frames are handled strictly in arrival order. An unexpected error in
  /// any step fails the worker closed instead of breaking the queue.
  receive(raw: unknown): Promise<void> {
    let frame: HostFrame;
    try {
      frame = parseHostFrame(raw);
    } catch (error) {
      if (error instanceof ProtocolViolation) return this.enqueue(() => this.fail("protocol_violation"));
      throw error;
    }
    if (frame.frame === "route_decision") {
      // Applied at once: a navigation in progress may be waiting for it.
      const waiter = this.pending.get(frame.request_id);
      if (this.state !== "launched" || frame.request_id > this.nextRequestId) {
        return this.enqueue(() => this.fail("protocol_violation"));
      }
      if (waiter) {
        this.pending.delete(frame.request_id);
        clearTimeout(waiter.timer);
        waiter.resolve(frame.allow);
      }
      // A late answer to a request that already timed out is ignored.
      return Promise.resolve();
    }
    return this.enqueue(() => this.handle(frame));
  }

  /// Describe a request to the host and wait (bounded) for its decision.
  private ask(query: RouteQuery): Promise<boolean> {
    if (this.state !== "launched") return Promise.resolve(false);
    if (
      query.url.length === 0 ||
      Buffer.byteLength(query.url, "utf8") > MAX_URL_BYTES ||
      Buffer.byteLength(query.origin, "utf8") > MAX_ORIGIN_BYTES ||
      /[\u0000-\u001f\u007f]/.test(query.url + query.origin)
    ) {
      return Promise.resolve(false);
    }
    const requestId = ++this.nextRequestId;
    return new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        resolve(false);
      }, ROUTE_DECISION_TIMEOUT_MS);
      this.pending.set(requestId, { resolve, timer });
      this.send({ frame: "route_request", request_id: requestId, kind: query.kind, url: query.url, origin: query.origin });
    });
  }

  /// A transport-level violation (bad length, non-JSON, broken pipe).
  violation(): Promise<void> {
    return this.enqueue(() => this.fail("protocol_violation"));
  }

  endOfInput(): Promise<void> {
    return this.enqueue(() => this.close(0, null));
  }

  private enqueue(step: () => Promise<void>): Promise<void> {
    this.queue = this.queue.then(step).catch(() => this.fail("protocol_violation"));
    return this.queue;
  }

  private async handle(frame: HostFrame): Promise<void> {
    if (this.state === "closed") return;
    if (this.state === "awaiting_hello") {
      if (frame.frame !== "hello" || frame.generation !== WORKER_PROTOCOL_GENERATION) {
        return this.fail("protocol_violation");
      }
      this.state = "ready";
      this.send({ frame: "hello", generation: WORKER_PROTOCOL_GENERATION, worker: WORKER_IDENTITY });
      return;
    }
    switch (frame.frame) {
      case "hello":
      case "route_decision":
        return this.fail("protocol_violation");
      case "download": {
        if (this.state !== "launched" || !this.session) return this.fail("not_ready");
        if (this.navigating) {
          this.send({ frame: "download_failed", dl_id: frame.dl_id, reason: "busy" });
          return;
        }
        this.navigating = true;
        void this.runDownload(this.session, frame.dl_id, frame.ref, frame.path);
        return;
      }
      case "activate": {
        if (this.state !== "launched" || !this.session) return this.fail("not_ready");
        if (this.navigating) {
          this.send({ frame: "activation_failed", act_id: frame.act_id, reason: "busy" });
          return;
        }
        this.navigating = true;
        void this.runActivation(this.session, frame.act_id, frame.ref);
        return;
      }
      case "observe": {
        if (this.state !== "launched" || !this.session) return this.fail("not_ready");
        try {
          const { url, snapshot } = await this.session.observe();
          this.send({ frame: "observed", obs_id: frame.obs_id, url, truncated: snapshot.truncated, nodes: snapshot.nodes });
        } catch {
          this.send({ frame: "observation_failed", obs_id: frame.obs_id });
        }
        return;
      }
      case "navigate": {
        if (this.state !== "launched" || !this.session) return this.fail("not_ready");
        if (this.navigating) {
          this.send({ frame: "navigation_failed", nav_id: frame.nav_id, reason: "busy" });
          return;
        }
        this.navigating = true;
        void this.runNavigation(this.session, frame.nav_id, frame.url);
        return;
      }
      case "ping":
        this.send({ frame: "pong", nonce: frame.nonce });
        return;
      case "shutdown":
        return this.close(0, { frame: "bye" });
      case "launch": {
        if (this.state !== "ready" || this.launchedOnce) return this.fail("not_ready");
        this.launchedOnce = true;
        if (launchRefusal(frame) !== null) return this.fail("launch_refused");
        let session: EngineSession;
        try {
          session = await this.launcher.launch(frame, this.policy);
        } catch {
          return this.fail("launch_failed");
        }
        // A shutdown or violation may not interleave (frames are queued),
        // so the worker is still `ready` here.
        this.session = session;
        this.state = "launched";
        session.onUnexpectedExit(() => {
          if (this.session === session) {
            this.session = null;
            void this.enqueue(() => this.fail("engine_exited"));
          }
        });
        this.send({ frame: "launched" });
        return;
      }
    }
  }

  private async runNavigation(session: EngineSession, navId: number, url: string): Promise<void> {
    let outcome: WorkerFrame;
    try {
      const result = await session.navigate(url);
      outcome = { frame: "navigated", nav_id: navId, url: result.url, status: result.status };
    } catch (error) {
      const reason: NavigationFailure = error instanceof NavigationError ? error.reason : "failed";
      outcome = { frame: "navigation_failed", nav_id: navId, reason };
    }
    this.navigating = false;
    if (this.state === "launched" && this.session === session) this.send(outcome);
  }

  private async runDownload(session: EngineSession, dlId: number, ref: string, path: string): Promise<void> {
    let outcome: WorkerFrame;
    try {
      const result = await session.download(ref, path);
      outcome = { frame: "downloaded", dl_id: dlId, ...result };
    } catch (error) {
      const reason: NavigationFailure = error instanceof NavigationError ? error.reason : "failed";
      outcome = { frame: "download_failed", dl_id: dlId, reason };
    }
    this.navigating = false;
    if (this.state === "launched" && this.session === session) this.send(outcome);
  }

  private async runActivation(session: EngineSession, actId: number, ref: string): Promise<void> {
    let outcome: WorkerFrame;
    try {
      const result = await session.activate(ref);
      outcome = { frame: "activated", act_id: actId, url: result.url, status: result.status };
    } catch (error) {
      const reason: NavigationFailure = error instanceof NavigationError ? error.reason : "failed";
      outcome = { frame: "activation_failed", act_id: actId, reason };
    }
    this.navigating = false;
    if (this.state === "launched" && this.session === session) this.send(outcome);
  }

  private fail(code: ErrorCode): Promise<void> {
    return this.close(1, { frame: "error", code });
  }

  private async close(exitCode: number, last: WorkerFrame | null): Promise<void> {
    if (this.state === "closed") return;
    this.state = "closed";
    for (const [, waiter] of this.pending) {
      clearTimeout(waiter.timer);
      waiter.resolve(false);
    }
    this.pending.clear();
    // From here on nothing can throw out of close: the engine is closed best
    // effort and the worker always finishes exactly once.
    const session = this.session;
    this.session = null;
    if (session) {
      try {
        await session.close();
      } catch {
        // The engine is gone either way; the host's Job Object reaps it.
      }
    }
    try {
      if (last) this.send(last);
    } finally {
      this.finish(exitCode);
    }
  }
}
