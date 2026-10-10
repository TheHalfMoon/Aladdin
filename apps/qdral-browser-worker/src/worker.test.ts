import assert from "node:assert/strict";
import { test } from "node:test";
import { validLaunch } from "./fixtures.js";
import type { WorkerFrame } from "./protocol.js";
import { Worker, type EngineSession, type Launcher, type LaunchSpec } from "./worker.js";

class FakeEngine implements EngineSession {
  closed = 0;
  private exitListener: (() => void) | null = null;
  async close(): Promise<void> {
    this.closed++;
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
  const launcher: Launcher = {
    launch(spec) {
      launches.push(spec);
      return launchBehavior(spec);
    }
  };
  const worker = new Worker(launcher, (frame) => sent.push(frame), (code) => exits.push(code));
  return { worker, sent, exits, launches };
}

test("hello, launch, ping and shutdown follow the closed lifecycle", async () => {
  const engine = new FakeEngine();
  const { worker, sent, exits, launches } = harness(async () => engine);
  await worker.receive({ frame: "hello", generation: 1 });
  await worker.receive(validLaunch());
  await worker.receive({ frame: "ping", nonce: "n1" });
  await worker.receive({ frame: "shutdown" });
  assert.deepEqual(sent, [
    { frame: "hello", generation: 1, worker: "qdral-browser-worker" },
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
  for (const first of [validLaunch(), { frame: "ping", nonce: "n" }, { frame: "hello", generation: 2 }, { frame: "bogus" }]) {
    const { worker, sent, exits, launches } = harness();
    await worker.receive(first);
    assert.deepEqual(sent, [{ frame: "error", code: "protocol_violation" }]);
    assert.deepEqual(exits, [1]);
    assert.equal(launches.length, 0);
  }
  const { worker, sent, exits } = harness();
  await worker.receive({ frame: "hello", generation: 1 });
  await worker.receive({ frame: "hello", generation: 1 });
  assert.deepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" });
  assert.deepEqual(exits, [1]);
});

test("a refused launch never reaches the launcher", async () => {
  const { worker, sent, exits, launches } = harness();
  await worker.receive({ frame: "hello", generation: 1 });
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
  await worker.receive({ frame: "hello", generation: 1 });
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
    await worker.receive({ frame: "hello", generation: 1 });
    await worker.receive(validLaunch());
    assert.deepEqual(sent.at(-1), { frame: "error", code: "launch_failed" });
    assert.deepEqual(exits, [1]);
  }
  {
    const engine = new FakeEngine();
    const { worker, sent, exits } = harness(async () => engine);
    await worker.receive({ frame: "hello", generation: 1 });
    await worker.receive(validLaunch());
    engine.crash();
    await worker.receive({ frame: "ping", nonce: "after" });
    assert.deepEqual(sent.at(-1), { frame: "error", code: "engine_exited" });
    assert.deepEqual(exits, [1]);
  }
  {
    const engine = new FakeEngine();
    const { worker, sent, exits } = harness(async () => engine);
    await worker.receive({ frame: "hello", generation: 1 });
    await worker.receive(validLaunch());
    await worker.violation();
    assert.deepEqual(sent.at(-1), { frame: "error", code: "protocol_violation" });
    assert.deepEqual(exits, [1]);
    assert.equal(engine.closed, 1);
  }
  {
    const engine = new FakeEngine();
    const { worker, sent, exits } = harness(async () => engine);
    await worker.receive({ frame: "hello", generation: 1 });
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
  await worker.receive({ frame: "hello", generation: 1 });
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
  await worker.receive({ frame: "hello", generation: 1 });
  await worker.receive(validLaunch());
  await worker.receive({ frame: "ping", nonce: "boom" });
  await worker.receive({ frame: "ping", nonce: "after" });
  assert.deepEqual(exits, [1]);
  assert.equal(engine.closed, 1);
});
