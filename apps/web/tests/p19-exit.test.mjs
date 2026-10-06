import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const here = dirname(fileURLToPath(import.meta.url));
const web = join(here, "..");
const repo = join(web, "..", "..");

const read = (path) => readFileSync(join(repo, path), "utf8");
const json = (path) => JSON.parse(read(path));
const sha256 = (path) => createHash("sha256").update(readFileSync(join(repo, path))).digest("hex");

const matrix = json("docs/p19/sg000091_exit_matrix.json");

test("SG-000091 exit matrix is complete and authority-neutral", () => {
  assert.equal(matrix.schema, "deskal-p19-exit-matrix/1");
  assert.equal(matrix.grain, "SG-000091");
  assert.equal(matrix.program, "DESKAL-P19");
  assert.equal(matrix.activation_merge, "064124ce64650de29e69f04d42dbd4ec0a89dbfa");
  assert.equal(matrix.activation_post_merge_ci, "37538199470");
  assert.equal(matrix.authority_delta, "none");
  assert.deepEqual(matrix.classification_vocabulary, ["PROVEN", "OUT_OF_SCOPE", "UNVERIFIED"]);
  assert.ok(matrix.rows.length >= 18);
  assert.equal(matrix.rows.some((row) => row.classification === "UNVERIFIED"), false);
  for (const row of matrix.rows.filter((entry) => entry.required)) {
    assert.equal(row.classification, "PROVEN", row.id);
    assert.ok(row.evidence.length > 0, row.id);
  }
  assert.ok(matrix.exit_invariants.length >= 8);
});

test("P19 predecessor grains stay CLOSED with exact canonical evidence", () => {
  const expected = new Map(matrix.prior_grains.map((entry) => [entry.id, entry]));
  assert.deepEqual([...expected.keys()], ["SG-000087", "SG-000088", "SG-000089", "SG-000090"]);

  for (const [id, pinned] of expected) {
    const spec = json(`.specgrain/specs/${id}.json`);
    assert.equal(spec.state, "CLOSED", id);
    assert.equal(spec.qualified_head, pinned.qualified_head, id);
    assert.equal(spec.implementation_merge, pinned.merge, id);
    assert.equal(spec.post_merge_ci, pinned.post_merge_ci, id);
    assert.equal(spec.canonical_evidence.unresolved_review_threads, 0, id);
  }
});

test("SG-000091 is closed with exact P19 exit evidence", () => {
  const spec = json(".specgrain/specs/SG-000091.json");
  assert.equal(spec.state, "CLOSED");
  assert.equal(spec.program, "DESKAL-P19");
  assert.equal(spec.qualified_head, "eab70d3a14f4fbfcf01ca4f87c69c1df8f6cd733");
  assert.equal(spec.implementation_merge, "e674d4740ab15f39ec524248f8e512e64cb70d06");
  assert.equal(spec.post_merge_ci, "37541526231");
  assert.equal(spec.canonical_evidence.implementation_pr, 238);
  assert.equal(spec.canonical_evidence.unresolved_review_threads, 0);
  assert.equal(spec.canonical_evidence.program_exit, "DESKAL-P19 EXITED");
});

test("brand, static export, and zero-client-runtime locks remain exact", () => {
  assert.equal(
    sha256("apps/web/public/brand/deskal-mark-mask.svg"),
    "150ae5ccd9fdb6b25a8f0421a64acc2d0bf1615a32c2b33aea92e77700706fce"
  );

  const config = read("apps/web/next.config.ts");
  const page = read("apps/web/app/page.tsx");
  assert.match(config, /output:\s*"export"/);
  for (const marker of ['"use client"', "onClick=", "onKeyDown=", "useState(", "useEffect(", "fetch(", "XMLHttpRequest", "WebSocket("]) {
    assert.equal(page.includes(marker), false, marker);
  }
});

test("forbidden hosted-service and telemetry markers remain absent", () => {
  const sources = [
    read("apps/web/app/page.tsx"),
    read("apps/web/app/layout.tsx"),
    read("apps/web/app/globals.css"),
    read("apps/web/next.config.ts")
  ].join("\n").toLowerCase();

  for (const marker of [
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
    "sendbeacon",
    "document.cookie"
  ]) {
    assert.equal(sources.includes(marker), false, marker);
  }
});

test("Pages deployment contract stays workflow-built, HTTPS-oriented, secret-free, and custom-domain free", () => {
  const workflow = read(".github/workflows/pages.yml");
  assert.doesNotMatch(workflow, /secrets\./);
  assert.doesNotMatch(workflow, /cname:\s*[^\s]/i);
  assert.match(workflow, /test ! -e apps\/web\/public\/CNAME/);
  assert.match(workflow, /https:\/\/thehalfmoon\.github\.io\/Deskal/);

  assert.equal(matrix.live_snapshot.website.url, "https://thehalfmoon.github.io/Deskal/");
  assert.equal(matrix.live_snapshot.website.http_status, 200);
  assert.equal(matrix.live_snapshot.pages.build_type, "workflow");
  assert.equal(matrix.live_snapshot.pages.https_enforced, true);
  assert.equal(matrix.live_snapshot.pages.cname, null);
  assert.deepEqual(matrix.live_snapshot.pages.source, { branch: "main", path: "/" });
});

test("canonical repository metadata matches the live exit snapshot exactly", () => {
  const metadata = json("docs/canonical/REPOSITORY_METADATA.json");
  assert.equal(metadata.description, matrix.live_snapshot.repository.description);
  assert.equal(metadata.homepage, matrix.live_snapshot.repository.homepage);
  assert.deepEqual(metadata.topics, matrix.live_snapshot.repository.topics);
});

test("P19 is exited and P20 activates only SG-000092", () => {
  const p19 = read("docs/canonical/DESKAL_POST_P18_LAUNCH_PLAN.md");
  const p20 = read("docs/canonical/DESKAL_P20_UNIVERSAL_AGENT_RUNTIME_PLAN.md");
  const sg91 = json(".specgrain/specs/SG-000091.json");
  const sg92 = json(".specgrain/specs/SG-000092.json");

  assert.match(p19, /Status: EXITED CANONICAL/);
  assert.match(p19, /SG-000091 is CLOSED canonical/);
  assert.equal(sg91.state, "CLOSED");

  assert.match(p20, /Activation grain: SG-000092/);
  assert.equal(sg92.id, "SG-000092");
  assert.equal(sg92.program, "DESKAL-P20");
  assert.equal(sg92.state, "GRAIN");
  assert.equal(sg92.base, "835200a90fd47425ff3ad40b5b8815dc8ad7871c");
  assert.deepEqual(sg92.dependencies, ["SG-000091"]);

  for (let n = 93; n <= 107; n += 1) {
    const id = `SG-${String(n).padStart(6, "0")}.json`;
    assert.throws(() => read(`.specgrain/specs/${id}`), undefined, id);
  }
});
