#!/usr/bin/env node
// Writes a deterministic CycloneDX 1.5 SBOM for a packaged Cotra release.
//
// Usage:
//   node scripts/generate-sbom.mjs --release <release-dir> --out <sbom.cdx.json>
//
// The Rust components are the crates actually linked into the shipped
// binaries (cotrad, cotra, cotra-mcp-host) for x86_64-pc-windows-msvc, taken
// from `cargo metadata` and following only normal (non-dev, non-build)
// dependency edges. The npm components are the packages present in the
// release's app/cotra-mcp/node_modules. Every payload file listed in the
// release manifest is recorded with its SHA-256.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TARGET = "x86_64-pc-windows-msvc";
const ROOT_PACKAGES = ["cotrad", "cotra-lifecycle"];

function fail(message) {
  console.error(`generate-sbom: ${message}`);
  process.exit(1);
}

function arg(name) {
  const index = process.argv.indexOf(name);
  if (index === -1 || !process.argv[index + 1]) fail(`${name} <value> is required`);
  return resolve(process.argv[index + 1]);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

/** Deterministic UUID (version 5 layout) derived from the release manifest. */
function uuidFrom(hex) {
  const b = hex.slice(0, 32).split("");
  b[12] = "5";
  b[16] = ((parseInt(b[16], 16) & 0x3) | 0x8).toString(16);
  const s = b.join("");
  return `${s.slice(0, 8)}-${s.slice(8, 12)}-${s.slice(12, 16)}-${s.slice(16, 20)}-${s.slice(20, 32)}`;
}

function npmPurl(name, version) {
  return `pkg:npm/${name.replace("@", "%40")}@${version}`;
}

function cargoComponents() {
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--filter-platform", TARGET], {
      cwd: repo,
      maxBuffer: 256 * 1024 * 1024
    }).toString()
  );
  const lock = readFileSync(join(repo, "Cargo.lock"), "utf8");
  const checksums = new Map();
  for (const block of lock.split("[[package]]")) {
    const name = block.match(/\nname = "([^"]+)"/)?.[1];
    const version = block.match(/\nversion = "([^"]+)"/)?.[1];
    const checksum = block.match(/\nchecksum = "([0-9a-f]{64})"/)?.[1];
    if (name && version && checksum) checksums.set(`${name}@${version}`, checksum);
  }
  const packages = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const roots = metadata.packages.filter((p) => ROOT_PACKAGES.includes(p.name) && !p.source);
  if (roots.length !== ROOT_PACKAGES.length) fail("workspace root packages not found");

  const reached = new Set();
  const edges = new Map();
  const stack = roots.map((p) => p.id);
  while (stack.length) {
    const id = stack.pop();
    if (reached.has(id)) continue;
    reached.add(id);
    const next = (nodes.get(id)?.deps ?? [])
      .filter((dep) => dep.dep_kinds.some((kind) => kind.kind === null))
      .map((dep) => dep.pkg);
    edges.set(id, next);
    stack.push(...next);
  }

  const ref = (p) => (p.source ? `pkg:cargo/${p.name}@${p.version}` : `cotra:${p.name}@${p.version}`);
  const components = [];
  for (const id of reached) {
    const p = packages.get(id);
    const component = {
      type: "library",
      "bom-ref": ref(p),
      name: p.name,
      version: p.version,
      licenses: p.license ? [{ expression: p.license }] : [],
      ...(p.source ? { purl: ref(p) } : { description: "Cotra workspace crate" })
    };
    const checksum = checksums.get(`${p.name}@${p.version}`);
    if (checksum) component.hashes = [{ alg: "SHA-256", content: checksum }];
    if (p.repository) component.externalReferences = [{ type: "vcs", url: p.repository }];
    components.push(component);
  }
  const dependencies = [...reached].map((id) => ({
    ref: ref(packages.get(id)),
    dependsOn: [...new Set(edges.get(id).map((dep) => ref(packages.get(dep))))].sort()
  }));
  return { components, dependencies, rootRefs: roots.map(ref) };
}

