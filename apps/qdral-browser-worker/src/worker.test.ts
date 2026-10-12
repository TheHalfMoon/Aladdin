import assert from "node:assert/strict";
import { test } from "node:test";
import { validLaunch } from "./fixtures.js";
import type { WorkerFrame } from "./protocol.js";
import { NavigationError, Worker, type EngineSession, type Launcher, type LaunchSpec, type RoutePolicy } from "./worker.js";

class FakeEngine implements EngineSession {
  closed = 0;
  private exitListener: (() => void) | null = null;
  navigations: string[] = [];
  navigateBehavior: (url: string) => Promise<{ url: string; status: number }> = async (url) => ({ url, status: 200 });
  async close(): Promise<void> {
    this.closed++;
  }
  navigate(url: string): Promise<{ url: string; status: number }> {
    this.navigations.push(url);
    return this.navigateBehavior(url);
  }
  observeBehavior: () => Promise<{ url: string; snapshot: { nodes: never[]; truncated: boolean } }> = async () => ({
    url: "https://fixture.test/",
    snapshot: { nodes: [], truncated: false }
  });
  observe(): Promise<{ url: string; snapshot: { nodes: never[]; truncated: boolean } }> {
    return this.observeBehavior();
  }
  activations: string[] = [];
  activateBehavior: (ref: string) => Promise<{ url: string; status: number }> = async () => ({ url: "https://fixture.test/next", status: 200 });
  activate(ref: string): Promise<{ url: string; status: number }> {
    this.activations.push(ref);
    return this.activateBehavior(ref);
  }
  downloadBehavior: (ref: string, path: string) => Promise<{ url: string; name: string; media: string; bytes: number }> = async () => ({
    url: "https://fixture.test/r.pdf",
    name: "r.pdf",
    media: "application/pdf",
    bytes: 10
  });
  download(ref: string, path: string): Promise<{ url: string; name: string; media: string; bytes: number }> {
    return this.downloadBehavior(ref, path);
  }
  onUnexpectedExit(listener: () => void): void {
    this.exitListener = listener;
  }
  crash(): void {
    this.exitListener?.();
  }
}

function harness(launchBehavior: (spec: LaunchSpec) => Promise<EngineSession> = async () => new FakeEngine()) {
  const sent: WorkerFrame[] = [];
  const exits: number[] = [];
  const launches: LaunchSpec[] = [];
  const policies: RoutePolicy[] = [];
  const launcher: Launcher = {
    launch(spec, policy) {
      launches.push(spec);
      policies.push(policy);
      return launchBehavior(spec);
    }
  };
  const worker = new Worker(launcher, (frame) => sent.push(frame), (code) => exits.push(code));
  return { worker, sent, exits, launches, policies };
}

test("hello, launch, ping and shutdown follow the closed lifecycle", async () => {
  const engine = new FakeEngine();
  const { worker, sent, exits, launches } = harness(async () => engine);
  await worker.receive({ frame: "hello", generation: 5 });
  await worker.receive(validLaunch());
  await worker.receive({ frame: "ping", nonce: "n1" });
  await worker.receive({ frame: "shutdown" });
  assert.deepEqual(sent, [
    { frame: "hello", generation: 5, worker: "qdral-browser-worker" },
    { frame: "launched" },
    { frame: "pong", nonce: "n1" },
    { frame: "bye" }
  ]);
  assert.deepEqual(exits, [0]);
  assert.equal(engine.closed, 1);
  // The launcher received the host argv unchanged.
  assert.deepEqual(launches[0]?.argv, validLaunch().argv);
  // Frames after close do nothing.
  await worker.receive({ frame: "ping", nonce: "n2" });
  assert.equal(sent.length, 4);
});

test("anything before hello, a wrong generation, or a second hello fails closed", async () => {
  for (const first of [validLaunch(), { frame: "ping", nonce: "n" }, { frame: "hello", generation: 4 }, { frame: "hello", generation: 6 }, { frame: "bogus" }]) {
    const { worker, sent, exits, launches } = harness();
    await worker.receive(first);
    assert.deepEqual(sent, [{ frame: "error", code: "protocol_violation" }]);
    assert.deepEqual(exits, [1]);
    assert.equal(launches.length, 0);
  }
  const { worker, sent, exits } = harness();
  await worker.receive({ frame: "hello", generation: 5 });
  await worker.receive({ frame: "hello", generation: 5 });
  assert.deepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" });
  assert.deepEqual(exits, [1]);
});

test("a refused launch never reaches the launcher", async () => {
  const { worker, sent, exits, launches } = harness();
  await worker.receive({ frame: "hello", generation: 5 });
  const launch = validLaunch();
  launch.argv.splice(9, 0, "--remote-debugging-port=9222");
  await worker.receive(launch);
  assert.equal(launches.length, 0);
  assert.deepEqual(sent.at(-1), { frame: "error", code: "launch_refused" });
  assert.deepEqual(exits, [1]);
});

