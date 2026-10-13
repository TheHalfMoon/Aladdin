#!/usr/bin/env node
// SG-000108 A8: run the TesterArmy E2E suite against the controlled HTTPS
// fixture, then check the outcome independently of TesterArmy:
// - the suite passed (it verifies the report's exact bytes and SHA-256);
// - the fixture saw only fixture.test, the latest report and no older one;
// - telemetry is off and no model is configured.
// Test-only: never packaged. Windows with an installed Edge.

import { spawn, spawnSync } from "node:child_process";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const fixtureScript = join(here, "..", "scripts", "https-fixture.mjs");
const results = join(here, ".e2e");
rmSync(results, { recursive: true, force: true });
// edge.ts records each Edge it launches here.
const pidsFile = join(tmpdir(), `qdral-a8-pids-${process.pid}.txt`);
rmSync(pidsFile, { force: true });

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

const cli = join(here, "node_modules", "e2e", "dist", "cli", "bin.js");
// Asynchronous, so the fixture's log keeps draining while the suite runs.
const status = await new Promise((resolve) => {
  const run = spawn(process.execPath, [cli, "run"], {
    cwd: here,
    stdio: "inherit",
    env: { ...process.env, FIXTURE_SPKI: spki, A8_PIDS: pidsFile, E2E_TELEMETRY_DISABLED: "1" },
  });
  run.on("exit", (code) => resolve(code));
});
fixture.stdin.end();
await new Promise((resolve) => fixture.on("close", resolve));

const failures = [];
if (status !== 0) failures.push(`e2e run exited ${status}`);

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
const running = (pid) =>
  (spawnSync("tasklist", ["/FI", `PID eq ${pid}`, "/NH"], { encoding: "utf8" }).stdout ?? "").includes(` ${pid} `);
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
  if (running(pid)) spawnSync("taskkill", ["/PID", pid, "/T", "/F"], { stdio: "ignore" });
  for (let attempt = 0; attempt < 40; attempt++) {
    try {
      rmSync(profile, { recursive: true, force: true });
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  }
  if (running(pid)) failures.push(`Edge ${pid} still runs`);
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
if (hits.some((hit) => hit.includes("2026-06-30.pdf") || hit.includes("2026-03-31.pdf")))
  failures.push("an older report was requested");

if (failures.length > 0) {
  process.stderr.write(`A8: FAILED\n- ${failures.join("\n- ")}\n`);
  process.exit(1);
}
process.stdout.write("A8: PASSED\n");