function npmComponents(release) {
  const base = join(release, "app", "cotra-mcp", "node_modules");
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
      const manifest = join(path, "package.json");
      if (existsSync(manifest)) {
        const pkg = JSON.parse(readFileSync(manifest, "utf8"));
        found.push({ pkg, path });
        visit(join(path, "node_modules"));
      }
    }
  };
  visit(base);
  const appPkg = JSON.parse(readFileSync(join(release, "app", "cotra-mcp", "package.json"), "utf8"));
  const components = new Map();
  const dependencies = new Map();
  for (const { pkg } of found) {
    const ref = npmPurl(pkg.name, pkg.version);
    components.set(ref, {
      type: "library",
      "bom-ref": ref,
      name: pkg.name,
      version: pkg.version,
      purl: ref,
      licenses: pkg.license ? [{ expression: pkg.license }] : []
    });
    const deps = Object.keys(pkg.dependencies ?? {})
      .map((dep) => found.find((entry) => entry.pkg.name === dep))
      .filter(Boolean)
      .map((entry) => npmPurl(entry.pkg.name, entry.pkg.version));
    dependencies.set(ref, [...new Set(deps)].sort());
  }
  const appRef = `cotra:${appPkg.name}@${appPkg.version}`;
  const appDeps = Object.keys(appPkg.dependencies ?? {})
    .map((dep) => found.find((entry) => entry.pkg.name === dep))
    .filter(Boolean)
    .map((entry) => npmPurl(entry.pkg.name, entry.pkg.version))
    .sort();
  return {
    components: [
      { type: "application", "bom-ref": appRef, name: appPkg.name, version: appPkg.version, description: "Cotra MCP server app" },
      ...components.values()
    ],
    dependencies: [{ ref: appRef, dependsOn: appDeps }, ...[...dependencies].map(([ref, dependsOn]) => ({ ref, dependsOn }))],
    rootRef: appRef
  };
}

const release = arg("--release");
const out = arg("--out");
const manifestBytes = readFileSync(join(release, "manifest.json"));
const manifest = JSON.parse(manifestBytes);
const manifestHash = sha256(manifestBytes);
const cargo = cargoComponents();
const npm = npmComponents(release);
const files = manifest.files.map((file) => ({
  type: "file",
  "bom-ref": `file:${file.path}`,
  name: file.path,
  hashes: [{ alg: "SHA-256", content: file.sha256 }]
}));
const productRef = `cotra@${manifest.version}`;
const sortByRef = (a, b) => (a["bom-ref"] < b["bom-ref"] ? -1 : a["bom-ref"] > b["bom-ref"] ? 1 : 0);
const sbom = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  serialNumber: `urn:uuid:${uuidFrom(manifestHash)}`,
  version: 1,
  metadata: {
    component: {
      type: "application",
      "bom-ref": productRef,
      name: "cotra",
      version: manifest.version,
      description: "Cotra Windows x64 release payload",
      licenses: [{ expression: "Apache-2.0" }],
      hashes: [{ alg: "SHA-256", content: manifestHash }]
    },
    properties: [
      { name: "cotra:manifest-sha256", value: manifestHash },
      { name: "cotra:rust-target", value: TARGET }
    ]
  },
  components: [...cargo.components, ...npm.components, ...files].sort(sortByRef),
  dependencies: [
    { ref: productRef, dependsOn: [...cargo.rootRefs, npm.rootRef, ...files.map((f) => f["bom-ref"])].sort() },
    ...cargo.dependencies,
    ...npm.dependencies
  ].sort((a, b) => (a.ref < b.ref ? -1 : a.ref > b.ref ? 1 : 0))
};
writeFileSync(out, `${JSON.stringify(sbom, null, 2)}\n`);
console.log(
  `generate-sbom: ${cargo.components.length} crates, ${npm.components.length} npm components, ${files.length} files -> ${out}`
);
