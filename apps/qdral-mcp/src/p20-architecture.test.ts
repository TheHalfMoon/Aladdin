import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");

interface Donor {
  id: string;
  repository: string;
  revision: string;
  license: string;
  owner_grains: string[];
  runtime_imported: boolean;
  permission_gate: string;
}

interface ArchitectureFreeze {
  schema: string;
  grain: string;
  program: string;
  canonical_base: string;
  activation: {
    pr: number;
    base: string;
    qualified_head: string;
    pre_merge_ci: string;
    review_gates: string;
    merge_sha: string;
    post_merge_ci: string;
    pages_run: string;
    jev: { expected_hunks: number; reviewed_hunks: number; findings: number; blocking_findings: number };
    ocr: { reviewable_files: number; excluded_files: number; manual_exclusions_reviewed: number; blocking_findings: number };
    unresolved_review_threads: number;
  };
  authority_delta: string;
  caller_facing_authority: string;
  authority_modes: Array<{ id: string; default: boolean; grantor: string; privilege: string; self_grant: boolean }>;
  privilege_denials: string[];
  execution_preference: string[];
  result_states: string[];
  private_hosts: Array<{ id: string; donor_id: string; owner_grains: string[]; caller_facing: boolean }>;
  donors: Donor[];
  grain_sequence: Array<{ id: string; role: string }>;
  import_state: {
    donor_files_imported: string[];
    runtime_dependencies_added: string[];
    mcp_tools_added: string[];
    privileged_services_added: string[];
    releases_or_tags_mutated: boolean;
  };
  invariants: string[];
}

const read = (path: string): string => readFileSync(join(repo, path), "utf8");
const readJson = <T>(path: string): T => JSON.parse(read(path)) as T;
const freeze = readJson<ArchitectureFreeze>("docs/p20/sg000092_architecture_freeze.json");

const EXPECTED_DONORS: Record<string, { repository: string; revision: string; licenseNeedle: string }> = {
  open_computer_use: {
    repository: "opensymph/open-computer-use",
    revision: "5b433b98019c18201a15d11e8c3cb0010879a3d8",
    licenseNeedle: "MIT"
  },
  desktop_commander: {
    repository: "wonderwhy-er/DesktopCommanderMCP",
    revision: "bc1e944e30302e0022d49f418d55563dd74a162c",
    licenseNeedle: "MIT"
  },
  firecrawl: {
    repository: "firecrawl/firecrawl",
    revision: "7120d16926544483513782b5393ae9850691d52a",
    licenseNeedle: "AGPL-3.0"
  },
  agent_reach: {
    repository: "Panniantong/Agent-Reach",
    revision: "a19a171fa980a0785849596492e0af4db800c82f",
    licenseNeedle: "MIT"
  },
  ui_tars: {
    repository: "bytedance/UI-TARS-desktop",
    revision: "2ff41a9e515828c5bd5b276e493d73aa0bdf4a3a",
    licenseNeedle: "Apache-2.0"
  },
  gsudo: {
    repository: "gerardog/gsudo",
    revision: "a4eda9b263d72a3e174f73b6b6b3ecfc5bc6656b",
    licenseNeedle: "MIT"
  }
};

test("SG-000092 activation evidence is exact and authority-neutral", () => {
  assert.equal(freeze.schema, "deskal-p20-architecture-freeze/1");
  assert.equal(freeze.grain, "SG-000092");
  assert.equal(freeze.program, "DESKAL-P20");
  assert.equal(freeze.canonical_base, "ed82cb4c976d4169fd4f81327be62d37dd5ea0a7");
  assert.equal(freeze.activation.pr, 241);
  assert.equal(freeze.activation.base, "835200a90fd47425ff3ad40b5b8815dc8ad7871c");
  assert.equal(freeze.activation.qualified_head, "771eaaeba234b1f533fd849440e133ff03ef62ae");
  assert.equal(freeze.activation.pre_merge_ci, "37547623637");
  assert.equal(freeze.activation.review_gates, "37547623682");
  assert.equal(freeze.activation.merge_sha, "ed82cb4c976d4169fd4f81327be62d37dd5ea0a7");
  assert.equal(freeze.activation.post_merge_ci, "37548262164");
  assert.equal(freeze.activation.pages_run, "37548261888");
  assert.deepEqual(freeze.activation.jev, { expected_hunks: 8, reviewed_hunks: 8, findings: 0, blocking_findings: 0 });
  assert.deepEqual(freeze.activation.ocr, { reviewable_files: 2, excluded_files: 4, manual_exclusions_reviewed: 4, blocking_findings: 0 });
  assert.equal(freeze.activation.unresolved_review_threads, 0);
  assert.equal(freeze.authority_delta, "none");
});

