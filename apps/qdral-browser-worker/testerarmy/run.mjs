#!/usr/bin/env node
// SG-000108 A8: run the TesterArmy E2E suite against the controlled HTTPS
// fixture, then check the outcome independently of TesterArmy:
// - the suite passed (it verifies the report's exact bytes and SHA-256);
// - the fixture saw only fixture.test, the latest report and no older one;
// - the suite's Edge ran under the production network confinement (the
//   worker's own flag values), and an IP-literal bypass toward this host's
//   LAN address was denied: listeners there received nothing (after a
//   positive control proved them reachable, and a negative control proved an
//   unconfined Edge does reach them), and the OS's connection tables showed
//   no connection, listener or mDNS endpoint of that Edge beyond loopback;
// - telemetry is off and no model is configured.
// Test-only: never packaged. Windows with an installed Edge.

import { spawn, spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { createSocket } from "node:dgram";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createConnection, createServer } from "node:net";
import { networkInterfaces, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const fixtureScript = join(here, "..", "scripts", "https-fixture.mjs");
const results = join(here, ".e2e");
rmSync(results, { recursive: true, force: true });
// edge.ts records each Edge it launches here.
const pidsFile = join(tmpdir(), `qdral-a8-pids-${process.pid}.txt`);
rmSync(pidsFile, { force: true });

const failures = [];
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
// Whether `pid` is a running Edge process (never another program that
// reused the pid).
const isEdge = (pid) =>
  (
    spawnSync("tasklist", ["/FI", `PID eq ${pid}`, "/FI", "IMAGENAME eq msedge.exe", "/NH"], { encoding: "utf8" }).stdout ??
    ""
  ).includes(` ${pid} `);

// The production launch values, from the built worker itself.
const protocolJs = join(here, "..", "dist", "protocol.js");
if (!existsSync(protocolJs)) {
  process.stderr.write("A8: build the worker first (npm run build in apps/qdral-browser-worker)\n");
  process.exit(1);
}
const protocol = await import(pathToFileURL(protocolJs).href);
if (typeof protocol.launchRefusal !== "function" || !Array.isArray(protocol.FROZEN_FLAGS)) {
  process.stderr.write("A8: the worker's launch contract is missing\n");
  process.exit(1);
}
const edgeExe = [
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].find((path) => existsSync(path));
if (!edgeExe) {
  process.stderr.write("A8: Edge is not installed\n");
  process.exit(1);
}
// The engine environment the host hands over: the allowlisted keys only.
const engineEnv = Object.fromEntries(
  protocol.ENGINE_ENV_ALLOWLIST.filter((key) => process.env[key] !== undefined).map((key) => [key, process.env[key]]),
);

// This host's own LAN address, where the bypass is aimed and listened for.
const lan = Object.values(networkInterfaces())
  .flat()
  .find((entry) => entry && entry.family === "IPv4" && !entry.internal)?.address;
if (!lan) {
  process.stderr.write("A8: no non-loopback IPv4 interface; the bypass check needs one\n");
  process.exit(1);
}
const arrivals = { datagrams: [], connections: [] };
const udp = createSocket("udp4");
udp.on("message", (_message, from) => arrivals.datagrams.push(`${from.address}:${from.port}`));
await new Promise((resolve) => udp.bind(0, lan, resolve));
const tcp = createServer((socket) => {
  arrivals.connections.push(`${socket.remoteAddress}:${socket.remotePort}`);
  socket.destroy();
});
await new Promise((resolve) => tcp.listen(0, lan, resolve));
const udpPort = udp.address().port;
const tcpPort = tcp.address().port;
const settle = async (check) => {
  for (let i = 0; i < 100 && !check(); i++) await sleep(100);
  return check();
};

// Positive control: the listeners are reachable at that address.
const probe = createSocket("udp4");
await new Promise((resolve) => probe.bind(0, lan, resolve));
probe.send("control", udpPort, lan);
createConnection({ host: lan, port: tcpPort }).on("error", () => {}).end();
if (!(await settle(() => arrivals.datagrams.length > 0 && arrivals.connections.length > 0)))
  failures.push(`the LAN listeners are not reachable at ${lan}`);
probe.close();
arrivals.datagrams.length = 0;
arrivals.connections.length = 0;

// Negative control: an Edge without the confinement does send a navigation
// to a LAN IP literal, so the bypass probe is a real one. Its proxy is a
// separate listener at the same address that records each request line,
// and it must see this exact navigation (a nonce path, on a port nothing
// else uses), not merely some connection: Edge's own background requests
// also reach a proxy. It reaches nothing else: every request it makes goes
// to that proxy, and the services that ignore the proxy (#324) are off.
{
  const requests = [];
  const proxy = createServer((socket) => {
    let head = "";
    socket.on("data", (chunk) => {
      head += chunk;
      const end = head.indexOf("\r\n");
      if (end >= 0) {
        requests.push(head.slice(0, end));
        socket.destroy();
      }
    });
    socket.on("error", () => {});
  });
  await new Promise((resolve) => proxy.listen(0, lan, resolve));
  const proxyPort = proxy.address().port;
  const nonce = randomBytes(8).toString("hex");
  const target = `http://${lan}:${tcpPort}/control-${nonce}`;
  const expected = `GET ${target} HTTP/1.1`;
  const profile = mkdtempSync(join(tmpdir(), "qdral-a8-control-"));
  const raw = spawn(
    edgeExe,
    [
      ...protocol.FROZEN_FLAGS,
      `--user-data-dir=${profile}`,
      `--proxy-server=http://${lan}:${proxyPort}`,
      "--proxy-bypass-list=<-loopback>",
      protocol.WEBRTC_POLICY,
      protocol.DISABLE_QUIC,
      protocol.DISABLE_FEATURES,
      target,
    ],
    { stdio: "ignore", env: engineEnv },
  );
  for (let i = 0; i < 300 && !requests.includes(expected); i++) await sleep(100);
  const reached = requests.includes(expected);
  if (isEdge(raw.pid)) spawnSync("taskkill", ["/PID", String(raw.pid), "/T", "/F"], { stdio: "ignore" });
  for (let attempt = 0; attempt < 40 && existsSync(profile); attempt++) {
    try {
      rmSync(profile, { recursive: true, force: true });
    } catch {
      await sleep(250);
    }
  }
  proxy.close();
  process.stdout.write(
    `A8-EVIDENCE negative control: an Edge without the confinement sent "${expected}": ${reached} (${requests.length} requests at its proxy)\n`,
  );
  if (!reached) failures.push("the negative control's navigation never reached its proxy: the bypass probe proves nothing");
  if (isEdge(raw.pid)) failures.push(`the negative control's Edge ${raw.pid} still runs`);
  if (existsSync(profile)) failures.push(`the negative control left its profile: ${profile}`);
  // Nothing of the control may count as the suite's.
  arrivals.datagrams.length = 0;
  arrivals.connections.length = 0;
}

// The OS's view while the suite runs.
const scanOut = join(tmpdir(), `qdral-a8-scan-${process.pid}.jsonl`);
const scanStop = `${scanOut}.stop`;
rmSync(scanOut, { force: true });
rmSync(scanStop, { force: true });
const scanner = spawn(
  "powershell.exe",
  ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", join(here, "scan.ps1"), "-Stop", scanStop, "-Out", scanOut, "-Pids", pidsFile],
  { stdio: ["ignore", "ignore", "inherit"] },
);

const fixture = spawn(process.execPath, [fixtureScript], { stdio: ["pipe", "pipe", "inherit"] });
const hits = [];
const spki = await new Promise((resolve, reject) => {
  let buffered = "";
  fixture.stdout.on("data", (chunk) => {
    buffered += chunk;
    let at;
    while ((at = buffered.indexOf("\n")) >= 0) {
      const line = buffered.slice(0, at).trim();
      buffered = buffered.slice(at + 1);
      if (line.startsWith("READY ")) resolve(line.slice(6));
      else if (line.startsWith("HIT ")) hits.push(line.slice(4));
    }
  });
  fixture.on("exit", () => reject(new Error("the fixture exited")));
});

// The exact argv the host builds for the fixture.test destination
// (confinement.rs), with the worker's own fixed values; the worker's
// contract check must accept it. edge.ts swaps only the pipe for DevTools.
const argvTemplate = [
  ...protocol.FROZEN_FLAGS,
  "--user-data-dir={PROFILE}",
  protocol.PIPE,
  "--host-resolver-rules=MAP fixture.test 127.0.0.1:443, MAP * ~NOTFOUND",
  protocol.NO_ROUTE_PROXY,
  "--proxy-bypass-list=fixture.test;<-loopback>",
  protocol.WEBRTC_POLICY,
  protocol.DISABLE_QUIC,
  protocol.DISABLE_SHARED_WORKER,
  protocol.DISABLE_FEATURES,
  `--ignore-certificate-errors-spki-list=${spki}`,
  protocol.INITIAL_URL,
];
const sampleProfile = "C:\\qdral-a8\\profile";
const refusal = protocol.launchRefusal({
  frame: "launch",
  engine: edgeExe,
  profile_dir: sampleProfile,
  env: engineEnv,
  argv: argvTemplate.map((arg) => (arg === "--user-data-dir={PROFILE}" ? `--user-data-dir=${sampleProfile}` : arg)),
});
if (refusal !== null) failures.push(`the worker's contract refuses the suite's launch: ${refusal}`);
process.stdout.write(`A8-EVIDENCE production launch contract: ${refusal === null ? "accepted" : refusal}\n`);

const cli = join(here, "node_modules", "e2e", "dist", "cli", "bin.js");
// Asynchronous, so the fixture's log keeps draining while the suite runs.
const status = await new Promise((resolve) => {
  const run = spawn(process.execPath, [cli, "run"], {
    cwd: here,
    stdio: "inherit",
    env: {
      ...process.env,
      FIXTURE_SPKI: spki,
      A8_PIDS: pidsFile,
      A8_ARGV: JSON.stringify(argvTemplate),
      A8_ENGINE_ENV: JSON.stringify(engineEnv),
      A8_LAN_IP: lan,
      A8_TCP_PORT: String(tcpPort),
      A8_UDP_PORT: String(udpPort),
      E2E_TELEMETRY_DISABLED: "1",
    },
  });
  run.on("exit", (code) => resolve(code));
});
fixture.stdin.end();
await new Promise((resolve) => fixture.on("close", resolve));
writeFileSync(scanStop, "");
await new Promise((resolve) => scanner.on("exit", resolve));
udp.close();
tcp.close();

if (status !== 0) failures.push(`e2e run exited ${status}`);

// The bypass was denied: nothing arrived at the LAN listeners.
process.stdout.write(
  `A8-EVIDENCE LAN listeners during the suite: ${arrivals.datagrams.length} datagrams, ${arrivals.connections.length} connections\n`,
);
if (arrivals.datagrams.length + arrivals.connections.length > 0)
  failures.push(`the suite's Edge reached ${lan}: ${[...arrivals.datagrams, ...arrivals.connections].join(", ")}`);

// And the OS saw no network endpoint of that Edge beyond loopback.
const loopback = (address) => address === "::1" || address.startsWith("127.") || address.startsWith("::ffff:127.");
const unspecified = (address) => address === "0.0.0.0" || address === "::";
let samples = 0;
let owned = 0;
const flagged = new Set();
const ice = new Set();
for (const line of existsSync(scanOut) ? readFileSync(scanOut, "utf8").split("\n").filter(Boolean) : []) {
  const sample = JSON.parse(line);
  samples += 1;
  owned = Math.max(owned, sample.processes);
  for (const row of [sample.rows ?? []].flat()) {
    const at = `${row.kind} ${row.state} ${row.local}:${row.lport} -> ${row.remote}:${row.rport} (${row.name} ${row.pid})`;
    if (row.kind === "udp") {
      if (row.lport === 5353) flagged.add(at);
      else if (!loopback(row.local)) ice.add(`${row.local}:${row.lport}`);
    } else if (row.state === "Listen") {
      if (!loopback(row.local)) flagged.add(at);
    } else if (row.state !== "Bound" && !loopback(row.remote) && !unspecified(row.remote)) {
      flagged.add(at);
    }
  }
}
rmSync(scanOut, { force: true });
rmSync(scanStop, { force: true });
process.stdout.write(
  `A8-EVIDENCE OS tables: ${samples} samples, up to ${owned} Edge processes; flagged ${JSON.stringify([...flagged])}; ${ice.size} other UDP sockets\n`,
);
if (owned === 0) failures.push("the OS scan never saw the suite's Edge: the scan proves nothing");
// The page's WebRTC code ran: ICE sockets were bound (the UDP leg is only
// as good as the attempt).
if (ice.size === 0) failures.push("no ICE socket was seen: the page's WebRTC probe never ran");
if (flagged.size > 0) failures.push(`the suite's Edge held a network endpoint beyond loopback: ${[...flagged].join("; ")}`);

// Nothing left behind: every Edge the suite launched is ended with its
// whole process tree, and its profile removed. A profile can be deleted
// only once no Edge process holds it, so every profile gone means every
// browser released it; each recorded main process must also be gone.
let launched = [];
try {
  launched = readFileSync(pidsFile, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((line) => line.split("\t"));
} catch {}
rmSync(pidsFile, { force: true });
if (launched.length === 0) failures.push("no launched Edge was recorded");
// Ask the browser itself to close over its own DevTools endpoint (this ends
// every process of that browser), then end any recorded process still left.
const closeOverCdp = (endpoint) =>
  new Promise((resolve) => {
    let socket;
    try {
      socket = new WebSocket(endpoint);
    } catch {
      resolve();
      return;
    }
    const done = () => resolve();
    socket.onopen = () => socket.send(JSON.stringify({ id: 1, method: "Browser.close" }));
    socket.onclose = done;
    socket.onerror = done;
    setTimeout(done, 5000);
  });
for (const [pid, profile, endpoint] of launched) {
  if (endpoint) await closeOverCdp(endpoint);
  if (isEdge(pid)) spawnSync("taskkill", ["/PID", pid, "/T", "/F"], { stdio: "ignore" });
  for (let attempt = 0; attempt < 40; attempt++) {
    try {
      rmSync(profile, { recursive: true, force: true });
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  }
  if (isEdge(pid)) failures.push(`Edge ${pid} still runs`);
  if (existsSync(profile)) failures.push(`profile left behind: ${profile}`);
}
if (failures.length === 0) {
  process.stdout.write(`A8-EVIDENCE cleanup: ${launched.length} launched Edge gone, no profile left\n`);
}

// Fixture origin only.
const hosts = [...new Set(hits.map((hit) => hit.split(" ")[0]))];
process.stdout.write(`A8-EVIDENCE fixture hits: ${hits.join(" | ")}\n`);
if (hosts.some((host) => host !== "fixture.test")) failures.push(`another origin was contacted: ${hosts.join(", ")}`);
if (!hits.includes("fixture.test /reports/2026-09-30.pdf")) failures.push("the latest report was never requested");
if (!hits.some((hit) => hit.startsWith("fixture.test /rtc?"))) failures.push("the bypass page was never served");
if (hits.some((hit) => hit.includes("2026-06-30.pdf") || hit.includes("2026-03-31.pdf")))
  failures.push("an older report was requested");

if (failures.length > 0) {
  process.stderr.write(`A8: FAILED\n- ${failures.join("\n- ")}\n`);
  process.exit(1);
}
process.stdout.write("A8: PASSED\n");
