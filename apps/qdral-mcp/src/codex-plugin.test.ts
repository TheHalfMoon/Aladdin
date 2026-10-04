import assert from "node:assert/strict";
import test from "node:test";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { discoverTools } from "./discovery.js";
import { findSecrets, MCP_SCHEMA_URL, PLUGIN_SCHEMA_URL } from "./openai_plugin.js";
import { defaultTransportContext } from "./server.js";
import { REMOTE_TOOL_NAMES } from "./tool_contract.js";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");
const pluginDir = join(repo, "distribution", "codex", "qdral");
const read = (path: string): Record<string, unknown> => JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;

/**
 * SG-000070: the Codex plugin package is a local stdio integration. It
 * declares exactly the canonical `qdral mcp stdio` entrypoint, carries no
 * endpoint or secret, and its marketplace entry points at this directory.
 */
test("the Codex plugin declares exactly the canonical local stdio entrypoint", () => {
  const plugin = read(join(pluginDir, "plugin.json"));
  assert.equal(plugin.$schema, PLUGIN_SCHEMA_URL);
  assert.equal(plugin.name, "qdral");
  const version = (read(join(repo, "apps", "qdral-mcp", "package.json")) as { version: string }).version;
  assert.equal(plugin.version, version, "plugin version follows the release version");
  assert.deepEqual(Object.keys(plugin.extensions as object), ["com.openai"]);
  const iface = ((plugin.extensions as Record<string, Record<string, Record<string, unknown>>>)["com.openai"]!.interface)!;
  for (const field of ["composerIcon", "logo"]) {
    const asset = String(iface[field]);
    assert.ok(asset.startsWith("./assets/") && !asset.includes(".."), field);
    assert.ok(existsSync(join(pluginDir, asset)), asset);
  }
  const mcp = read(join(pluginDir, "mcp.json"));
  assert.equal(mcp.$schema, MCP_SCHEMA_URL);
  assert.deepEqual(mcp.mcpServers, { qdral: { type: "stdio", command: "qdral", args: ["mcp", "stdio"] } });
  for (const name of readdirSync(pluginDir, { recursive: true }).map(String)) {
    const path = join(pluginDir, name);
    if (name.endsWith(".json") || name.endsWith(".svg")) {
      const text = readFileSync(path, "utf8");
      assert.deepEqual(findSecrets(text), [], name);
    }
  }
});

test("the repository marketplace lists exactly the Qdral plugin from this repository", () => {
  const marketplace = read(join(repo, ".agents", "plugins", "marketplace.json")) as {
    name: string;
    interface: { displayName: string };
    plugins: Array<{ name: string; source: { source: string; path: string }; policy: Record<string, string>; category: string }>;
  };
  assert.equal(marketplace.name, "qdral");
  assert.equal(marketplace.interface.displayName, "Qdral");
  assert.equal(marketplace.plugins.length, 1);
  const entry = marketplace.plugins[0]!;
  assert.equal(entry.name, "qdral");
  assert.equal(entry.source.source, "local");
  assert.equal(entry.source.path, "./distribution/codex/qdral");
  assert.ok(existsSync(join(repo, entry.source.path, "plugin.json")));
  assert.equal(typeof entry.policy.installation, "string");
  assert.equal(typeof entry.policy.authentication, "string");
  assert.equal(entry.category, "Productivity");
});

test("shared tools carry identical metadata on local and remote transports", async () => {
  const local = await discoverTools({ ...defaultTransportContext(), transportKind: "stdio" });
  const loopback = await discoverTools({ ...defaultTransportContext(), transportKind: "loopback_http" });
  const remote = await discoverTools({ transportKind: "relay", providerKind: "codex", toolSurfaceProfile: "core" });
  assert.deepEqual(loopback, local, "stdio and loopback serve the same local profile");
  assert.deepEqual(remote.map((tool) => tool.name).sort(), [...REMOTE_TOOL_NAMES].sort());
  for (const tool of remote) {
    const same = local.find((candidate) => candidate.name === tool.name);
    assert.deepEqual(same, tool, `${tool.name} metadata is identical on every transport`);
  }
});
