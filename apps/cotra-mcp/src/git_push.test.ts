import assert from "node:assert/strict";
import test from "node:test";
import { gitPushPreviewSchema, gitPushSchema } from "./git_push.js";

const HEAD = "0123456789abcdef0123456789abcdef01234567";

function rejects(schema: { safeParse(value: unknown): { success: boolean } }, value: unknown) {
  assert.equal(schema.safeParse(value).success, false);
}

test("SG-000017 MCP schemas accept only bounded push preview and push inputs", () => {
  const preview = gitPushPreviewSchema("default").parse({
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    credential_reference: "github-push-token"
  });
  assert.equal(preview.workspace_id, "default");
  assert.equal(preview.policy_id, "github-push");
  assert.equal(preview.credential_reference, "github-push-token");

  const push = gitPushSchema("default").parse({
    path: "repo",
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "feature/push",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous"
  });
  assert.equal(push.expected_prior, "ABSENT");

  const prior = gitPushSchema("default").parse({
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: HEAD,
    credential_reference: "github-push-token"
  });
  assert.equal(prior.expected_head, HEAD);
});

test("SG-000017 MCP schemas reject arbitrary URLs, refspecs, force, and raw credentials", () => {
  const preview = gitPushPreviewSchema("default");
  rejects(preview, { policy_id: "github-push", source_branch: "main", dest_branch: "main" });
  rejects(preview, { policy_id: "", source_branch: "main", dest_branch: "main", credential_reference: "anonymous" });
  rejects(preview, { policy_id: "UPPER", source_branch: "main", dest_branch: "main", credential_reference: "anonymous" });
  rejects(preview, { policy_id: "github-push", source_branch: "", dest_branch: "main", credential_reference: "anonymous" });
  rejects(preview, { policy_id: "github-push", source_branch: "refs/heads/main", dest_branch: "main", credential_reference: "anonymous" });
  rejects(preview, { policy_id: "github-push", source_branch: "main:other", dest_branch: "main", credential_reference: "anonymous" });
  rejects(preview, { policy_id: "github-push", source_branch: "+main", dest_branch: "main", credential_reference: "anonymous" });
  rejects(preview, { policy_id: "github-push", source_branch: "st*", dest_branch: "main", credential_reference: "anonymous" });
  rejects(preview, { policy_id: "github-push", source_branch: "main", dest_branch: "main", credential_reference: "anonymous", url: "https://example.com/repo.git" });
  rejects(preview, { policy_id: "github-push", source_branch: "main", dest_branch: "main", credential_reference: "anonymous", refspec: "refs/heads/main:refs/heads/other" });
  rejects(preview, { policy_id: "github-push", source_branch: "main", dest_branch: "main", credential_reference: "anonymous", token: "secret" });
  rejects(preview, { policy_id: "github-push", source_branch: "main", dest_branch: "main", credential_reference: "ghp_rawtoken123" });
  rejects(preview, { policy_id: "github-push", source_branch: "main", dest_branch: "main", credential_reference: "https://example.com/token" });

  const push = gitPushSchema("default");
  rejects(push, { policy_id: "github-push", source_branch: "main", dest_branch: "main", expected_head: HEAD, credential_reference: "anonymous" });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: "HEAD",
    expected_prior: "ABSENT",
    credential_reference: "anonymous"
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD.toUpperCase(),
    expected_prior: "ABSENT",
    credential_reference: "anonymous"
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "raw-secret",
    credential_reference: "anonymous"
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous",
    proxy: "http://proxy.invalid"
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous",
    credential: "token"
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous",
    password: "secret"
  });
});

test("SG-000017 MCP push keeps force, delete, and widening operations unreachable", () => {
  const push = gitPushSchema("default");
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous",
    operation: "push"
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous",
    force: true
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous",
    delete: true
  });
  rejects(push, {
    policy_id: "github-push",
    source_branch: "main",
    dest_branch: "main",
    expected_head: HEAD,
    expected_prior: "ABSENT",
    credential_reference: "anonymous",
    tags: true
  });
});
