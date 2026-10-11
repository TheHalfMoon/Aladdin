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
  ["test", "-p", "qdral-browser-host", "--lib", "native_tests", "--", "--ignored", "--test-threads=1"],
  {
    cwd: repo,
    stdio: "inherit",
    shell: false,
    env: { ...process.env, QDRAL_SG108_NODE: process.execPath, QDRAL_SG108_WORKER_MAIN: main }
  }
);
process.exit(result.status ?? 1);