test("only one engine per worker lifetime", async () => {
  const engine = new FakeEngine();
  const { worker, sent, exits, launches } = harness(async () => engine);
  await worker.receive({ frame: "hello", generation: 5 });
  await worker.receive(validLaunch());
  await worker.receive(validLaunch());
  assert.equal(launches.length, 1);
  assert.deepEqual(sent.at(-1), { frame: "error", code: "not_ready" });
  assert.deepEqual(exits, [1]);
  assert.equal(engine.closed, 1);
});

test("launch failure, engine exit, violations and end of input all close the engine", async () => {
  {
    const { worker, sent, exits } = harness(async () => {
      throw new Error("no engine");
    });
    await worker.receive({ frame: "hello", generation: 5 });
    await worker.receive(validLaunch());
    assert.deepEqual(sent.at(-1), { frame: "error", code: "launch_failed" });
    assert.deepEqual(exits, [1]);
  }
  {
    const engine = new FakeEngine();
    const { worker, sent, exits } = harness(async () => engine);
    await worker.receive({ frame: "hello", generation: 5 });
    await worker.receive(validLaunch());
    engine.crash();
    await worker.receive({ frame: "ping", nonce: "after" });
    assert.deepEqual(sent.at(-1), { frame: "error", code: "engine_exited" });
    assert.deepEqual(exits, [1]);
  }
  {
    const engine = new FakeEngine();
    const { worker, sent, exits } = harness(async () => engine);
    await worker.receive({ frame: "hello", generation: 5 });
    await worker.receive(validLaunch());
    await worker.violation();
    assert.deepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" });
    assert.deepEqual(exits, [1]);
    assert.equal(engine.closed, 1);
  }
  {
    const engine = new FakeEngine();
    const { worker, sent, exits } = harness(async () => engine);
    await worker.receive({ frame: "hello", generation: 5 });
    await worker.receive(validLaunch());
    await worker.endOfInput();
    assert.deepEqual(sent.at(-1), { frame: "launched" });
    assert.deepEqual(exits, [0]);
    assert.equal(engine.closed, 1);
  }
});

test("a shutdown queued behind a slow launch closes the launched engine", async () => {
  const engine = new FakeEngine();
  let release: () => void = () => {};
  const gate = new Promise<void>((resolve) => (release = resolve));
  const { worker, sent, exits } = harness(async () => {
    await gate;
    return engine;
  });
  await worker.receive({ frame: "hello", generation: 5 });
  const launching = worker.receive(validLaunch());
  const shutdown = worker.receive({ frame: "shutdown" });
  release();
  await launching;
  await shutdown;
  assert.deepEqual(sent.slice(1), [{ frame: "launched" }, { frame: "bye" }]);
  assert.deepEqual(exits, [0]);
  assert.equal(engine.closed, 1);
});

test("an unexpected error in a step fails the worker closed exactly once", async () => {
  const engine = new FakeEngine();
  const exits: number[] = [];
  let sends = 0;
  const worker = new Worker(
    { launch: async () => engine },
    () => {
      sends++;
      if (sends === 3) throw new Error("broken pipe");
    },
    (code) => exits.push(code)
  );
  await worker.receive({ frame: "hello", generation: 5 });
  await worker.receive(validLaunch());
  await worker.receive({ frame: "ping", nonce: "boom" });
  await worker.receive({ frame: "ping", nonce: "after" });
  assert.deepEqual(exits, [1]);
  assert.equal(engine.closed, 1);
});

async function launched(engine = new FakeEngine()) {
  const h = harness(async () => engine);
  await h.worker.receive({ frame: "hello", generation: 5 });
  await h.worker.receive(validLaunch());
  return { ...h, engine, policy: h.policies[0] as RoutePolicy };
}

const settle = (): Promise<void> => new Promise((resolve) => setImmediate(resolve));

test("navigation runs outside the queue and reports its outcome", async () => {
  const { worker, sent, engine } = await launched();
  let finish: (value: { url: string; status: number }) => void = () => {};
  engine.navigateBehavior = () => new Promise((resolve) => (finish = resolve));
  await worker.receive({ frame: "navigate", nav_id: 1, url: "https://fixture.test/a" });
  // Ping is answered while the navigation is still running.
  await worker.receive({ frame: "ping", nonce: "during" });
  assert.deepEqual(sent.at(-1), { frame: "pong", nonce: "during" });
  // A second navigation while one runs is refused as busy.
  await worker.receive({ frame: "navigate", nav_id: 2, url: "https://fixture.test/b" });
  assert.deepEqual(sent.at(-1), { frame: "navigation_failed", nav_id: 2, reason: "busy" });
  finish({ url: "https://fixture.test/a", status: 200 });
  await settle();
  assert.deepEqual(sent.at(-1), { frame: "navigated", nav_id: 1, url: "https://fixture.test/a", status: 200 });
  assert.deepEqual(engine.navigations, ["https://fixture.test/a"]);
});

test("navigation failures are typed", async () => {
  for (const [error, reason] of [
    [new NavigationError("denied"), "denied"],
    [new NavigationError("timeout"), "timeout"],
    [new Error("anything else"), "failed"]
  ] as const) {
    const { worker, sent, engine } = await launched();
    engine.navigateBehavior = async () => {
      throw error;
    };
    await worker.receive({ frame: "navigate", nav_id: 7, url: "https://fixture.test/x" });
    await settle();
    assert.deepEqual(sent.at(-1), { frame: "navigation_failed", nav_id: 7, reason });
  }
});