test("all six donor pins are exact, import-neutral, and obligation-bearing", () => {
  assert.equal(freeze.donors.length, 6);
  assert.deepEqual(freeze.donors.map((entry) => entry.id).sort(), Object.keys(EXPECTED_DONORS).sort());

  const ledger = read("docs/canonical/DESKAL_P20_DONOR_LEDGER.md");
  for (const donor of freeze.donors) {
    const expected = EXPECTED_DONORS[donor.id];
    assert.ok(expected, donor.id);
    assert.equal(donor.repository, expected.repository, donor.id);
    assert.equal(donor.revision, expected.revision, donor.id);
    assert.match(donor.revision, /^[0-9a-f]{40}$/, donor.id);
    assert.ok(donor.license.includes(expected.licenseNeedle), donor.id);
    assert.equal(donor.runtime_imported, false, donor.id);
    assert.ok(donor.permission_gate.length > 20, donor.id);
    assert.ok(donor.owner_grains.length > 0, donor.id);
    assert.ok(ledger.includes(donor.repository), donor.id);
    assert.ok(ledger.includes(donor.revision), donor.id);
  }

  const firecrawl = freeze.donors.find((entry) => entry.id === "firecrawl");
  assert.ok(firecrawl);
  assert.match(firecrawl.permission_gate, /direct-permission evidence/i);
  assert.match(firecrawl.permission_gate, /third-party dependencies/i);
});

test("authority modes are exact and none can self-grant", () => {
  assert.deepEqual(
    freeze.authority_modes.map((entry) => entry.id),
    ["safe", "full_user", "full_admin", "persistent_admin", "remote_full_control"]
  );
  assert.equal(freeze.authority_modes.filter((entry) => entry.default).length, 1);
  assert.equal(freeze.authority_modes[0]?.id, "safe");
  for (const mode of freeze.authority_modes) {
    assert.equal(mode.self_grant, false, mode.id);
    assert.ok(mode.grantor.length > 0, mode.id);
    assert.ok(mode.privilege.length > 0, mode.id);
  }

  const fullAdmin = freeze.authority_modes.find((entry) => entry.id === "full_admin");
  assert.ok(fullAdmin);
  assert.match(fullAdmin.grantor, /Windows elevation/);
  assert.match(fullAdmin.privilege, /OS-issued elevated administrator token/);

  for (const denied of ["UAC bypass", "Windows Hello bypass", "secure-desktop automation", "model self-grant", "remote self-grant"]) {
    assert.ok(freeze.privilege_denials.includes(denied), denied);
  }
});

test("P20 subsystem ownership and sequence have no gap", () => {
  const ids = freeze.grain_sequence.map((entry) => entry.id);
  assert.deepEqual(ids, Array.from({ length: 16 }, (_, index) => `SG-${String(index + 92).padStart(6, "0")}`));
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(freeze.private_hosts.length, 6);
  assert.equal(freeze.private_hosts.every((host) => host.caller_facing === false), true);
  assert.deepEqual(
    freeze.private_hosts.map((host) => host.donor_id).sort(),
    Object.keys(EXPECTED_DONORS).sort()
  );

  const plan = read("docs/canonical/DESKAL_P20_UNIVERSAL_AGENT_RUNTIME_PLAN.md");
  for (const entry of freeze.grain_sequence) {
    assert.ok(plan.includes(entry.id), entry.id);
    assert.ok(entry.role.length > 0, entry.id);
  }
  assert.equal(plan.includes("SG-000108"), false);
});

