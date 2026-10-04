#!/usr/bin/env node
// SG-000067: build and validate the OpenAI plugin package from source.
//   npm run build -w @qdral/mcp
//   node scripts/build-openai-plugin.mjs --mcp-url https://<your-relay-host>/mcp --out <dir>
// The relay endpoint is supplied at build time and never committed.
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
const dist = (name) => pathToFileURL(join(repo, "apps", "qdral-mcp", "dist", name)).href;
const { buildOpenAIPluginPackage, validateOpenAIPluginPackage } = await import(dist("openai_plugin.js"));
const { discoverTools } = await import(dist("discovery.js"));

const argv = process.argv.slice(2);
const option = (name) => {
  const index = argv.indexOf(name);
  return index >= 0 ? argv[index + 1] : undefined;
};
const mcpUrl = option("--mcp-url");
const out = option("--out");
if (mcpUrl === undefined || out === undefined) {
  process.stderr.write("usage: node scripts/build-openai-plugin.mjs --mcp-url https://<relay-host>/mcp --out <dir>\n");
  process.exit(2);
}

const source = join(repo, "distribution", "openai");
const config = JSON.parse(readFileSync(join(source, "plugin.config.json"), "utf8"));
const version = JSON.parse(readFileSync(join(repo, "apps", "qdral-mcp", "package.json"), "utf8")).version;
const reviewFiles = Object.fromEntries(
  readdirSync(join(source, "review")).map((name) => [name, readFileSync(join(source, "review", name), "utf8")])
);
const tools = await discoverTools({ transportKind: "relay", providerKind: "openai", toolSurfaceProfile: "core" });
const files = buildOpenAIPluginPackage({
  config,
  version,
  mcpUrl,
  logoSvg: readFileSync(join(source, "assets", "logo.svg"), "utf8"),
  tools,
  reviewFiles
});
const problems = validateOpenAIPluginPackage(files);
if (problems.length > 0) {
  process.stderr.write(`OpenAI plugin package is invalid:\n${problems.join("\n")}\n`);
  process.exit(1);
}
for (const [path, text] of Object.entries(files)) {
  mkdirSync(dirname(join(out, path)), { recursive: true });
  writeFileSync(join(out, path), text);
}
process.stdout.write(`wrote ${Object.keys(files).length} files to ${out} (${tools.length} core tools)\n`);
