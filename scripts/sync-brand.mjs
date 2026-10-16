#!/usr/bin/env node
// Orcel's single geometric identity, no third-party runtime or image dependency.
// A continuous circular exterior and an optical rounded-square aperture.
// The white symbol is the mark; the black tile is only a contrast surface.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
export const MARK_PATH = "M128 16a112 112 0 1 0 0 224a112 112 0 1 0 0-224ZM101 71H155C174 71 185 83 185 102V154C185 173 174 185 155 185H101C82 185 71 173 71 154V102C71 83 82 71 101 71Z";
const head = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" role="img" aria-label="Orcel">';
const mark = '<path fill="#ffffff" fill-rule="evenodd" d="' + MARK_PATH + '"/>';
const pure = head + "\n  " + mark + "\n</svg>\n";
const tile = head + '\n  <rect width="256" height="256" rx="56" fill="#080808"/>\n  ' + mark + "\n</svg>\n";
const outputs = new Map([
  ["assets/brand/orcel-o-white.svg", pure],
  ["assets/brand/orcel-app-icon.svg", tile],
  ["apps/web/public/brand/orcel-o-white.svg", pure],
  ["apps/web/public/brand/orcel-app-icon.svg", tile],
  ["apps/web/app/icon.svg", tile],
  ["distribution/openai/assets/logo.svg", tile],
  ["distribution/codex/orcel/assets/logo.svg", tile]
]);
let failures = 0;
for (const [relative, expected] of outputs) {
  const path = join(repo, ...relative.split("/"));
  if (process.argv.includes("--check")) {
    let actual;
    try { actual = readFileSync(path, "utf8"); } catch { actual = undefined; }
    if (actual !== expected) { console.error("Out of sync: " + relative); failures++; }
  } else {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, expected);
  }
}
if (failures) process.exit(1);
console.log("Orcel single-geometry brand assets " + (process.argv.includes("--check") ? "verified" : "generated") + ": " + outputs.size);
console.log("white mark sha256: " + createHash("sha256").update(pure).digest("hex"));
console.log("app tile sha256: " + createHash("sha256").update(tile).digest("hex"));
