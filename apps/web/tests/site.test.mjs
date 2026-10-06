import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import test from "node:test";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const repo = join(root, "..", "..");

const read = (path) => readFile(path, "utf8");
const sha256 = async (path) =>
  createHash("sha256").update(await readFile(path)).digest("hex");

test("web dependencies are exact-pinned and telemetry is disabled in every Next command", async () => {
  const pkg = JSON.parse(await read(join(root, "package.json")));
  for (const group of ["dependencies", "devDependencies"]) {
    for (const [name, version] of Object.entries(pkg[group] ?? {})) {
      assert.match(version, /^\d+\.\d+\.\d+$/, `${name} must be exact-pinned`);
    }
  }
  for (const command of ["dev", "build", "start"]) {
    assert.match(pkg.scripts[command], /NEXT_TELEMETRY_DISABLED=1/);
  }
});

test("public copy uses Deskal identity and keeps compatibility naming bounded", async () => {
  const page = await read(join(root, "app", "page.tsx"));
  assert.match(page, /Deskal/);
  for (const staleName of ["Cotra", "Qun" + "tal"]) {
    assert.equal(page.includes(staleName), false, `stale product identity: ${staleName}`);
  }
  const qdralMentions = page.match(/qdral/g) ?? [];
  assert.equal(qdralMentions.length, 2, "qdral may appear only in the explicit compatibility example and note");
  assert.match(page, /Compatibility note/);
});

test("website source contains no analytics, tracking, dynamic service, auth, payment, or form dependency", async () => {
  const sources = [
    await read(join(root, "app", "page.tsx")),
    await read(join(root, "app", "layout.tsx")),
    await read(join(root, "app", "globals.css")),
    await read(join(root, "next.config.ts"))
  ].join("\n").toLowerCase();

  for (const forbidden of [
    "google-analytics",
    "googletagmanager",
    "posthog",
    "segment.com",
    "mixpanel",
    "sentry",
    "@vercel/analytics",
    "@vercel/speed-insights",
    "stripe",
    "auth0",
    "clerk",
    "supabase",
    "<form",
    "formaction",
    "xmlhttprequest",
    "sendbeacon",
    "websocket(",
    "document.cookie",
    "fetch("
  ]) {
    assert.equal(sources.includes(forbidden), false, `forbidden hosted dependency marker: ${forbidden}`);
  }
});

