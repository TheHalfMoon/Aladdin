import assert from "node:assert/strict";
import test from "node:test";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { CANONICAL_TOOL_NAMES } from "./server.js";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");

interface Workflow {
  id: string;
  status: string;
  via?: string[];
}

interface Capability {
  capability: string;
  operation: string;
  status: string;
  mcp_tool?: string;
}

interface Inventory {
  schema: string;
  workflows: Workflow[];
  capabilities: Capability[];
}

interface Donor {
  id: string;
  repository: string;
  revision: string;
  license: string;
  license_path: string;
  source_paths: string[];
  reuse: string;
  runtime_imported: boolean;
}

interface Replacement {
  donor_shape: string;
  workflow: string;
  expected_status: string;
}

interface UiTarsClassification {
  subsystem: string;
  workflow: string;
  expected_status: string;
  disposition: string;
}

interface LocalAuthorityDenial {
  capability: string;
  operation: string;
}

interface ExitEvidence {
  schema: string;
  grain: string;
  canonical_base: string;
  authority_delta: string;
  donors: Donor[];
  desktop_commander_replacement: Replacement[];
  desktop_commander_denials: string[];
  ui_tars_classification: UiTarsClassification[];
  local_authority_denials: LocalAuthorityDenial[];
  qualification: {
    authoritative_mcp_builder: string;
    tool_contract: string;
    daemon_entrypoint: string;
    parity_inventory: string;
    tests: string[];
  };
  exit_invariants: string[];
}

function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(join(repo, path), "utf8")) as T;
}

function sorted(values: Iterable<string>): string[] {
  return [...values].sort();
}

function collectFiles(root: string, predicate: (path: string) => boolean): string[] {
  const files: string[] = [];
  const visit = (path: string): void => {
    for (const entry of readdirSync(path, { withFileTypes: true })) {
      const child = join(path, entry.name);
      if (entry.isDirectory()) {
        visit(child);
      } else if (predicate(child)) {
        files.push(child);
      }
    }
  };
  visit(root);
  return files;
}

const evidence = readJson<ExitEvidence>("docs/p16/sg000066_exit_evidence.json");
const inventory = readJson<Inventory>("docs/p16/capability_parity_inventory.json");
const workflows = new Map(inventory.workflows.map((entry) => [entry.id, entry]));
const capabilities = new Map(
  inventory.capabilities.map((entry) => [`${entry.capability}/${entry.operation}`, entry])
);

const REQUIRED_REPLACEMENT_WORKFLOWS = [
  "create_directory",
  "delete_file",
  "edit_block",
  "file_info",
  "list_directory",
  "list_processes",
  "move_or_rename",
  "read_file",
  "read_file_range",
  "read_multiple_files",
  "run_bounded_program",
  "run_build_and_test_tools",
  "search_file_content",
  "search_file_names",
  "write_file"
];

const REQUIRED_DESKTOP_COMMANDER_DENIALS = [
  "agent_changes_own_policy",
  "arbitrary_network_sockets",
  "background_long_running_processes",
  "browser_script_or_devtools",
  "elevation_or_admin",
  "install_software_or_services",
  "interactive_repl_sessions",
  "kill_arbitrary_process",
  "powershell_or_cmd",
  "read_outside_workspaces",
  "shell_command_string"
];

const REQUIRED_UI_TARS_CLASSIFICATIONS: Record<string, { status: string; disposition: string }> = {
  browser_structured: { status: "missing", disposition: "defer_to_governed_successor" },
  desktop_act: { status: "missing", disposition: "defer_to_governed_successor" },
  screenshots: { status: "missing", disposition: "defer_to_governed_successor" },
  browser_script_or_devtools: { status: "intentionally_denied", disposition: "deny" },
  arbitrary_network_sockets: { status: "intentionally_denied", disposition: "deny" },
  agent_changes_own_policy: { status: "intentionally_denied", disposition: "deny" }
};

const REQUIRED_LOCAL_AUTHORITY_DENIALS = [
  "executable.registry.add/add",
  "remote.enrollment.authorize/authorize",
  "remote.lease.create/create",
  "trust.revoke_emergency/revoke",
  "workspace.trust.grant/grant"
];

test("SG-000066 exit evidence is pinned and authority-neutral", () => {
  assert.equal(evidence.schema, "quntal-p16-exit-evidence/1");
  assert.equal(evidence.grain, "SG-000066");
  assert.match(evidence.canonical_base, /^[0-9a-f]{40}$/);
  assert.equal(evidence.canonical_base, "e770a6913d11c92d6855943f6e47d306a5c42fca");
  assert.equal(evidence.authority_delta, "none");
  assert.ok(evidence.exit_invariants.length >= 6);
});

test("Desktop Commander and UI-TARS donors are exact reference-only pins", () => {
  assert.deepEqual(sorted(evidence.donors.map((entry) => entry.id)), ["desktop_commander", "ui_tars"]);

  const desktopCommander = evidence.donors.find((entry) => entry.id === "desktop_commander");
  assert.ok(desktopCommander);
  assert.equal(desktopCommander.repository, "wonderwhy-er/DesktopCommanderMCP");
  assert.equal(desktopCommander.revision, "c774c3b505de990219637ecdc9a830c8772fae9d");
  assert.equal(desktopCommander.license, "MIT");

  const uiTars = evidence.donors.find((entry) => entry.id === "ui_tars");
  assert.ok(uiTars);
  assert.equal(uiTars.repository, "bytedance/UI-TARS-desktop");
  assert.equal(uiTars.revision, "2ff41a9e515828c5bd5b276e493d73aa0bdf4a3a");
  assert.equal(uiTars.license, "Apache-2.0");

  for (const donor of evidence.donors) {
    assert.match(donor.revision, /^[0-9a-f]{40}$/);
    assert.equal(donor.reuse, "reference_only");
    assert.equal(donor.runtime_imported, false);
    assert.ok(donor.license_path.length > 0);
    assert.ok(donor.source_paths.length > 0);
  }
});

