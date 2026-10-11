// SG-000108 native confinement probes against the installed Edge.
//
// These run the real engine with the exact confinement argv (the shape that
// crates/qdral-browser-host/src/confinement.rs builds and that launchRefusal
// accepts). Layers 1 (resolver pinning) and 2 (no-route proxy) are probed
// with no policy route at all, as the decision record requires. On Windows a
// missing engine fails the suite; elsewhere the suite is skipped because only
// Windows results count as evidence. Fixtures are owned local servers, and
// each probe that could pass vacuously has a positive control.

import assert from "node:assert/strict";
import { spawn, execFileSync, type ChildProcess } from "node:child_process";
import { createSocket } from "node:dgram";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { createServer as createHttpServer, type Server } from "node:http";
import { createServer as createTcpServer, connect, type Server as TcpServer } from "node:net";
import { networkInterfaces, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { chromium, type BrowserContext } from "playwright-core";
import { splitWindowsCommandLine } from "./cmdline.js";
import { encodeFrame, FrameDecoder, launchRefusal, type HostFrame, type WorkerFrame } from "./protocol.js";

const EDGE_CANDIDATES = [
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe"
];
const isWindows = process.platform === "win32";
const engine = EDGE_CANDIDATES.find((path) => existsSync(path));
const skip = isWindows ? false : "native probes are Windows-only evidence";
const WORKER_MAIN = join(dirname(fileURLToPath(import.meta.url)), "main.js");
const FIXTURE_PORT = 443;
const WEBRTC_POLICY = "--webrtc-ip-handling-policy=disable_non_proxied_udp";

const FROZEN = [
  "--headless",
  "--no-first-run",
  "--no-default-browser-check",
  "--disable-extensions",
  "--disable-background-networking",
  "--disable-sync",
  "--no-service-autorun"
];

interface Destination {
  host: string;
  pin: string;
}

/// The confinement argv, built exactly as confinement.rs builds it.
function confinedArgv(profile: string, destinations: Destination[]): string[] {
  const rules = destinations
    .map(({ host, pin }) => `MAP ${host} ${pin.includes(":") ? `[${pin}]` : pin}:${FIXTURE_PORT}`)
    .concat("MAP * ~NOTFOUND")
    .join(", ");
  const bypass = [...destinations.map((d) => d.host), "<-loopback>"].join(";");
  return [
    ...FROZEN,
    `--user-data-dir=${profile}`,
    "--remote-debugging-pipe",
    `--host-resolver-rules=${rules}`,
    "--proxy-server=http://0.0.0.0:9",
    `--proxy-bypass-list=${bypass}`,
    WEBRTC_POLICY,
    "--disable-quic",
    "about:blank"
  ];
}

const FIXTURE: Destination[] = [{ host: "fixture.test", pin: "127.0.0.1" }];

/// Per-test cleanup that runs last-in, first-out and always runs every step
/// (node:test runs `t.after` hooks in registration order and a failing hook
/// skips the rest, which would leave an engine or a server alive). The test
/// body's own failure takes precedence over a cleanup failure.
class Cleanup {
  private readonly steps: Array<() => unknown> = [];

  after(step: () => unknown): void {
    this.steps.push(step);
  }

  async run(): Promise<void> {
    let first: unknown = null;
    for (const step of this.steps.reverse()) {
      try {
        await step();
      } catch (error) {
        first ??= error;
      }
    }
    if (first !== null) throw first;
  }
}

function nativeTest(name: string, body: (t: Cleanup) => Promise<void>): void {
  test(name, { skip, timeout: 120_000 }, async () => {
    const cleanup = new Cleanup();
    let failure: unknown = null;
    try {
      await body(cleanup);
    } catch (error) {
      failure = error;
    }
    try {
      await cleanup.run();
    } catch (error) {
      failure ??= error;
    }
    if (failure !== null) throw failure;
  });
}

function requireEngine(): string {
  assert.ok(engine, `no Edge engine found at ${EDGE_CANDIDATES.join(" or ")}`);
  return engine;
}

function cleanEnv(): { SystemRoot: string; SystemDrive: string; PATH: string; TEMP: string; TMP: string } {
  const root = process.env.SystemRoot ?? "C:\\Windows";
  return {
    SystemRoot: root,
    SystemDrive: process.env.SystemDrive ?? "C:",
    PATH: `${root}\\System32`,
    TEMP: process.env.TEMP ?? `${root}\\Temp`,
    TMP: process.env.TMP ?? `${root}\\Temp`
  };
}

function launchFrame(profile: string, destinations: Destination[]): Extract<HostFrame, { frame: "launch" }> {
  const env = cleanEnv();
  const frame = {
    frame: "launch" as const,
    engine: requireEngine(),
    profile_dir: profile,
    argv: confinedArgv(profile, destinations),
    env: { SystemRoot: env.SystemRoot, PATH: env.PATH, TEMP: env.TEMP }
  };
  assert.equal(launchRefusal(frame), null, "the probe argv must satisfy the worker contract");
  return frame;
}

/// Command lines of engine processes using `profile` (every engine child
/// carries --user-data-dir with the profile). A failing query throws instead
/// of reporting "no processes".
function engineCommandLines(profile: string): string[] {
  const leaf = profile.split("\\").pop() ?? profile;
  const out = execFileSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like '*${leaf}*' } | ForEach-Object { $_.CommandLine }`
    ],
    { encoding: "utf8" }
  );
  return out.split(/\r?\n/).filter((line) => line.trim().length > 0);
}

async function waitFor(condition: () => boolean, timeoutMs: number): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (condition()) return true;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  return condition();
}

/// A fresh profile removed only after every engine process using it is gone,
/// so cleanup never replaces the real failure with a locked-file error.
function profileFor(t: Cleanup, label: string): string {
  const profile = mkdtempSync(join(tmpdir(), `sg108-${label}-`));
  t.after(async () => {
    await waitFor(() => engineCommandLines(profile).length === 0, 15_000);
    rmSync(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
  });
  return profile;
}

class WorkerProcess {
  readonly child: ChildProcess;
  readonly replies: WorkerFrame[] = [];
  private readonly decoder = new FrameDecoder();
  private waiters: Array<() => void> = [];
  readonly exited: Promise<number | null>;

  constructor(t: Cleanup, nodeFlags: string[] = ["--disable-sigusr1"]) {
    this.child = spawn(process.execPath, [...nodeFlags, WORKER_MAIN], { env: cleanEnv(), stdio: ["pipe", "pipe", "pipe"] });
    this.child.stdout?.on("data", (chunk: Buffer) => {
      for (const frame of this.decoder.push(chunk)) this.replies.push(frame as WorkerFrame);
      for (const waiter of this.waiters.splice(0)) waiter();
    });
    this.child.stderr?.resume();
    this.exited = new Promise((resolve) => this.child.on("exit", (code) => resolve(code)));
    t.after(() => this.stop());
  }

  send(frame: HostFrame): void {
    this.child.stdin?.write(encodeFrame(frame));
  }

  async reply(count: number, timeoutMs = 45_000): Promise<WorkerFrame> {
    const deadline = Date.now() + timeoutMs;
    while (this.replies.length < count) {
      assert.ok(Date.now() < deadline, `timed out waiting for reply ${count}: ${JSON.stringify(this.replies)}`);
      await new Promise<void>((resolve) => {
        this.waiters.push(resolve);
        setTimeout(resolve, 500);
      });
    }
    return this.replies[count - 1] as WorkerFrame;
  }

  async stop(): Promise<void> {
    if (this.child.exitCode === null && this.child.signalCode === null) this.child.kill();
    await this.exited;
  }
}

function listen(t: Cleanup, server: Server | TcpServer, port: number, host: string): Promise<void> {
  // Track sockets so teardown never waits on a keep-alive connection held by
  // an engine that is closed later in the same teardown.
  const sockets = new Set<import("node:net").Socket>();
  server.on("connection", (socket: import("node:net").Socket) => {
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
  });
  t.after(
    () =>
      new Promise<void>((resolve) => {
        for (const socket of sockets) socket.destroy();
        server.close(() => resolve());
      })
  );
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen({ port, host, ipv6Only: host === "::" }, () => resolve());
  });
}

async function launchDirect(t: Cleanup, profile: string, args: string[]): Promise<BrowserContext> {
  const context = await chromium.launchPersistentContext(profile, {
    executablePath: requireEngine(),
    ignoreDefaultArgs: true,
    args,
    env: cleanEnv(),
    headless: true,
    timeout: 30_000
  });
  t.after(() => context.close());
  return context;
}

nativeTest("the worker drives the installed engine with exactly the host argv and leaves nothing behind", async (t) => {
  const profile = profileFor(t, "native");
  const worker = new WorkerProcess(t);
  worker.send({ frame: "hello", generation: 1 });
  assert.deepEqual(await worker.reply(1), { frame: "hello", generation: 1, worker: "qdral-browser-worker" });
  const launch = launchFrame(profile, FIXTURE);
  worker.send(launch);
  assert.deepEqual(await worker.reply(2), { frame: "launched" });

  const lines = engineCommandLines(profile);
  const browsers = lines.filter((line) => !line.includes("--type="));
  assert.equal(browsers.length, 1, `exactly one browser process: ${JSON.stringify(browsers)}`);
  assert.deepEqual(splitWindowsCommandLine(browsers[0] ?? ""), [launch.engine, ...launch.argv]);
  for (const line of lines) {
    for (const forbidden of ["--remote-debugging-port", "--remote-debugging-address", "--enable-automation"]) {
      assert.equal(line.includes(forbidden), false, `${forbidden} in ${line}`);
    }
  }
  worker.send({ frame: "ping", nonce: "native" });
  assert.deepEqual(await worker.reply(3), { frame: "pong", nonce: "native" });
  worker.send({ frame: "shutdown" });
  assert.deepEqual(await worker.reply(4), { frame: "bye" });
  assert.equal(await worker.exited, 0);
  assert.ok(await waitFor(() => engineCommandLines(profile).length === 0, 10_000), "engine processes remain after shutdown");
});

nativeTest("killing the worker takes the engine down with its pipe", async (t) => {
  const profile = profileFor(t, "native-kill");
  const worker = new WorkerProcess(t);
  worker.send({ frame: "hello", generation: 1 });
  await worker.reply(1);
  worker.send(launchFrame(profile, FIXTURE));
  assert.deepEqual(await worker.reply(2), { frame: "launched" });
  assert.ok(engineCommandLines(profile).length > 0, "the engine is running before the kill");
  worker.child.kill("SIGKILL");
  await worker.exited;
  // The host's kill-on-close Job Object (slice 2b-ii) is the guarantee;
  // this proves the engine also exits on its own when the pipe closes.
  assert.ok(await waitFor(() => engineCommandLines(profile).length === 0, 15_000), "engine survived the worker");
});

nativeTest("resolver pinning and the no-route proxy confine traffic without any policy route", async (t) => {
  const hits: string[] = [];
  const fixture = createHttpServer((request, response) => {
    hits.push(`${request.headers.host ?? ""}${request.url ?? ""}`);
    response.setHeader("content-type", "text/html");
    response.end("<p>fixture</p>");
  });
  await listen(t, fixture, FIXTURE_PORT, "127.0.0.1");
  const accepts: Record<string, number> = {};
  for (const host of ["0.0.0.0", "127.0.0.1", "::"]) {
    const trap = createTcpServer((socket) => {
      accepts[host] = (accepts[host] ?? 0) + 1;
      socket.destroy();
    });
    await listen(t, trap, 9, host);
    accepts[host] = 0;
  }
  const profile = profileFor(t, "native-net");
  const context = await launchDirect(t, profile, confinedArgv(profile, FIXTURE));
  const open = async (url: string): Promise<string> => {
    const page = await context.newPage();
    try {
      const response = await page.goto(url, { timeout: 10_000, waitUntil: "commit" });
      return `ok ${response?.status() ?? 0}`;
    } catch (error) {
      return `fail ${String((error as Error).message).split("\n")[0]}`;
    } finally {
      await page.close();
    }
  };
  assert.match(await open("http://fixture.test/admitted"), /^ok 200/);
  // Another port of the admitted name lands on the pinned port.
  assert.match(await open("http://fixture.test:8080/other-port"), /^ok 200/);
  // Every other destination is refused by the no-route proxy (layer 2), and
  // nothing reaches the fixture or a port-9 listener.
  for (const url of [
    "http://unmapped.test/",
    "http://example.com/",
    "http://127.0.0.1/",
    `http://127.0.0.1:${FIXTURE_PORT}/literal`,
    "http://[::1]/",
    "http://10.0.0.1/",
    "http://169.254.169.254/latest/meta-data/",
    "http://[fd00:ec2::254]/"
  ]) {
    assert.match(await open(url), /^fail .*net::ERR_PROXY_CONNECTION_FAILED/, url);
  }
  assert.deepEqual(accepts, { "0.0.0.0": 0, "127.0.0.1": 0, "::": 0 }, "nothing reached a port-9 listener");
  assert.deepEqual(
    hits.filter((hit) => !hit.endsWith("/favicon.ico")),
    ["fixture.test/admitted", "fixture.test:8080/other-port"]
  );
});