test("navigate before launch fails closed", async () => {
  const { worker, sent, exits } = harness();
  await worker.receive({ frame: "hello", generation: 5 });
  await worker.receive({ frame: "navigate", nav_id: 1, url: "https://fixture.test/" });
  assert.deepEqual(sent.at(-1), { frame: "error", code: "not_ready" });
  assert.deepEqual(exits, [1]);
});

test("route requests are described to the host and only its answer applies", async () => {
  const { worker, sent, policy } = await launched();
  const allowed = policy.decide({ kind: "document", url: "https://fixture.test/a", origin: "" });
  const refused = policy.decide({ kind: "frame", url: "https://other.test/f", origin: "https://fixture.test" });
  assert.deepEqual(sent.slice(-2), [
    { frame: "route_request", request_id: 1, kind: "document", url: "https://fixture.test/a", origin: "" },
    { frame: "route_request", request_id: 2, kind: "frame", url: "https://other.test/f", origin: "https://fixture.test" }
  ]);
  // Decisions bypass the queue and may arrive in any order.
  await worker.receive({ frame: "route_decision", request_id: 2, allow: false });
  await worker.receive({ frame: "route_decision", request_id: 1, allow: true });
  assert.equal(await allowed, true);
  assert.equal(await refused, false);
  // A late or duplicate answer for a known id is ignored; an id never
  // issued is a violation.
  await worker.receive({ frame: "route_decision", request_id: 1, allow: true });
  assert.notDeepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" });
  await worker.receive({ frame: "route_decision", request_id: 99, allow: true });
  assert.deepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" });
});

test("oversized or control-character requests are refused without asking", async () => {
  const { sent, policy } = await launched();
  const before = sent.length;
  assert.equal(await policy.decide({ kind: "subresource", url: `https://fixture.test/${"a".repeat(3000)}`, origin: "https://fixture.test" }), false);
  assert.equal(await policy.decide({ kind: "subresource", url: "https://fixture.test/\u0000", origin: "https://fixture.test" }), false);
  assert.equal(await policy.decide({ kind: "document", url: "", origin: "" }), false);
  assert.equal(sent.length, before, "a refused request reached the host");
});

test("pending decisions are refused when the worker closes", async () => {
  const { worker, policy } = await launched();
  const waiting = policy.decide({ kind: "document", url: "https://fixture.test/a", origin: "" });
  await worker.receive({ frame: "shutdown" });
  assert.equal(await waiting, false);
  // Nothing is asked after close.
  assert.equal(await policy.decide({ kind: "document", url: "https://fixture.test/b", origin: "" }), false);
});

test("a route decision before launch is a violation", async () => {
  const { worker, sent, exits } = harness();
  await worker.receive({ frame: "hello", generation: 5 });
  await worker.receive({ frame: "route_decision", request_id: 1, allow: true });
  assert.deepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" });
  assert.deepEqual(exits, [1]);
});

test("observe reports a bounded snapshot or a typed failure", async () => {
  const { worker, sent, engine } = await launched();
  engine.observeBehavior = async () => ({ url: "https://fixture.test/r", snapshot: { nodes: [], truncated: true } });
  await worker.receive({ frame: "observe", obs_id: 3 });
  assert.deepEqual(sent.at(-1), { frame: "observed", obs_id: 3, url: "https://fixture.test/r", truncated: true, nodes: [] });
  engine.observeBehavior = async () => {
    throw new Error("snapshot timed out");
  };
  await worker.receive({ frame: "observe", obs_id: 4 });
  assert.deepEqual(sent.at(-1), { frame: "observation_failed", obs_id: 4 });
});

test("observe before launch fails closed", async () => {
  const { worker, sent, exits } = harness();
  await worker.receive({ frame: "hello", generation: 5 });
  await worker.receive({ frame: "observe", obs_id: 1 });
  assert.deepEqual(sent.at(-1), { frame: "error", code: "not_ready" });
  assert.deepEqual(exits, [1]);
});

test("activation runs outside the queue and reports a typed outcome", async () => {
  const { worker, sent, engine } = await launched();
  await worker.receive({ frame: "activate", act_id: 1, ref: "e5" });
  await settle();
  assert.deepEqual(sent.at(-1), { frame: "activated", act_id: 1, url: "https://fixture.test/next", status: 200 });
  assert.deepEqual(engine.activations, ["e5"]);
  engine.activateBehavior = async () => {
    throw new NavigationError("denied");
  };
  await worker.receive({ frame: "activate", act_id: 2, ref: "f1e2" });
  await settle();
  assert.deepEqual(sent.at(-1), { frame: "activation_failed", act_id: 2, reason: "denied" });
});

test("malformed activation refs are protocol violations", async () => {
  for (const ref of ["", "e", "button", "e5;drop", "../e1", "f1", "e1234567"]) {
    const { worker, sent } = await launched();
    await worker.receive({ frame: "activate", act_id: 1, ref });
    assert.deepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" }, ref);
  }
});
