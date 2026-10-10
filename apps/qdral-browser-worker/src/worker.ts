// SG-000108 worker state machine. Transport-free: frames arrive through
// `receive`, replies leave through `send`, and the engine is behind an
// injected `Launcher` so the protocol is testable without a browser.
//
// awaiting_hello --hello--> ready --launch--> launched --shutdown--> closed
// Any violation, refused launch, engine exit, or end of input closes the
// engine and ends the worker. At most one engine per worker lifetime.

import {
  launchRefusal,
  parseHostFrame,
  ProtocolViolation,
  WORKER_IDENTITY,
  WORKER_PROTOCOL_GENERATION,
  type ErrorCode,
  type HostFrame,
  type WorkerFrame
} from "./protocol.js";

export type LaunchSpec = Extract<HostFrame, { frame: "launch" }>;

export interface EngineSession {
  close(): Promise<void>;
  /// Called once if the engine goes away without `close`.
  onUnexpectedExit(listener: () => void): void;
}

export interface Launcher {
  launch(spec: LaunchSpec): Promise<EngineSession>;
}

export type WorkerState = "awaiting_hello" | "ready" | "launched" | "closed";

export class Worker {
  private state: WorkerState = "awaiting_hello";
  private session: EngineSession | null = null;
  private launchedOnce = false;
  private queue: Promise<void> = Promise.resolve();

  constructor(
    private readonly launcher: Launcher,
    private readonly send: (frame: WorkerFrame) => void,
    private readonly finish: (exitCode: number) => void
  ) {}

  get currentState(): WorkerState {
    return this.state;
  }

  /// Frames are handled strictly in arrival order.
  receive(raw: unknown): Promise<void> {
    this.queue = this.queue.then(() => this.handle(raw));
    return this.queue;
  }

  /// A transport-level violation (bad length, non-JSON) or end of input.
  violation(): Promise<void> {
    this.queue = this.queue.then(() => this.fail("protocol_violation"));
    return this.queue;
  }

  endOfInput(): Promise<void> {
    this.queue = this.queue.then(() => this.close(0, null));
    return this.queue;
  }

  private async handle(raw: unknown): Promise<void> {
    if (this.state === "closed") return;
    let frame: HostFrame;
    try {
      frame = parseHostFrame(raw);
    } catch (error) {
      if (error instanceof ProtocolViolation) return this.fail("protocol_violation");
      throw error;
    }
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
        return this.fail("protocol_violation");
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
          session = await this.launcher.launch(frame);
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
            this.queue = this.queue.then(() => this.fail("engine_exited"));
          }
        });
        this.send({ frame: "launched" });
        return;
      }
    }
  }

  private fail(code: ErrorCode): Promise<void> {
    return this.close(1, { frame: "error", code });
  }

  private async close(exitCode: number, last: WorkerFrame | null): Promise<void> {
    if (this.state === "closed") return;
    this.state = "closed";
    const session = this.session;
    this.session = null;
    if (session) {
      try {
        await session.close();
      } catch {
        // The engine is gone either way; the host's Job Object reaps it.
      }
    }
    if (last) this.send(last);
    this.finish(exitCode);
  }
}
