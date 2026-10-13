#!/usr/bin/env node
// Runs the SG-000108 native launcher tests (crates/qdral-browser-host
// worker_launch_tests.rs) against the built worker and the installed Edge.
// They are ignored in a plain `cargo test` because they need this worker
// build; on non-Windows hosts they are skipped, since only Windows results
// count as evidence. Run after `npm --prefix apps/qdral-browser-worker test`
// (which builds dist/).

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Every native test must run: a filter that matches nothing (for example
// after a module rename) exits 0 and would otherwise pass vacuously.
const EXPECTED_TESTS = 20;

const app = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repo = resolve(app, "..", "..");
const main = join(app, "dist", "main.js");

if (process.platform !== "win32") {
  process.stdout.write("browser-launcher:native skipped: native launcher tests are Windows-only evidence\n");
  process.exit(0);
}
if (!existsSync(main)) {
  process.stderr.write(`browser-launcher:native: ${main} is missing; build the worker first\n`);
  process.exit(1);
}
const result = spawnSync(
  "cargo",
  ["test", "-p", "qdral-browser-host", "--lib", "native_tests", "--", "--ignored", "--test-threads=1", "--nocapture"],
  {
    cwd: repo,
    stdio: ["ignore", "pipe", "inherit"],
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    shell: false,
    env: { ...process.env, QDRAL_SG108_NODE: process.execPath, QDRAL_SG108_WORKER_MAIN: main }
  }
);
if (result.error) {
  process.stderr.write(`browser-launcher:native: could not run cargo: ${result.error.message}\n`);
  process.exit(1);
}
process.stdout.write(result.stdout ?? "");
if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
const summary = /test result: ok\. (\d+) passed; 0 failed; 0 ignored/.exec(result.stdout ?? "");
if (!summary || Number(summary[1]) !== EXPECTED_TESTS) {
  process.stderr.write(
    `browser-launcher:native: expected ${EXPECTED_TESTS} native tests to pass, saw ${summary ? summary[1] : "none"}\n`
  );
  process.exit(1);
}
