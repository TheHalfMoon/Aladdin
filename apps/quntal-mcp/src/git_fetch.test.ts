import assert from "node:assert/strict";
import test from "node:test";
import { gitFetchPreviewSchema, gitFetchSchema } from "./git_fetch.js";

const HEAD = "0123456789abcdef0123456789abcdef01234567";

function rejects(schema: { safeParse(value: unknown): { success: boolean } }, value: unknown) {
  assert.equal(schema.safeParse(value).success, false);
}

test("SG-000016 MCP schemas accept only bounded fetch preview and fetch inputs", () => {
  const preview = gitFetchPreviewSchema("default").parse({
    policy_id: "github-main",
    branch: "main"
  });
  assert.equal(preview.workspace_id, "default");
  assert.equal(preview.policy_id, "github-main");

  const fetch = gitFetchSchema("default").parse({
    path: "repo",
    policy_id: "github-main",
    branch: "feature/fetch",
    expected_head: HEAD,
    expected_prior: "ABSENT"
  });
  assert.equal(fetch.expected_prior, "ABSENT");

  const prior = gitFetchSchema("default").parse({
    policy_id: "github-main",
    branch: "main",
    expected_head: HEAD,
    expected_prior: HEAD
  });
  assert.equal(prior.expected_head, HEAD);
});

test("SG-000016 MCP schemas reject arbitrary URLs, refspecs, and credentials", () => {
  const preview = gitFetchPreviewSchema("default");
  rejects(preview, { policy_id: "github-main" });
  rejects(preview, { policy_id: "", branch: "main" });
  rejects(preview, { policy_id: "UPPER", branch: "main" });
  rejects(preview, { policy_id: "github-main", branch: "" });
  rejects(preview, { policy_id: "github-main", branch: "refs/heads/main" });
  rejects(preview, { policy_id: "github-main", branch: "main", url: "https://example.com/repo.git" });
  rejects(preview, { policy_id: "github-main", branch: "main", refspec: "refs/heads/main:refs/heads/other" });
  rejects(preview, { policy_id: "github-main", branch: "main", token: "secret" });

  const fetch = gitFetchSchema("default");
  rejects(fetch, { policy_id: "github-main", branch: "main", expected_head: HEAD });
  rejects(fetch, {
    policy_id: "github-main",
    branch: "main",
    expected_head: "HEAD",
    expected_prior: "ABSENT"
  });
  rejects(fetch, {
    policy_id: "github-main",
    branch: "main",
    expected_head: HEAD.toUpperCase(),
    expected_prior: "ABSENT"
  });
  rejects(fetch, {
    policy_id: "github-main",
    branch: "main",
    expected_head: HEAD,
    expected_prior: "raw-secret"
  });
  rejects(fetch, {
    policy_id: "github-main",
    branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    proxy: "http://proxy.invalid"
  });
  rejects(fetch, {
    policy_id: "github-main",
    branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential: "token"
  });
});

test("SG-000016 MCP fetch keeps push unreachable through typed schemas", () => {
  const fetch = gitFetchSchema("default");
  rejects(fetch, {
    policy_id: "github-main",
    branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    operation: "push"
  });
  rejects(fetch, {
    policy_id: "github-main",
    branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    force: true
  });
});