test("Next output is a static export with a bounded explicit project base path", async () => {
  const config = await read(join(root, "next.config.ts"));
  assert.match(config, /output:\s*"export"/);
  assert.match(config, /DESKAL_WEB_BASE_PATH/);
  assert.match(config, /basePath,/);
  assert.match(config, /one absolute path segment/);
  assert.doesNotMatch(config, /rewrites|redirects|headers\s*\(|serverRuntimeConfig/);

  const layout = await read(join(root, "app", "layout.tsx"));
  const css = await read(join(root, "app", "globals.css"));
  assert.match(layout, /--deskal-mark-mask/);
  assert.match(layout, /brand\/deskal-mark-mask\.svg/);
  assert.equal(css.includes('url("/brand/deskal-mark-mask.svg")'), false);
  assert.equal((css.match(/var\(--deskal-mark-mask\)/g) ?? []).length, 2);
});

test("approved Deskal brand geometry is byte-pinned and shared by both variants", async () => {
  assert.equal(
    await sha256(join(root, "public", "brand", "deskal-mark-mask.svg")),
    "150ae5ccd9fdb6b25a8f0421a64acc2d0bf1615a32c2b33aea92e77700706fce"
  );
  const css = await read(join(root, "app", "globals.css"));
  assert.equal((css.match(/var\(--deskal-mark-mask\)/g) ?? []).length, 2, "both mask declarations must use the one canonical geometry variable");
  assert.match(css, /\.brand-mark-white[\s\S]*color:\s*#fff/);
  assert.match(css, /\.brand-mark-color[\s\S]*linear-gradient/);
});

test("web dependency graph is isolated from runtime workspaces and lockfile", async () => {
  const pkg = JSON.parse(await read(join(repo, "package.json")));
  assert.deepEqual([...pkg.workspaces].sort(), ["apps/qdral-mcp", "apps/qdral-relay"].sort());
  assert.equal(pkg.scripts["web:typecheck"], "npm --prefix apps/web run typecheck");
  assert.equal(pkg.scripts["web:test"], "npm --prefix apps/web run test");
  assert.equal(pkg.scripts["web:build"], "npm --prefix apps/web run build");

  const runtimeLock = JSON.parse(await read(join(repo, "package-lock.json")));
  assert.equal(runtimeLock.packages?.["apps/web"], undefined, "runtime lockfile must not contain the web app");

  const webLock = JSON.parse(await read(join(root, "package-lock.json")));
  assert.equal(webLock.packages?.[""]?.name, "@deskal/web");
  assert.equal(webLock.packages?.[""]?.dependencies?.next, "16.3.8");
  assert.equal(webLock.packages?.[""]?.dependencies?.react, "19.3.0");
  assert.equal(webLock.packages?.[""]?.dependencies?.["react-dom"], "19.3.0");
});

test("public launch links stay on canonical Deskal repository, docs, and Releases surfaces", async () => {
  const page = await read(join(root, "app", "page.tsx"));
  const readme = await read(join(repo, "README.md"));

  assert.match(page, /const github = "https:\/\/github\.com\/TheHalfMoon\/Deskal"/);
  assert.match(page, /const releases = `\$\{github\}\/releases`/);
  assert.match(page, /View verified releases/);
  assert.equal(page.includes("/releases/download/"), false, "website must not link directly to release artifacts");
  assert.equal(page.includes(".zip"), false, "website must not hard-code downloadable archives");

  assert.match(readme, /https:\/\/thehalfmoon\.github\.io\/Deskal\//);
  assert.match(readme, /https:\/\/github\.com\/TheHalfMoon\/Deskal\/releases/);
});

test("Pages workflow is zero-cost, pinned, secret-free, and least-privilege", async () => {
  const workflow = await read(join(repo, ".github", "workflows", "pages.yml"));

  for (const pin of [
    "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
    "actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020",
    "actions/configure-pages@45bfe0192ca1faeb007ade9deae92b16b8254a0d",
    "actions/upload-pages-artifact@fc324d3547104276b827a68afc52ff2a11cc49c9",
    "actions/deploy-pages@368f82528645a54fb793d4d04e342629a3f51346"
  ]) {
    assert.equal(workflow.includes(pin), true, "workflow action must be commit-pinned");
  }

  assert.match(workflow, /contents:\s*read/);
  assert.match(workflow, /pages:\s*read/);
  assert.match(workflow, /pages:\s*write/);
  assert.match(workflow, /id-token:\s*write/);
  assert.match(workflow, /permissions:\s*\{\}/);
  assert.doesNotMatch(workflow, /secrets\./);
  assert.doesNotMatch(workflow, /pull_request:/);
  assert.doesNotMatch(workflow, /schedule:/);
  assert.match(workflow, /test ! -e apps\/web\/public\/CNAME/);
  assert.doesNotMatch(workflow, /cname:\s*[^\s]/i);
});

test("Pages workflow builds the canonical project path and verifies the deployed HTTPS site", async () => {
  const workflow = await read(join(repo, ".github", "workflows", "pages.yml"));

  assert.match(workflow, /DESKAL_WEB_BASE_PATH/);
  assert.match(workflow, /base_path/);
  assert.match(workflow, /test "\$\{\{ steps\.pages\.outputs\.base_path \}\}" = "\/Deskal"/);
  assert.match(workflow, /\/Deskal\/_next\//);
  assert.match(workflow, /\/Deskal\/brand\/deskal-mark-mask\.svg/);
  assert.match(workflow, /https:\/\/thehalfmoon\.github\.io\/Deskal/);
  assert.match(workflow, /curl --fail --location/);
  assert.match(workflow, /https:\/\/github\.com\/TheHalfMoon\/Deskal\/releases/);
});

test("canonical repository metadata is exact, bounded, and aligned with public entrypoints", async () => {
  const metadata = JSON.parse(await read(join(repo, "docs", "canonical", "REPOSITORY_METADATA.json")));
  const page = await read(join(root, "app", "page.tsx"));
  const readme = await read(join(repo, "README.md"));

  assert.deepEqual(Object.keys(metadata).sort(), ["description", "homepage", "repository", "topics"]);
  assert.equal(metadata.repository, "TheHalfMoon/Deskal");
  assert.equal(
    metadata.description,
    "Local-first MCP authority layer for safe, bounded computer use and developer workflows."
  );
  assert.equal(metadata.homepage, "https://thehalfmoon.github.io/Deskal/");
  assert.deepEqual(metadata.topics, [
    "ai-agents",
    "computer-use",
    "desktop-automation",
    "local-first",
    "mcp",
    "mcp-server",
    "windows"
  ]);
  assert.equal(new Set(metadata.topics).size, metadata.topics.length, "topics must be unique");

  assert.equal(readme.includes(`**Website:** ${metadata.homepage}`), true);
  assert.equal(page.includes("https://github.com/TheHalfMoon/Deskal"), true);
  assert.equal(page.includes("/releases/download/"), false);
});


test("canonical and social metadata stay exact, static, and bounded to the Pages identity", async () => {
  const layout = await read(join(root, "app", "layout.tsx"));

  assert.match(layout, /const siteUrl = "https:\/\/thehalfmoon\.github\.io\/Deskal\/"/);
  assert.match(layout, /metadataBase:\s*new URL\(siteUrl\)/);
  assert.match(layout, /alternates:\s*\{[\s\S]*canonical:\s*siteUrl[\s\S]*\}/);
  assert.match(layout, /openGraph:\s*\{[\s\S]*url:\s*siteUrl[\s\S]*siteName:\s*"Deskal"/);
  assert.match(layout, /twitter:\s*\{[\s\S]*card:\s*"summary"[\s\S]*title[\s\S]*description/);
  assert.equal((layout.match(/https:\/\/thehalfmoon\.github\.io\/Deskal\//g) ?? []).length, 1);
  assert.doesNotMatch(layout, /images:\s*\[/);
});

test("responsive primary navigation stays keyboard and touch usable without client runtime", async () => {
  const page = await read(join(root, "app", "page.tsx"));
  const css = await read(join(root, "app", "globals.css"));

  assert.match(page, /<nav className="desktop-nav" aria-label="Primary navigation">/);
  assert.match(page, /<details className="mobile-nav">/);
  assert.match(page, /<summary aria-label="Toggle primary navigation">/);
  assert.match(page, /<nav className="mobile-nav-panel" aria-label="Mobile primary navigation">/);
  assert.match(page, /<div id="content" tabIndex=\{-1\}>/);

  for (const marker of ['"use client"', "onClick=", "onKeyDown=", "useState(", "useEffect("]) {
    assert.equal(page.includes(marker), false, `static navigation must not add client behavior: ${marker}`);
  }

  assert.match(css, /a:focus-visible,\s*summary:focus-visible/);
  assert.match(css, /\.mobile-nav summary\s*\{[\s\S]*min-height:\s*44px/);
  assert.match(css, /\.mobile-nav-panel a\s*\{[\s\S]*min-height:\s*48px/);
  assert.match(css, /@media \(max-width: 960px\)[\s\S]*\.mobile-nav\s*\{\s*display:\s*block/);
  assert.match(css, /@media \(prefers-reduced-motion: reduce\)[\s\S]*scroll-behavior:\s*auto/);
});