nativeTest("WebRTC sends no UDP and opens no TURN connection, and the probe detects both without the policy", async (t) => {
  const lan = Object.values(networkInterfaces())
    .flat()
    .find((address) => address && address.family === "IPv4" && !address.internal)?.address;
  assert.ok(lan, "a non-loopback IPv4 address is required for the LAN STUN case");
  const udp = createSocket("udp4");
  let packets = 0;
  udp.on("message", () => packets++);
  // Bound on every interface, so both the loopback and the LAN target can
  // be observed.
  await new Promise<void>((resolve) => udp.bind(0, "0.0.0.0", () => resolve()));
  t.after(() => new Promise<void>((resolve) => udp.close(() => resolve())));
  const udpPort = (udp.address() as { port: number }).port;
  let tcpConnections = 0;
  const fixture = createHttpServer((_request, response) => response.end("<p>fixture</p>"));
  fixture.on("connection", () => tcpConnections++);
  await listen(t, fixture, FIXTURE_PORT, "127.0.0.1");
  const servers = [`stun:127.0.0.1:${udpPort}`, `stun:${lan}:${udpPort}`, `turn:127.0.0.1:${FIXTURE_PORT}?transport=tcp`];

  const gather = async (label: string, adjust: (arg: string) => string) => {
    const profile = profileFor(t, `native-rtc-${label}`);
    const context = await launchDirect(t, profile, confinedArgv(profile, FIXTURE).map(adjust));
    const page = await context.newPage();
    await page.goto("http://fixture.test/", { waitUntil: "commit" });
    packets = 0;
    const tcpBefore = tcpConnections;
    const candidates = await page.evaluate(async (urls: string[]) => {
      const types: string[] = [];
      for (const url of urls) {
        const server = url.startsWith("turn:") ? { urls: url, username: "u", credential: "c" } : { urls: url };
        const connection = new RTCPeerConnection({ iceServers: [server] });
        connection.createDataChannel("probe");
        connection.onicecandidate = (event) => {
          if (event.candidate) types.push(event.candidate.type ?? "unknown");
        };
        await connection.setLocalDescription(await connection.createOffer());
        // Wait for gathering to finish (bounded), not a fixed sleep.
        await new Promise<void>((resolve) => {
          const done = () => connection.iceGatheringState === "complete" && resolve();
          connection.onicegatheringstatechange = done;
          done();
          setTimeout(resolve, 8000);
        });
        await new Promise((resolve) => setTimeout(resolve, 300));
        connection.close();
      }
      return types;
    }, servers);
    await new Promise((resolve) => setTimeout(resolve, 300));
    const result = { candidates, packets, tcp: tcpConnections - tcpBefore };
    await context.close();
    return result;
  };

  const confined = await gather("confined", (arg) => arg);
  assert.deepEqual(confined, { candidates: [], packets: 0, tcp: 0 }, "WebRTC must gather nothing and send nothing");
  // Positive control: the same probe with the default WebRTC policy (outside
  // the contract) does gather candidates and reach the STUN listener, so the
  // empty result above is not an artifact of headless Edge or the timing.
  const control = await gather("control", (arg) => (arg === WEBRTC_POLICY ? "--webrtc-ip-handling-policy=default" : arg));
  assert.ok(control.candidates.length > 0 && control.packets > 0, `control gathered nothing: ${JSON.stringify(control)}`);
});

