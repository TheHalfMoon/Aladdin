import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { LOCAL_ONLY_TOOL_NAMES, OAUTH_SCOPE_TOOL_MATRIX } from "./oauth_authorization.js";
import { CANONICAL_TOOL_NAMES } from "./server.js";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");

interface Capability {
  capability: string;
  operation: string;
  status: string;
  mcp_tool?: string;
  owner_grain?: string;
  reason?: string;
}

interface Workflow {
  id: string;
  status: string;
  via?: string[];
  owner_grain?: string;
  reason?: string;
  note?: string;
}

const inventory = JSON.parse(
  readFileSync(join(repo, "docs", "p16", "capability_parity_inventory.json"), "utf8")
) as { schema: string; statuses: string[]; capabilities: Capability[]; workflows: Workflow[] };

const STATUSES = ["implemented_exposed", "implemented_hidden", "missing", "intentionally_denied"];
const P16_GRAINS = ["SG-000060", "SG-000061", "SG-000062", "SG-000063", "SG-000064"];

function rustSources(): string {
  const parts: string[] = [];
  const visit = (dir: string): void => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) {
        visit(path);
      } else if (entry.name.endsWith(".rs")) {
        parts.push(readFileSync(path, "utf8"));
      }
    }
  };
  visit(join(repo, "crates"));
  return parts.join("\n");
}

test("inventory statuses are exactly the four P16 classifications", () => {
  assert.equal(inventory.schema, "cotra-parity-inventory/1");
  assert.deepEqual(inventory.statuses, STATUSES);
  for (const entry of [...inventory.capabilities, ...inventory.workflows]) {
    assert.ok(STATUSES.includes(entry.status), JSON.stringify(entry));
  }
});

test("every capability shape and workflow is classified exactly once", () => {
  const shapes = inventory.capabilities.map((c) => `${c.capability}/${c.operation}`);
  assert.equal(new Set(shapes).size, shapes.length, "duplicate capability shape");
  const workflows = inventory.workflows.map((w) => w.id);
  assert.equal(new Set(workflows).size, workflows.length, "duplicate workflow");
});

test("exposed entries match the authoritative MCP catalog exactly", () => {
  const exposed = inventory.capabilities.filter((c) => c.status === "implemented_exposed");
  assert.deepEqual(exposed.map((c) => c.mcp_tool).sort(), [...CANONICAL_TOOL_NAMES].sort());
  assert.deepEqual(
    Object.keys(OAUTH_SCOPE_TOOL_MATRIX).sort(),
    CANONICAL_TOOL_NAMES.filter((tool) => !LOCAL_ONLY_TOOL_NAMES.includes(tool)).sort()
  );
  for (const workflow of inventory.workflows.filter((w) => w.status === "implemented_exposed")) {
    assert.ok(workflow.via !== undefined && workflow.via.length > 0, workflow.id);
    for (const tool of workflow.via ?? []) {
      assert.ok(CANONICAL_TOOL_NAMES.includes(tool), `${workflow.id} names unknown tool ${tool}`);
    }
  }
  // Every exposed tool's capability shape is the one its source sends.
  const sources = readdirSync(here.replace(/dist$/, "src"))
    .filter((name) => name.endsWith(".ts") && !name.endsWith(".test.ts"))
    .map((name) => readFileSync(join(here.replace(/dist$/, "src"), name), "utf8"))
    .join("\n");
  for (const entry of exposed) {
    assert.ok(sources.includes(`capability: "${entry.capability}"`), `${entry.mcp_tool} must send ${entry.capability}`);
  }
});

test("hidden and denied shapes are real cotrad shapes, and hidden ones are not exposed", () => {
  const rust = rustSources();
  for (const entry of inventory.capabilities.filter((c) => c.status !== "implemented_exposed")) {
    const tuple = rust.includes(`("${entry.capability}", "${entry.operation}")`);
    const conditional = rust
      .split(/\r?\n/)
      .some((line) => line.includes(`"${entry.capability}"`) && line.includes(`== "${entry.operation}"`));
    assert.ok(tuple || conditional, `${entry.capability}/${entry.operation} must exist in cotrad sources`);
    assert.equal(entry.mcp_tool, undefined, `${entry.capability}/${entry.operation} is not an MCP tool`);
  }
});

test("missing and hidden entries name an owning grain; denials name a security reason", () => {
  for (const entry of [...inventory.capabilities, ...inventory.workflows]) {
    if (entry.status === "implemented_hidden" || entry.status === "missing") {
      assert.ok(
        entry.owner_grain !== undefined && (P16_GRAINS.includes(entry.owner_grain) || entry.owner_grain === "outside_v0_2_target"),
        JSON.stringify(entry)
      );
    }
    if (entry.status === "intentionally_denied") {
      assert.ok(typeof entry.reason === "string" && entry.reason.length >= 20, JSON.stringify(entry));
    }
  }
});

test("the unsafe Desktop Commander shapes are recorded as security decisions, not gaps", () => {
  const denied = new Set(inventory.workflows.filter((w) => w.status === "intentionally_denied").map((w) => w.id));
  for (const id of [
    "shell_command_string",
    "powershell_or_cmd",
    "interactive_repl_sessions",
    "elevation_or_admin",
    "arbitrary_network_sockets",
    "browser_script_or_devtools",
    "agent_changes_own_policy"
  ]) {
    assert.ok(denied.has(id), id);
  }
  const deniedShapes = new Set(
    inventory.capabilities.filter((c) => c.status === "intentionally_denied").map((c) => c.capability)
  );
  for (const capability of ["workspace.trust.grant", "trust.revoke_emergency", "remote.lease.create", "remote.enrollment.authorize"]) {
    assert.ok(deniedShapes.has(capability), capability);
  }
});

test("the parity document is generated from the same inventory", () => {
  const doc = readFileSync(join(repo, "docs", "p16", "CAPABILITY_PARITY.md"), "utf8");
  for (const workflow of inventory.workflows) {
    assert.ok(doc.includes(`| \`${workflow.id}\` | ${workflow.status} |`), workflow.id);
  }
  for (const entry of inventory.capabilities) {
    assert.ok(doc.includes(`| \`${entry.capability}/${entry.operation}\` | ${entry.status} |`), `${entry.capability}/${entry.operation}`);
  }
});
