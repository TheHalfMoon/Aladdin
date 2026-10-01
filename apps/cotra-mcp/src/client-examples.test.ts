import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { CANONICAL_TOOL_NAMES } from "./server.js";

const here = dirname(fileURLToPath(import.meta.url));
const examplesDir = join(here, "..", "..", "..", "examples", "mcp-clients");

function example(name: string): string {
  return readFileSync(join(examplesDir, name), "utf8");
}

function jsonExample(name: string): unknown {
  return JSON.parse(example(name)) as unknown;
}

test("client examples use only supported installed entrypoints", () => {
  const claude = jsonExample("claude-desktop.json") as {
    mcpServers: Record<string, { command: string; args: string[] }>;
  };
  assert.deepEqual(claude.mcpServers.cotra, { command: "cotra", args: ["mcp", "stdio"] });

  const genericStdio = jsonExample("generic-stdio.json") as {
    command: string;
    args: string[];
  };
  assert.deepEqual(genericStdio, { command: "cotra", args: ["mcp", "stdio"] });

  for (const name of ["codex-config.toml", "vibe-code-config.toml"]) {
    const text = example(name);
    assert.ok(text.includes('"cotra"'), `${name} must reference the cotra command`);
    assert.ok(text.includes('"mcp"'), `${name} must use the mcp entrypoint`);
    assert.ok(!text.includes("tunnel_"), `${name} must not contain tunnel material`);
  }

  const manifest = jsonExample("claude-desktop-mcpb-manifest.json") as {
    server: { type: string; mcp_config: { command: string; args: string[] } };
  };
  assert.equal(manifest.server.type, "binary");
  assert.equal(manifest.server.mcp_config.command, "cotra");
  assert.deepEqual(manifest.server.mcp_config.args, ["mcp", "stdio"]);
});

test("client examples carry no secrets and no remote endpoints", () => {
  for (const name of readdirSync(examplesDir).sort()) {
    if (name === "README.md" || name === "inspector.sh") {
      continue;
    }
    const text = example(name);
    assert.ok(!text.includes("tunnel_"), `${name} must not contain tunnel material`);
    assert.ok(!text.includes("BEGIN PRIVATE"), `${name} must not contain key material`);
    assert.ok(!text.includes("0.0.0.0"), `${name} must not reference wildcard binds`);
    if (name !== "claude-desktop-mcpb-manifest.json") {
      assert.ok(!text.includes("https://"), `${name} must not point at remote endpoints`);
    }
  }
  const http = example("vibe-code-config.toml");
  assert.ok(http.includes("127.0.0.1"), "loopback examples must stay on loopback");
  assert.ok(http.includes("PORT"), "loopback examples must force an explicit port choice");
});

test("desktop extension manifest matches the canonical catalog", () => {
  const manifest = jsonExample("claude-desktop-mcpb-manifest.json") as {
    manifest_version: string;
    name: string;
    version: string;
    author: { name: string };
    server: { type: string; entry_point: string };
    tools: Array<{ name: string }>;
    compatibility: { platforms: string[] };
  };
  assert.equal(manifest.manifest_version, "0.3");
  assert.equal(manifest.name, "cotra");
  assert.equal(manifest.server.type, "binary");
  const pkg = JSON.parse(
    readFileSync(join(here, "..", "package.json"), "utf8")
  ) as { version: string };
  assert.equal(manifest.version, pkg.version);
  assert.ok(manifest.author.name.length > 0);
  assert.deepEqual(manifest.compatibility.platforms, ["win32"]);
  assert.deepEqual(
    manifest.tools.map((tool) => tool.name).sort(),
    [...CANONICAL_TOOL_NAMES].sort()
  );
});