nativeTest("a same-user process cannot activate the worker's inspector when it runs with --disable-sigusr1", async (t) => {
  const listening = (port: number) =>
    new Promise<boolean>((resolve) => {
      const socket = connect({ host: "127.0.0.1", port }, () => {
        socket.destroy();
        resolve(true);
      });
      socket.on("error", () => resolve(false));
    });
  const debugProcess = (process as unknown as { _debugProcess?: (pid: number) => void })._debugProcess;
  assert.equal(typeof debugProcess, "function", "process._debugProcess is required for this probe");
  const attempt = async (flags: string[]) => {
    const worker = new WorkerProcess(t, flags);
    // The worker is fully started once it answers hello.
    worker.send({ frame: "hello", generation: 1 });
    await worker.reply(1);
    const before = await listening(9229);
    let threw = false;
    try {
      debugProcess?.(worker.child.pid ?? 0);
    } catch {
      threw = true;
    }
    await new Promise((resolve) => setTimeout(resolve, 1500));
    const after = await listening(9229);
    await worker.stop();
    return { before, threw, after };
  };
  const guarded = await attempt(["--disable-sigusr1"]);
  assert.deepEqual(guarded, { before: false, threw: true, after: false }, "activation must be refused with no listener");
  // Negative control: the worker refuses to start without the flag, so the
  // control activates a plain Node process instead.
  const plain = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], { env: cleanEnv(), stdio: "ignore" });
  t.after(() => plain.kill());
  await new Promise((resolve) => setTimeout(resolve, 750));
  const controlBefore = await listening(9229);
  debugProcess?.(plain.pid ?? 0);
  await new Promise((resolve) => setTimeout(resolve, 1500));
  assert.deepEqual({ before: controlBefore, after: await listening(9229) }, { before: false, after: true }, "the probe detects an activated inspector");
});
