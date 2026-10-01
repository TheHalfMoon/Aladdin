#!/usr/bin/env node
// Writes a deterministic ZIP archive of a release directory: entries in
// sorted order, forward-slash names, a fixed timestamp (1980-01-01), fixed
// attributes, and DEFLATE data from Node's bundled zlib. The same release
// directory therefore always produces the same archive bytes with the same
// Node.js version.
//
// Usage:
//   node scripts/archive-release.mjs --release <release-dir> --out <file.zip>

import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { deflateRawSync } from "node:zlib";

function fail(message) {
  console.error(`archive-release: ${message}`);
  process.exit(1);
}

function arg(name) {
  const index = process.argv.indexOf(name);
  if (index === -1 || !process.argv[index + 1]) fail(`${name} <value> is required`);
  return resolve(process.argv[index + 1]);
}

const CRC_TABLE = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function walk(dir, out) {
  for (const name of readdirSync(dir).sort()) {
    const path = join(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) walk(path, out);
    else if (stat.isFile()) out.push(path);
  }
}

const DOS_TIME = 0;
const DOS_DATE = (0 << 9) | (1 << 5) | 1; // 1980-01-01
const release = arg("--release");
const out = arg("--out");
const relativeOut = relative(release, out);
if (!relativeOut.startsWith("..") && !relativeOut.startsWith(sep) && !/^[A-Za-z]:/.test(relativeOut)) {
  fail("--out must be outside the release directory");
}
const files = [];
walk(release, files);
const entries = files
  .map((path) => ({ path, name: relative(release, path).split(sep).join("/") }))
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));

const chunks = [];
const central = [];
let offset = 0;
for (const entry of entries) {
  const data = readFileSync(entry.path);
  const compressed = deflateRawSync(data, { level: 9 });
  const name = Buffer.from(entry.name, "utf8");
  const crc = crc32(data);
  const local = Buffer.alloc(30);
  local.writeUInt32LE(0x04034b50, 0);
  local.writeUInt16LE(20, 4);
  local.writeUInt16LE(0x0800, 6); // UTF-8 names
  local.writeUInt16LE(8, 8); // deflate
  local.writeUInt16LE(DOS_TIME, 10);
  local.writeUInt16LE(DOS_DATE, 12);
  local.writeUInt32LE(crc, 14);
  local.writeUInt32LE(compressed.length, 18);
  local.writeUInt32LE(data.length, 22);
  local.writeUInt16LE(name.length, 26);
  local.writeUInt16LE(0, 28);
  chunks.push(local, name, compressed);
  const header = Buffer.alloc(46);
  header.writeUInt32LE(0x02014b50, 0);
  header.writeUInt16LE(20, 4);
  header.writeUInt16LE(20, 6);
  header.writeUInt16LE(0x0800, 8);
  header.writeUInt16LE(8, 10);
  header.writeUInt16LE(DOS_TIME, 12);
  header.writeUInt16LE(DOS_DATE, 14);
  header.writeUInt32LE(crc, 16);
  header.writeUInt32LE(compressed.length, 20);
  header.writeUInt32LE(data.length, 24);
  header.writeUInt16LE(name.length, 28);
  header.writeUInt16LE(0, 30);
  header.writeUInt16LE(0, 32);
  header.writeUInt16LE(0, 34);
  header.writeUInt16LE(0, 36);
  header.writeUInt32LE(0, 38);
  header.writeUInt32LE(offset, 42);
  central.push(header, name);
  offset += local.length + name.length + compressed.length;
  if (offset > 0xffffffff) fail("release is too large for a non-ZIP64 archive");
}
const centralBytes = Buffer.concat(central);
const end = Buffer.alloc(22);
end.writeUInt32LE(0x06054b50, 0);
end.writeUInt16LE(entries.length, 8);
end.writeUInt16LE(entries.length, 10);
end.writeUInt32LE(centralBytes.length, 12);
end.writeUInt32LE(offset, 16);
writeFileSync(out, Buffer.concat([...chunks, centralBytes, end]));
console.log(`archive-release: ${entries.length} entries -> ${out}`);
