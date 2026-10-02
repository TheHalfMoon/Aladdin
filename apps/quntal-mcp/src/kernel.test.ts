import assert from "node:assert/strict";
import test from "node:test";
import { buildDaemonEnv, buildRequest } from "./kernel.js";

test("daemon environment drops generic and Quntal secret variables", () => {
  const env = buildDaemonEnv({
    PATH: "safe-path",
    QUNTAL_WORKSPACE_ROOT: "C:\\work",
    QUNTAL_WORKSPACES_JSON: "[]",
    OPENAI_API_KEY: "must-not-leak",
    QUNTAL_TUNNEL_TOKEN: "must-not-leak",
    QUNTAL_API_KEY: "must-not-leak",
    RANDOM_SECRET: "must-not-leak"
  });

  assert.equal(env.PATH, "safe-path");
  assert.equal(env.QUNTAL_WORKSPACE_ROOT, "C:\\work");
  assert.equal(env.QUNTAL_WORKSPACES_JSON, "[]");
  assert.equal(env.OPENAI_API_KEY, undefined);
  assert.equal(env.QUNTAL_TUNNEL_TOKEN, undefined);
  assert.equal(env.QUNTAL_API_KEY, undefined);
  assert.equal(env.RANDOM_SECRET, undefined);
});

test("request builder preserves the declared bounded capability", () => {
  const request = buildRequest({
    sessionId: "session",
    workspaceId: "default",
    capability: "fs.read",
    operation: "read",
    target: "README.md",
    arguments: {}
  });

  assert.equal(request.version, 1);
  assert.equal(request.capability, "fs.read");
  assert.equal(request.operation, "read");
  assert.equal(request.target, "README.md");
  assert.equal(request.workspace_id, "default");
});
