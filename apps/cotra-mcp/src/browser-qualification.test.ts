import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { CANONICAL_TOOL_NAMES } from "./server.js";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");

const BROWSER_SHAPES = [
  "browser.profile/status",
  "browser.destination/validate",
  "browser.page/open",
  "browser.navigation/preview",
  "browser.navigation/navigate",
  "browser.snapshot/observe",
  "browser.dom/click",
  "browser.dom/fill",
  "browser.download/preview",
  "browser.download/download",
  "browser.upload/preview",
  "browser.upload/submit"
];

function nonTestRust(dir: string): Array<[string, string]> {
  return readdirSync(dir)
    .filter((name) => name.endsWith(".rs") && !name.includes("test"))
    .map((name) => [name, readFileSync(join(dir, name), "utf8")] as [string, string]);
}

test("no MCP tool claims browser capability", () => {
  for (const tool of CANONICAL_TOOL_NAMES) {
    assert.ok(!tool.startsWith("browser"), tool);
  }
});

test("the structured browser layer launches, attaches, and connects to nothing outside tests", () => {
  const sources = [
    ...nonTestRust(join(repo, "crates", "cotra-provider-browser", "src")),
    ["cotrad/browser.rs", readFileSync(join(repo, "crates", "cotrad", "src", "browser.rs"), "utf8")] as [string, string]
  ];
  for (const [name, text] of sources) {
    const production = text.split("#[cfg(test)]")[0] ?? text;
    for (const forbidden of ["Command::new", "CreateProcess", "WebSocket", "TcpStream", "remote-debugging", "msedge", "chrome.exe", "UdpSocket"]) {
      assert.ok(!production.includes(forbidden), `${name} must not reference ${forbidden}`);
    }
  }
});

test("every structured browser shape is recorded as not exposed and denied in the inventory", () => {
  const doc = readFileSync(join(repo, "docs", "p16", "BROWSER_QUALIFICATION.md"), "utf8");
  const inventory = JSON.parse(readFileSync(join(repo, "docs", "p16", "capability_parity_inventory.json"), "utf8")) as {
    capabilities: Array<{ capability: string; operation: string; status: string; reason?: string }>;
    workflows: Array<{ id: string; status: string; owner_grain?: string }>;
  };
  for (const shape of BROWSER_SHAPES) {
    assert.ok(doc.includes(`| \`${shape}\` | model-only`), shape);
    const [capability, operation] = shape.split("/");
    const entry = inventory.capabilities.find((c) => c.capability === capability && c.operation === operation);
    assert.equal(entry?.status, "intentionally_denied", shape);
    assert.ok((entry?.reason ?? "").includes("no live browser engine"), shape);
  }
  const workflow = inventory.workflows.find((w) => w.id === "browser_structured");
  assert.equal(workflow?.status, "missing");
  assert.equal(workflow?.owner_grain, "outside_v0_2_target");
});
