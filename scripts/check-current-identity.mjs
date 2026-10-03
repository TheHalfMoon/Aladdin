#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const git = (...args) => execFileSync("git", args, { encoding: "utf8" }).trim();
const tracked = git("ls-files", "-z").split("\0").filter(Boolean);
const superseded = /Quntal|QUNTAL|quntal/;

const historicalContentAllowed = (path) =>
  path === "docs/identity/QUNTAL_RENAME.md" ||
  path === "docs/identity/QDRAL_RENAME.md" ||
  path === "docs/canonical/CURRENT.md" ||
  path === ".specgrain/canonical-evidence.json" ||
  path === ".specgrain/ledger.json" ||
  path === "scripts/check-current-identity.mjs" ||
  (path.startsWith(".specgrain/specs/") && path !== ".specgrain/specs/SG-000067.json");

const isBinary = (bytes) => bytes.subarray(0, Math.min(bytes.length, 8192)).includes(0);

const pathResidue = tracked.filter(
  (path) => path !== "docs/identity/QUNTAL_RENAME.md" && superseded.test(path)
);
if (pathResidue.length) {
  throw new Error(`superseded identity remains in current paths:\n${pathResidue.join("\n")}`);
}

const contentResidue = [];
for (const path of tracked) {
  if (historicalContentAllowed(path)) continue;
  const bytes = readFileSync(path);
  if (isBinary(bytes)) continue;
  if (superseded.test(bytes.toString("utf8"))) contentResidue.push(path);
}
if (contentResidue.length) {
  throw new Error(`superseded identity remains outside the historical allowlist:\n${contentResidue.join("\n")}`);
}

for (const required of [
  "apps/qdral-mcp/package.json",
  "apps/qdral-relay/package.json",
  "crates/qdrald/Cargo.toml",
  "crates/qdral-lifecycle/Cargo.toml",
  "docs/identity/QDRAL_RENAME.md",
  ".specgrain/specs/SG-000067.json"
]) {
  if (!existsSync(required)) throw new Error(`missing current Qdral identity path: ${required}`);
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
if (rootPackage.name !== "qdral") throw new Error(`root npm package is ${rootPackage.name}, expected qdral`);
const workspaces = [...(rootPackage.workspaces ?? [])].sort();
const expectedWorkspaces = ["apps/qdral-mcp", "apps/qdral-relay"].sort();
if (JSON.stringify(workspaces) !== JSON.stringify(expectedWorkspaces)) {
  throw new Error(`unexpected npm workspaces: ${JSON.stringify(workspaces)}`);
}
for (const [path, expected] of [
  ["apps/qdral-mcp/package.json", "@qdral/mcp"],
  ["apps/qdral-relay/package.json", "@qdral/relay"]
]) {
  const manifest = JSON.parse(readFileSync(path, "utf8"));
  if (manifest.name !== expected) throw new Error(`${path} name is ${manifest.name}, expected ${expected}`);
}

const cargoRoot = readFileSync("Cargo.toml", "utf8");
if (!cargoRoot.includes('repository = "https://github.com/TheHalfMoon/Qdral"')) {
  throw new Error("Cargo workspace repository is not TheHalfMoon/Qdral");
}
if (superseded.test(readFileSync(".specgrain/specs/SG-000067.json", "utf8"))) {
  throw new Error("active SG-000067 still contains the superseded Quntal identity");
}

const identity = readFileSync("docs/identity/QDRAL_RENAME.md", "utf8");
for (const expected of [
  "Status: ACTIVE PRODUCT IDENTITY",
  "Product: `Qdral`",
  "MCP package: `@qdral/mcp`",
  "Relay package: `@qdral/relay`",
  "Local daemon: `qdrald`",
  "Environment-variable prefix: `QDRAL_`",
  "OAuth scope prefix: `qdral.`",
  "Repository identity: `TheHalfMoon/Qdral`"
]) {
  if (!identity.includes(expected)) throw new Error(`Qdral identity record is missing: ${expected}`);
}

console.log(`Qdral identity validated across ${tracked.length} tracked files.`);
