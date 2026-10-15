import assert from "node:assert/strict";
import test from "node:test";
import { processSpawnInputSchema } from "./process.js";

const valid = {
  workspace_id: "default",
  executable: "C:\\Windows\\System32\\whoami.exe",
  argv: [],
  cwd: ".",
  timeout_ms: 30_000,
  stdout_bytes: 2 * 1024 * 1024,
  stderr_bytes: 256 * 1024,
  stdin_policy: "null" as const,
  network_class: "NONE" as const
};

test("process.spawn schema accepts only the bounded argv contract", () => {
  assert.deepEqual(processSpawnInputSchema.parse(valid), valid);
});

test("process.spawn schema exposes no raw command, env, stdin payload, or network widening", () => {
  for (const extra of [
    { command: "cmd /c whoami" },
    { env: { PATH: "caller" } },
    { stdin: "payload" },
    { detached: true }
  ]) {
    assert.equal(processSpawnInputSchema.safeParse({ ...valid, ...extra }).success, false);
  }
  assert.equal(
    processSpawnInputSchema.safeParse({ ...valid, network_class: "DIRECT_DESTINATION" }).success,
    false
  );
  assert.equal(
    processSpawnInputSchema.safeParse({ ...valid, stdin_policy: "text" }).success,
    false
  );
});

test("process.spawn schema enforces explicit bounds", () => {
  assert.equal(processSpawnInputSchema.safeParse({ ...valid, timeout_ms: 999 }).success, false);
  assert.equal(
    processSpawnInputSchema.safeParse({ ...valid, stdout_bytes: 16 * 1024 * 1024 + 1 }).success,
    false
  );
  assert.equal(
    processSpawnInputSchema.safeParse({ ...valid, stderr_bytes: 4 * 1024 * 1024 + 1 }).success,
    false
  );
});