test("SG-000092 is closed and SG-000093 is the sole active grain", () => {
  const specDir = join(repo, ".specgrain", "specs");
  const specs = readdirSync(specDir).filter((name) => /^SG-\d{6}\.json$/.test(name));
  const open: string[] = [];
  for (const name of specs) {
    const spec = JSON.parse(readFileSync(join(specDir, name), "utf8")) as { id: string; state: string };
    if (spec.state !== "CLOSED") {
      open.push(spec.id);
    }
  }
  assert.deepEqual(open, ["SG-000093"]);

  const sg92 = readJson<{ state: string; qualified_head: string; implementation_merge: string; post_merge_ci: string; canonical_evidence: { implementation_pr: number; unresolved_review_threads: number } }>(".specgrain/specs/SG-000092.json");
  assert.equal(sg92.state, "CLOSED");
  assert.equal(sg92.qualified_head, "35cadb63bbf4373cff36f8fce73bc5f57552576c");
  assert.equal(sg92.implementation_merge, "03d28ac2e4c96d855dfd23df6257cc31a87b1a7a");
  assert.equal(sg92.post_merge_ci, "37551232655");
  assert.equal(sg92.canonical_evidence.implementation_pr, 242);
  assert.equal(sg92.canonical_evidence.unresolved_review_threads, 0);

  const sg93 = readJson<{ id: string; program: string; state: string; base: string; dependencies: string[] }>(".specgrain/specs/SG-000093.json");
  assert.equal(sg93.id, "SG-000093");
  assert.equal(sg93.program, "DESKAL-P20");
  assert.equal(sg93.state, "GRAIN");
  assert.equal(sg93.base, "03d28ac2e4c96d855dfd23df6257cc31a87b1a7a");
  assert.deepEqual(sg93.dependencies, ["SG-000092"]);

  for (let n = 94; n <= 107; n += 1) {
    const id = `SG-${String(n).padStart(6, "0")}.json`;
    assert.equal(specs.includes(id), false, id);
  }
});

test("the frozen execution and result vocabularies stay exact", () => {
  assert.deepEqual(freeze.execution_preference, [
    "semantic_uia_or_native",
    "browser_dom_or_accessibility",
    "visual_grounding",
    "coordinate_or_raw_foreground_input"
  ]);
  assert.deepEqual(freeze.result_states, [
    "not_started",
    "dispatched",
    "completed",
    "cancelled",
    "outcome_unknown"
  ]);
  assert.ok(freeze.invariants.length >= 9);
});

test("SG-000092 imports no donor runtime, dependency, tool, or privileged service", () => {
  assert.deepEqual(freeze.import_state.donor_files_imported, []);
  assert.deepEqual(freeze.import_state.runtime_dependencies_added, []);
  assert.deepEqual(freeze.import_state.mcp_tools_added, []);
  assert.deepEqual(freeze.import_state.privileged_services_added, []);
  assert.equal(freeze.import_state.releases_or_tags_mutated, false);

  const manifests = [
    "package.json",
    "package-lock.json",
    "Cargo.toml",
    "Cargo.lock",
    "apps/qdral-mcp/package.json",
    "apps/qdral-relay/package.json"
  ].map(read).join("\n").toLowerCase();

  for (const marker of [
    "desktopcommandermcp",
    "open-computer-use",
    "firecrawl/firecrawl",
    "agent-reach",
    "@ui-tars",
    "ui-tars-desktop",
    "gerardog/gsudo"
  ]) {
    assert.equal(manifests.includes(marker), false, marker);
  }
});

test("canonical P20 documents match the machine-readable freeze", () => {
  const plan = read("docs/canonical/DESKAL_P20_UNIVERSAL_AGENT_RUNTIME_PLAN.md");
  const ledger = read("docs/canonical/DESKAL_P20_DONOR_LEDGER.md");
  const current = read("docs/canonical/CURRENT.md");
  const planning = read("docs/canonical/PLANNING_INDEX.md");

  assert.match(plan, /Activation grain: SG-000092/);
  assert.match(plan, /Planning base: `835200a90fd47425ff3ad40b5b8815dc8ad7871c`/);
  for (const title of ["Safe", "Full User", "Full Admin", "Persistent Admin", "Remote Full Control"]) {
    assert.ok(plan.includes(title), title);
  }
  assert.match(plan, /UAC bypass/);
  assert.match(plan, /Windows Hello bypass/);
  assert.match(plan, /No donor import or authority delta/);

  assert.match(ledger, /Owner grain: SG-000092/);
  assert.match(ledger, /No donor code merge proceeds unless all boxes are proven/);

  assert.match(current, /SG-000092 - P20 architecture, donor, and authority freeze - is CLOSED canonical/);
  assert.match(current, /SG-000093 - Full-control authority and profile model - is the sole active DESKAL-P20 grain/);
  assert.match(planning, /\.specgrain\/specs\/SG-000093\.json/);
  assert.match(planning, /DESKAL_P20_UNIVERSAL_AGENT_RUNTIME_PLAN\.md/);
  assert.match(planning, /DESKAL_P20_DONOR_LEDGER\.md/);
});
