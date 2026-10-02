import assert from "node:assert/strict";
import test from "node:test";
import {
  gitBranchCreateSchema,
  gitCommitSchema,
  gitStageSchema,
  gitUnstageSchema
} from "./git_mutation.js";

const HEAD = "0123456789abcdef0123456789abcdef01234567";

function rejects(schema: { safeParse(value: unknown): { success: boolean } }, value: unknown) {
  assert.equal(schema.safeParse(value).success, false);
}

test("SG-000015 MCP schemas accept only typed local Git mutation inputs", () => {
  assert.equal(
    gitBranchCreateSchema("default").parse({
      expected_head: HEAD,
      branch: "quntal/approved"
    }).workspace_id,
    "default"
  );

  assert.deepEqual(
    gitStageSchema("default").parse({
      expected_head: HEAD,
      paths: ["src/a.ts", "README.md"]
    }).paths,
    ["src/a.ts", "README.md"]
  );

  assert.deepEqual(
    gitUnstageSchema("default").parse({
      path: "repo",
      expected_head: HEAD,
      paths: ["src/a.ts"]
    }).paths,
    ["src/a.ts"]
  );

  assert.equal(
    gitCommitSchema("default").parse({
      expected_head: HEAD,
      message: "feat: bounded local mutation"
    }).message,
    "feat: bounded local mutation"
  );
});

test("SG-000015 MCP schemas reject missing or malformed expected HEAD", () => {
  const stage = gitStageSchema("default");
  rejects(stage, { paths: ["a.txt"] });
  rejects(stage, { expected_head: "HEAD", paths: ["a.txt"] });
  rejects(stage, {
    expected_head: HEAD.toUpperCase(),
    paths: ["a.txt"]
  });
});

test("SG-000015 MCP schemas bound branch, path set, and commit message", () => {
  rejects(gitBranchCreateSchema("default"), {
    expected_head: HEAD,
    branch: ""
  });
  rejects(gitStageSchema("default"), {
    expected_head: HEAD,
    paths: []
  });
  rejects(gitStageSchema("default"), {
    expected_head: HEAD,
    paths: Array.from({ length: 129 }, (_, index) => `f${index}.txt`)
  });
  rejects(gitCommitSchema("default"), {
    expected_head: HEAD,
    message: ""
  });
  rejects(gitCommitSchema("default"), {
    expected_head: HEAD,
    message: "x".repeat(8 * 1024 + 1)
  });
});
