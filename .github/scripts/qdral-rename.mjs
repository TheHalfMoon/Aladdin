#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const EXPECTED_BASE = "6013ada69ea9c5c0ab9174391f1cc4c36408504e";
const branch = process.env.GITHUB_HEAD_REF || process.env.GITHUB_REF_NAME || "";
if (branch !== "identity/qdral-to-qdral") {
  throw new Error(`refusing Qdral rename on unexpected branch: ${branch}`);
}
const mergeBase = execFileSync("git", ["merge-base", "HEAD", EXPECTED_BASE], { encoding: "utf8" }).trim();
if (mergeBase !== EXPECTED_BASE) {
  throw new Error(`rename branch is not rooted at expected canonical base ${EXPECTED_BASE}: ${mergeBase}`);
}

const git = (...args) => execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
const tracked = () => git("ls-files", "-z").split("\0").filter(Boolean);
const isBinary = (buffer) => buffer.subarray(0, Math.min(buffer.length, 8192)).includes(0);
const renameToken = (value) => value
  .replaceAll("QDRAL", "QDRAL")
  .replaceAll("Qdral", "Qdral")
  .replaceAll("qdral", "qdral");

const preservedHistoricalContent = (path) => {
  if (path === ".specgrain/canonical-evidence.json" || path === ".specgrain/ledger.json") return true;
  if (path.startsWith(".specgrain/specs/") && path !== ".specgrain/specs/SG-000067.json") return true;
  if (path === "docs/canonical/CURRENT.md") return true;
  if (path === "docs/identity/QDRAL_RENAME.md") return true;
  return false;
};

// Rename project-owned paths first. The previous identity record keeps its historical filename.
for (const oldPath of tracked().sort((a, b) => b.length - a.length)) {
  if (oldPath === "docs/identity/QDRAL_RENAME.md") continue;
  const newPath = renameToken(oldPath);
  if (newPath === oldPath) continue;
  if (existsSync(newPath)) throw new Error(`rename collision: ${oldPath} -> ${newPath}`);
  mkdirSync(dirname(newPath), { recursive: true });
  execFileSync("git", ["mv", "--", oldPath, newPath]);
}

// Rewrite current product-owned text. Closed SpecGrain evidence and the canonical history ledger remain verbatim.
for (const path of tracked()) {
  if (preservedHistoricalContent(path)) continue;
  const bytes = readFileSync(path);
  if (isBinary(bytes)) continue;
  const before = bytes.toString("utf8");
  let after = renameToken(before);
  if (path === ".specgrain/specs/SG-000067.json") {
    after = after.replaceAll("cotrad", "qdrald").replaceAll("cotra-mcp", "qdral-mcp");
  }
  if (after !== before) writeFileSync(path, after, "utf8");
}

// Preserve the old identity record as history; mark it as superseded rather than rewriting its claims.
{
  const path = "docs/identity/QDRAL_RENAME.md";
  let text = readFileSync(path, "utf8");
  text = text.replace("Status: ACTIVE PRODUCT IDENTITY", "Status: SUPERSEDED HISTORICAL IDENTITY");
  const marker = "Superseded by the Qdral identity migration";
  if (!text.includes(marker)) {
    text += `\n\n## Supersession\n\n${marker} on 2026-10-03. This file is retained verbatim apart from this status note so the Qdral-era identity and migration evidence remain attributable to the name used at the time. Current naming is defined by \`QDRAL_RENAME.md\`.\n`;
  }
  writeFileSync(path, text, "utf8");
}

// Keep CURRENT as a historical ledger, but make the current identity unambiguous without rewriting old evidence.
{
  const path = "docs/canonical/CURRENT.md";
  let text = readFileSync(path, "utf8");
  const heading = "# Qdral Current Identity Notice";
  if (!text.startsWith(heading)) {
    const notice = `${heading}\n\nStatus: ACTIVE PRODUCT IDENTITY\nEffective date: 2026-10-03\n\nQdral is the current product and project name. Qdral and Cotra references in the historical ledger below identify earlier names of this same project and are intentionally preserved rather than rewritten. Current project-owned packages, crates, executables, CLI names, environment variables, paths, documentation, and repository references are governed by \`docs/identity/QDRAL_RENAME.md\`. This identity change grants no new authority.\n\n---\n\n`;
    text = notice + text;
    writeFileSync(path, text, "utf8");
  }
}

