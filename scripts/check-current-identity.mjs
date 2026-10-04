#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const git = (...args) => execFileSync("git", args, { encoding: "utf8" }).trim();
const tracked = git("ls-files", "-z").split("\0").filter(Boolean);
const supersededUnqualified = /Quntal|QUNTAL|quntal/;

const historicalContentAllowed = (path) =>
  path === "docs/identity/QUNTAL_RENAME.md" ||
  path === "docs/identity/QDRAL_RENAME.md" ||
  path === "docs/identity/DESKAL_RENAME.md" ||
  path === "docs/canonical/CURRENT.md" ||
  path === "docs/p16/sg000066_exit_evidence.json" ||
  path === ".specgrain/canonical-evidence.json" ||
  path === ".specgrain/ledger.json" ||
  path === "scripts/check-current-identity.mjs" ||
  (path.startsWith(".specgrain/specs/") && path !== ".specgrain/specs/SG-000067.json");

const isBinary = (bytes) => bytes.subarray(0, Math.min(bytes.length, 8192)).includes(0);

const historicalTokensAllowed = new Map([
  [
    "apps/qdral-mcp/src/p16-exit.test.ts",
    [
      '"quntal-p16-exit-evidence/2"',
      "/^apps\\/quntal-mcp\\//",
      "/^crates\\/quntald\\//"
    ]
  ]
]);

const withoutAllowedTokens = (path, text) => {
  let remaining = text;
  for (const token of historicalTokensAllowed.get(path) ?? []) {
    remaining = remaining.split(token).join("");
  }
  return remaining;
};

const pathResidue = tracked.filter(
  (path) => path !== "docs/identity/QUNTAL_RENAME.md" && supersededUnqualified.test(path)
);
if (pathResidue.length) {
  throw new Error(`superseded identity remains in current paths:\n${pathResidue.join("\n")}`);
}

const contentResidue = [];
for (const path of tracked) {
  if (historicalContentAllowed(path)) continue;
  const bytes = readFileSync(path);
  if (isBinary(bytes)) continue;
  if (supersededUnqualified.test(withoutAllowedTokens(path, bytes.toString("utf8")))) {
    contentResidue.push(path);
  }
}
if (contentResidue.length) {
  throw new Error(`superseded identity remains outside the historical allowlist:\n${contentResidue.join("\n")}`);
}

for (const required of [
  "apps/qdral-mcp/package.json",
  "apps/qdral-relay/package.json",
  "crates/qdrald/Cargo.toml",
  "crates/qdral-lifecycle/Cargo.toml",
  "docs/identity/DESKAL_RENAME.md",
  "docs/identity/QDRAL_RENAME.md",
  ".specgrain/specs/SG-000067.json"
]) {
  if (!existsSync(required)) throw new Error(`missing required Deskal or compatibility path: ${required}`);
}

for (const forbidden of [
  "apps/quntal-mcp",
  "apps/quntal-relay",
  "crates/quntald",
  "crates/quntal-lifecycle"
]) {
  if (existsSync(forbidden)) throw new Error(`superseded project-owned path still exists: ${forbidden}`);
}

const rootPackage = JSON.parse(readFileSync("package.json", "utf8"));
if (rootPackage.name !== "deskal") {
  throw new Error(`root npm package is ${rootPackage.name}, expected deskal`);
}

const workspaces = [...(rootPackage.workspaces ?? [])].sort();
const expectedWorkspaces = ["apps/qdral-mcp", "apps/qdral-relay"].sort();
if (JSON.stringify(workspaces) !== JSON.stringify(expectedWorkspaces)) {
  throw new Error(`unexpected compatibility npm workspaces: ${JSON.stringify(workspaces)}`);
}

for (const [path, expected] of [
  ["apps/qdral-mcp/package.json", "@qdral/mcp"],
  ["apps/qdral-relay/package.json", "@qdral/relay"]
]) {
  const manifest = JSON.parse(readFileSync(path, "utf8"));
  if (manifest.name !== expected) {
    throw new Error(`${path} compatibility name is ${manifest.name}, expected ${expected}`);
  }
}

const cargoRoot = readFileSync("Cargo.toml", "utf8");
if (!cargoRoot.includes('repository = "https://github.com/TheHalfMoon/Deskal"')) {
  throw new Error("Cargo workspace repository is not TheHalfMoon/Deskal");
}

if (supersededUnqualified.test(readFileSync(".specgrain/specs/SG-000067.json", "utf8"))) {
  throw new Error("active SG-000067 still contains the superseded Quntal identity");
}

const identity = readFileSync("docs/identity/DESKAL_RENAME.md", "utf8");
for (const expected of [
  "Status: ACTIVE PRODUCT IDENTITY",
  "Product: `Deskal`",
  "Root package: `deskal`",
  "Public plugin name: `deskal`",
  "CLI command: `qdral`",
  "Local daemon: `qdrald`",
  "MCP package: `@qdral/mcp`",
  "Relay package: `@qdral/relay`",
  "Environment-variable prefix: `QDRAL_`",
  "OAuth scope prefix: `qdral.`",
  "Repository identity: `TheHalfMoon/Deskal`"
]) {
  if (!identity.includes(expected)) throw new Error(`Deskal identity record is missing: ${expected}`);
}

const readme = readFileSync("README.md", "utf8");
if (!readme.startsWith("# Deskal\n")) {
  throw new Error("README does not present Deskal as the current product name");
}

const marketplace = JSON.parse(readFileSync(".agents/plugins/marketplace.json", "utf8"));
if (marketplace.name !== "deskal" || marketplace.interface?.displayName !== "Deskal") {
  throw new Error("plugin marketplace does not present Deskal as the current product identity");
}

const codexPlugin = JSON.parse(readFileSync("distribution/codex/qdral/plugin.json", "utf8"));
if (codexPlugin.name !== "deskal" || codexPlugin.extensions?.["com.openai"]?.interface?.displayName !== "Deskal") {
  throw new Error("Codex plugin does not present Deskal as the current product identity");
}

const openaiPlugin = JSON.parse(readFileSync("distribution/openai/plugin.config.json", "utf8"));
if (openaiPlugin.name !== "deskal" || openaiPlugin.displayName !== "Deskal") {
  throw new Error("OpenAI plugin does not present Deskal as the current product identity");
}

console.log(`Deskal identity validated across ${tracked.length} tracked files with qdral compatibility identifiers preserved.`);