test("the Desktop Commander replacement matrix cannot silently shrink or widen", () => {
  const actual = evidence.desktop_commander_replacement.map((entry) => entry.workflow);
  assert.deepEqual(sorted(actual), REQUIRED_REPLACEMENT_WORKFLOWS);
  assert.equal(new Set(actual).size, actual.length, "duplicate replacement workflow");

  for (const replacement of evidence.desktop_commander_replacement) {
    const workflow = workflows.get(replacement.workflow);
    assert.ok(workflow, `unknown replacement workflow ${replacement.workflow}`);
    assert.equal(workflow.status, replacement.expected_status, replacement.workflow);
    assert.equal(workflow.status, "implemented_exposed", replacement.workflow);
    assert.ok(workflow.via && workflow.via.length > 0, replacement.workflow);
    for (const tool of workflow.via ?? []) {
      assert.ok(CANONICAL_TOOL_NAMES.includes(tool), `${replacement.workflow} names non-canonical tool ${tool}`);
    }
  }
});

test("Desktop Commander high-authority shapes remain explicit denials", () => {
  assert.deepEqual(sorted(evidence.desktop_commander_denials), REQUIRED_DESKTOP_COMMANDER_DENIALS);
  for (const id of evidence.desktop_commander_denials) {
    const workflow = workflows.get(id);
    assert.ok(workflow, `unknown denial workflow ${id}`);
    assert.equal(workflow.status, "intentionally_denied", id);
    assert.equal(workflow.via, undefined, `${id} must not expose an MCP route`);
  }
});

test("UI-TARS authority is either deferred or denied, never imported by SG-000066", () => {
  const actual = new Map(evidence.ui_tars_classification.map((entry) => [entry.workflow, entry]));
  assert.deepEqual(sorted(actual.keys()), sorted(Object.keys(REQUIRED_UI_TARS_CLASSIFICATIONS)));

  for (const [workflowId, required] of Object.entries(REQUIRED_UI_TARS_CLASSIFICATIONS)) {
    const classification = actual.get(workflowId);
    assert.ok(classification, workflowId);
    assert.equal(classification.expected_status, required.status, workflowId);
    assert.equal(classification.disposition, required.disposition, workflowId);

    const workflow = workflows.get(workflowId);
    assert.ok(workflow, `unknown UI-TARS-classified workflow ${workflowId}`);
    assert.equal(workflow.status, required.status, workflowId);
    assert.equal(workflow.via, undefined, `${workflowId} must not expose an MCP route`);
  }
});

test("local authority remains unavailable to agents", () => {
  const actual = evidence.local_authority_denials.map((entry) => `${entry.capability}/${entry.operation}`);
  assert.deepEqual(sorted(actual), REQUIRED_LOCAL_AUTHORITY_DENIALS);

  for (const key of actual) {
    const capability = capabilities.get(key);
    assert.ok(capability, `unknown local-authority shape ${key}`);
    assert.equal(capability.status, "intentionally_denied", key);
    assert.equal(capability.mcp_tool, undefined, `${key} must not have an MCP tool`);
  }
});

test("every qualification artifact in the exit manifest exists", () => {
  const paths = [
    evidence.qualification.authoritative_mcp_builder,
    evidence.qualification.tool_contract,
    evidence.qualification.daemon_entrypoint,
    evidence.qualification.parity_inventory,
    ...evidence.qualification.tests
  ];
  assert.equal(new Set(paths).size, paths.length, "duplicate qualification path");
  for (const path of paths) {
    const absolute = join(repo, path);
    assert.ok(existsSync(absolute), `missing qualification artifact ${path}`);
    assert.ok(statSync(absolute).isFile(), `qualification artifact is not a file ${path}`);
  }
});

test("SG-000066 imports no Desktop Commander or UI-TARS runtime", () => {
  const runtimeFiles = [
    join(repo, "package.json"),
    join(repo, "package-lock.json"),
    join(repo, "Cargo.toml"),
    join(repo, "Cargo.lock"),
    join(repo, "apps", "quntal-mcp", "package.json"),
    ...collectFiles(join(repo, "apps", "quntal-mcp", "src"), (path) => path.endsWith(".ts") && !path.endsWith(".test.ts")),
    ...collectFiles(join(repo, "crates"), (path) => path.endsWith(".rs"))
  ];
  const forbiddenRuntimeMarkers = ["desktopcommandermcp", "@ui-tars", "ui-tars", "operator-browser"];

  for (const path of runtimeFiles) {
    const content = readFileSync(path, "utf8").toLowerCase();
    for (const marker of forbiddenRuntimeMarkers) {
      assert.equal(content.includes(marker), false, `${path} imports or embeds donor runtime marker ${marker}`);
    }
  }
});

test("future P18 grains remain outside the active SpecGrain registry", () => {
  const specs = new Set(readdirSync(join(repo, ".specgrain", "specs")));
  for (let n = 73; n <= 86; n += 1) {
    const id = `SG-${String(n).padStart(6, "0")}.json`;
    assert.equal(specs.has(id), false, `${id} must not be activated during SG-000066`);
  }
});