// New canonical identity record.
{
  const path = "docs/identity/QDRAL_RENAME.md";
  const text = `# Qdral Identity Migration\n\nStatus: ACTIVE PRODUCT IDENTITY\nEffective date: 2026-10-03\n\nQdral is the official product and project name formerly known as Qdral, and before that Cotra.\n\n## Canonical naming\n\n- Product: \`Qdral\`\n- Root package: \`qdral\`\n- MCP package: \`@qdral/mcp\`\n- Relay package: \`@qdral/relay\`\n- Rust crates: \`qdral-*\`\n- Local daemon: \`qdrald\`\n- CLI and install root: \`qdral\` / \`%LOCALAPPDATA%\\Qdral\`\n- Environment-variable prefix: \`QDRAL_\`\n- OAuth scope prefix: \`qdral.\`\n- Repository identity: \`TheHalfMoon/Qdral\`\n\n## Historical evidence and compatibility\n\nClosed SpecGrain records, canonical evidence, the canonical history ledger, merged PR/CI evidence, and the prior \`QDRAL_RENAME.md\` record intentionally retain Qdral or Cotra where those names identify the project at the time the evidence was authored. Those historical names must not be rewritten as if old evidence had originally used Qdral.\n\nCurrent project-owned code, packages, crates, executables, CLI surfaces, environment variables, OAuth/tool metadata, install paths, live documentation, and future planning use Qdral. A retained old-name string is permitted only when it is explicitly historical evidence or a deliberately documented compatibility boundary.\n\n## Authority\n\nThis rename changes identity only. It does not grant new filesystem, process, network, browser, UI, secret, approval, trust, executable-admission, remote-session, or privileged authority. Existing fail-closed security boundaries remain authoritative.\n\n## Repository migration\n\nThe intended canonical repository is \`https://github.com/TheHalfMoon/Qdral\`. Repository-name migration is an administrative identity operation and does not alter code authority. Historical GitHub URLs and exact evidence references may continue to resolve through GitHub redirects and remain valid as historical evidence.\n`;
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text, "utf8");
}

// Fail if current files still carry the superseded product identity. Only explicit historical records may do so.
const allowedResidue = (path) =>
  path === "docs/identity/QDRAL_RENAME.md" ||
  path === "docs/identity/QDRAL_RENAME.md" ||
  path === "docs/canonical/CURRENT.md" ||
  path === ".specgrain/canonical-evidence.json" ||
  path === ".specgrain/ledger.json" ||
  (path.startsWith(".specgrain/specs/") && path !== ".specgrain/specs/SG-000067.json") ||
  path === ".github/scripts/qdral-rename.mjs" ||
  path === ".github/workflows/qdral-rename.yml";

const residue = [];
for (const path of tracked()) {
  if (allowedResidue(path)) continue;
  const bytes = readFileSync(path);
  if (isBinary(bytes)) continue;
  const text = bytes.toString("utf8");
  if (/Qdral|QDRAL|qdral/.test(text)) residue.push(path);
}
if (residue.length) {
  throw new Error(`superseded Qdral identity remains outside historical allowlist:\n${residue.join("\n")}`);
}

// Current paths may not carry the superseded identity either.
const pathResidue = tracked().filter((path) => path !== "docs/identity/QDRAL_RENAME.md" && /Qdral|QDRAL|qdral/.test(path));
if (pathResidue.length) throw new Error(`superseded identity remains in current paths:\n${pathResidue.join("\n")}`);

// Validate the active grain and basic repository hygiene before committing.
JSON.parse(readFileSync(".specgrain/specs/SG-000067.json", "utf8"));
execFileSync("git", ["diff", "--check"], { stdio: "inherit" });
console.log("Qdral identity transform complete.");
console.log(git("status", "--short"));
