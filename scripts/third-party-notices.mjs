#!/usr/bin/env node
// Writes THIRD_PARTY_NOTICES.txt for a Cotra release: for every third-party
// crate linked into the shipped binaries (x86_64-pc-windows-msvc, normal
// dependency edges only) and every npm package in the release's
// app/cotra-mcp/node_modules, the declared license expression and the full
// text of each license or notice file the package ships.
//
// Usage:
//   node scripts/third-party-notices.mjs --release <release-dir> --out <file>

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TARGET = "x86_64-pc-windows-msvc";
const ROOT_PACKAGES = ["cotrad", "cotra-lifecycle"];
const NOTICE_FILE = /^(licen[cs]e|copying|notice|unlicense)([-_.].*)?$/i;

function fail(message) {
  console.error(`third-party-notices: ${message}`);
  process.exit(1);
}

function arg(name) {
  const index = process.argv.indexOf(name);
  if (index === -1 || !process.argv[index + 1]) fail(`${name} <value> is required`);
  return resolve(process.argv[index + 1]);
}

function noticeFiles(dir) {
  return readdirSync(dir)
    .filter((name) => NOTICE_FILE.test(name) && statSync(join(dir, name)).isFile())
    .sort()
    .map((name) => ({ name, text: readFileSync(join(dir, name), "utf8").replace(/\r\n/g, "\n").trimEnd() }));
}

function crates() {
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--filter-platform", TARGET], {
      cwd: repo,
      maxBuffer: 256 * 1024 * 1024
    }).toString()
  );
  const packages = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const reached = new Set();
  const stack = metadata.packages.filter((p) => ROOT_PACKAGES.includes(p.name) && !p.source).map((p) => p.id);
  while (stack.length) {
    const id = stack.pop();
    if (reached.has(id)) continue;
    reached.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (dep.dep_kinds.some((kind) => kind.kind === null)) stack.push(dep.pkg);
    }
  }
  return [...reached]
    .map((id) => packages.get(id))
    .filter((p) => p.source)
    .map((p) => ({ kind: "crate", name: p.name, version: p.version, license: p.license, dir: dirname(p.manifest_path) }));
}

function npmPackages(release) {
  const found = [];
  const visit = (dir) => {
    if (!existsSync(dir)) return;
    for (const name of readdirSync(dir).sort()) {
      if (name.startsWith(".")) continue;
      const path = join(dir, name);
      if (name.startsWith("@")) {
        visit(path);
        continue;
      }
      if (existsSync(join(path, "package.json"))) {
        const pkg = JSON.parse(readFileSync(join(path, "package.json"), "utf8"));
        found.push({ kind: "npm", name: pkg.name, version: pkg.version, license: pkg.license, dir: path });
        visit(join(path, "node_modules"));
      }
    }
  };
  visit(join(release, "app", "cotra-mcp", "node_modules"));
  return found;
}

const release = arg("--release");
const out = arg("--out");
const entries = [...crates(), ...npmPackages(release)].sort((a, b) =>
  `${a.kind}:${a.name}@${a.version}` < `${b.kind}:${b.name}@${b.version}` ? -1 : 1
);
const missing = [];
let text =
  "Cotra third-party notices\n\n" +
  "Cotra is licensed under the Apache License 2.0 (see LICENSE). This release\n" +
  "includes the following third-party components, each under its own license.\n";
for (const entry of entries) {
  const files = noticeFiles(entry.dir);
  if (!entry.license || files.length === 0) missing.push(`${entry.kind} ${entry.name}@${entry.version}`);
  text += `\n${"=".repeat(78)}\n${entry.name} ${entry.version} (${entry.kind})\nLicense: ${entry.license ?? "UNDECLARED"}\n`;
  for (const file of files) {
    text += `\n--- ${file.name} ---\n${file.text}\n`;
  }
}
if (missing.length) {
  text += `\n${"=".repeat(78)}\nComponents without a declared license or shipped license file:\n${missing.join("\n")}\n`;
}
writeFileSync(out, text);
console.log(`third-party-notices: ${entries.length} components; ${missing.length} without a shipped license file -> ${out}`);
