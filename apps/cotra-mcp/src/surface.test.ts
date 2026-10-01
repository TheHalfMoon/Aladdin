import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");

/** The closed MCP tool set. Adding a tool requires a governed grain. */
const CANONICAL_TOOLS = [
  "fs_list",
  "fs_read",
  "fs_search",
  "fs_stat",
  "fs_write",
  "fs_write_preview",
  "git_branch_create",
  "git_commit",
  "git_diff",
  "git_fetch",
  "git_fetch_preview",
  "git_log",
  "git_push",
  "git_push_preview",
  "git_stage",
  "git_status",
  "git_unstage",
  "process_spawn",
  "system_status",
  "workspace_get"
];

function toolSources(): Array<[string, string]> {
  return readdirSync(srcDir)
    .filter((name) => name.endsWith(".ts") && !name.endsWith(".test.ts"))
    .map((name) => [name, readFileSync(join(srcDir, name), "utf8")]);
}

test("the MCP surface registers exactly the canonical closed tool set", () => {
  const registered: string[] = [];
  for (const [, text] of toolSources()) {
    for (const match of text.matchAll(/registerTool\(\s*["']([^"']+)["']/g)) {
      registered.push(match[1] ?? "");
    }
  }
  assert.deepEqual([...registered].sort(), [...CANONICAL_TOOLS].sort());
  assert.equal(new Set(registered).size, registered.length, "duplicate tool registration");
});

test("no MCP tool source reaches lifecycle, installer, update, trust, or approval surfaces", () => {
  const offenders: string[] = [];
  const forbidden = [
    /cotra\.exe/i,
    /cotra-mcp-host/i,
    /\bsupervise\b/,
    /self-check/,
    /emergency[-_]revoke|revoke_emergency/,
    /workspace\.trust\.|trust\.history|approval\.history/,
    /tunnel-runtime-key|tunnel_runtime_key/i
  ];
  for (const [name, text] of toolSources()) {
    text.split("\n").forEach((line, index) => {
      if (forbidden.some((pattern) => pattern.test(line))) {
        offenders.push(`${name}:${String(index + 1)}:${line.trim()}`);
      }
    });
  }
  assert.deepEqual(offenders, []);
});
