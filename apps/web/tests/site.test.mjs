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

test("website source contains no analytics, tracking, auth, payment, or hosted backend dependency", async () => {
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
    "supabase"
  ]) {
    assert.equal(sources.includes(forbidden), false, `forbidden hosted dependency marker: ${forbidden}`);
  }
});

test("Next output is a static export with no application backend", async () => {
  const config = await read(join(root, "next.config.ts"));
  assert.match(config, /output:\s*"export"/);
  assert.doesNotMatch(config, /rewrites|redirects|headers\s*\(|serverRuntimeConfig/);
});

test("approved Deskal brand geometry is byte-pinned and shared by both variants", async () => {
  assert.equal(
    await sha256(join(root, "public", "brand", "deskal-mark-mask.svg")),
    "150ae5ccd9fdb6b25a8f0421a64acc2d0bf1615a32c2b33aea92e77700706fce"
  );
  const css = await read(join(root, "app", "globals.css"));
  const geometryRefs = css.match(/deskal-mark-mask\.svg/g) ?? [];
  assert.equal(geometryRefs.length, 2, "both mask declarations must use the one canonical geometry asset");
  assert.match(css, /\.brand-mark-white[\s\S]*color:\s*#fff/);
  assert.match(css, /\.brand-mark-color[\s\S]*linear-gradient/);
});

test("root workspace exposes independent web qualification scripts", async () => {
  const pkg = JSON.parse(await read(join(repo, "package.json")));
  assert.ok(pkg.workspaces.includes("apps/web"));
  assert.equal(pkg.scripts["web:typecheck"], "npm run typecheck --workspace @deskal/web");
  assert.equal(pkg.scripts["web:test"], "npm run test --workspace @deskal/web");
  assert.equal(pkg.scripts["web:build"], "npm run build --workspace @deskal/web");
});
