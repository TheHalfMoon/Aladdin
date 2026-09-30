#!/usr/bin/env node
// Assembles a Cotra Windows release directory and its manifest.json.
//
// Usage:
//   node scripts/package-release.mjs --out <empty-or-new-dir> [--target-dir target/release]
//
// Prerequisites: `cargo build --release -p cotrad -p cotra-lifecycle` on
// Windows and `npm run build` for the MCP app. The script copies only runtime
// payload (no tests, type declarations, or source maps), installs the MCP
// app's production dependencies, and writes a manifest whose paths satisfy
// the same rules the installer enforces.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const MANIFEST_SCHEMA = "cotra-release-manifest-v1";
const CONFIG_SCHEMA = { min: 1, max: 1 };
const BINARIES = ["cotra.exe", "cotrad.exe"];

function fail(message) {
  console.error(`package-release: ${message}`);
  process.exit(1);
}

function arg(name, fallback) {
  const index = process.argv.indexOf(name);
  if (index === -1) return fallback;
  const value = process.argv[index + 1];
  if (!value || value.startsWith("--")) fail(`${name} requires a value`);
  return value;
}

function workspaceVersion() {
  const cargo = readFileSync(join(repo, "Cargo.toml"), "utf8");
  const match = cargo.match(/\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/);
  if (!match) fail("workspace version not found in Cargo.toml");
  const app = JSON.parse(readFileSync(join(repo, "apps/cotra-mcp/package.json"), "utf8"));
  if (app.version !== match[1]) {
    fail(`Cargo workspace version ${match[1]} does not match @cotra/mcp ${app.version}`);
  }
  return match[1];
}

// Mirrors cotra_lifecycle::manifest::validate_relative_path.
function validateRelativePath(path) {
  if (path.length === 0 || path.length > 200) return "length out of range";
  if (path.startsWith("/")) return "absolute path";
  for (const segment of path.split("/")) {
    if (segment === "" || segment === "." || segment === "..") return "empty or relative segment";
    if (segment.endsWith(".") || segment.endsWith(" ")) return "trailing dot or space";
    if (!/^[\x20-\x7e]+$/.test(segment) || /[<>:"\\|?*]/.test(segment)) return "forbidden character";
    const stem = segment.split(".")[0].toUpperCase();
    if (["CON", "PRN", "AUX", "NUL"].includes(stem) || /^(COM|LPT)[0-9]$/.test(stem)) {
      return "reserved device name";
    }
  }
  return null;
}

function walk(dir, out) {
  for (const name of readdirSync(dir).sort()) {
    const path = join(dir, name);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) fail(`release payload must not contain links: ${path}`);
    if (stat.isDirectory()) walk(path, out);
    else if (stat.isFile()) out.push(path);
    else fail(`unsupported payload entry: ${path}`);
  }
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

const out = resolve(arg("--out", ""));
if (!process.argv.includes("--out")) fail("--out <dir> is required");
const targetDir = resolve(repo, arg("--target-dir", "target/release"));
const version = workspaceVersion();

if (existsSync(out) && readdirSync(out).length > 0) fail(`${out} must be empty or absent`);
mkdirSync(out, { recursive: true });

for (const binary of BINARIES) {
  const from = join(targetDir, binary);
  if (!existsSync(from)) fail(`missing ${from}; run cargo build --release on Windows first`);
  copyFileSync(from, join(out, binary));
}
copyFileSync(join(repo, "LICENSE"), join(out, "LICENSE"));

const appSource = join(repo, "apps/cotra-mcp");
const appOut = join(out, "app/cotra-mcp");
mkdirSync(join(appOut, "dist"), { recursive: true });
const pkg = JSON.parse(readFileSync(join(appSource, "package.json"), "utf8"));
const runtimePackage = {
  name: pkg.name,
  version: pkg.version,
  private: true,
  type: pkg.type,
  engines: pkg.engines,
  dependencies: pkg.dependencies
};
writeFileSync(join(appOut, "package.json"), `${JSON.stringify(runtimePackage, null, 2)}\n`);
const distSource = join(appSource, "dist");
if (!existsSync(join(distSource, "index.js"))) fail("apps/cotra-mcp/dist/index.js missing; run npm run build first");
for (const name of readdirSync(distSource).sort()) {
  if (!name.endsWith(".js") || name.endsWith(".test.js")) continue;
  copyFileSync(join(distSource, name), join(appOut, "dist", name));
}

// Run npm through the current Node executable so no shell is involved.
const npmCli = join(dirname(process.execPath), "node_modules", "npm", "bin", "npm-cli.js");
const npmArgs = ["install", "--omit=dev", "--ignore-scripts", "--no-audit", "--no-fund", "--no-package-lock"];
if (existsSync(npmCli)) {
  execFileSync(process.execPath, [npmCli, ...npmArgs], { cwd: appOut, stdio: "inherit" });
} else if (process.platform !== "win32") {
  execFileSync("npm", npmArgs, { cwd: appOut, stdio: "inherit" });
} else {
  fail(`npm CLI not found next to ${process.execPath}`);
}
rmSync(join(appOut, "node_modules", ".bin"), { recursive: true, force: true });
rmSync(join(appOut, "node_modules", ".package-lock.json"), { force: true });

const files = [];
walk(out, files);
const entries = files
  .map((path) => relative(out, path).split(sep).join("/"))
  .filter((path) => path !== "manifest.json")
  .sort()
  .map((path) => {
    const problem = validateRelativePath(path);
    if (problem) fail(`payload path ${JSON.stringify(path)} is not installable: ${problem}`);
    const full = join(out, ...path.split("/"));
    return { path, size: lstatSync(full).size, sha256: sha256(full) };
  });

const manifest = { schema: MANIFEST_SCHEMA, version, config_schema: CONFIG_SCHEMA, files: entries };
writeFileSync(join(out, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`package-release: Cotra ${version} with ${entries.length} files in ${out}`);
