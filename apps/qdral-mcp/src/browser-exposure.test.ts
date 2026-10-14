import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { attachProfile, buildRequest } from "./kernel.js";
import { UNMAPPED_REMOTE_PROFILES } from "./oauth_authorization.js";
import { localTransportContext } from "./server.js";
import {
  DESKTOP_STRUCTURED_TOOLS,
  LIVE_BROWSER_TOOLS,
  LOCAL_TOOL_SURFACE_PROFILES,
  REMOTE_PROVIDER_PROFILES,
  SURFACE_PROFILES,
  TOOL_CONTRACT,
  TOOL_SURFACE_PROFILE_ENV,
  localProfileFromConfig,
  profileTools
} from "./tool_contract.js";

// SG-000108 decision 6: the live browser exposure tables agree across the
// MCP contract, qdrald's profile gate and the browser host's exposure list.

const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function rustStrings(path: string, constant: string): string[] {
  const text = readFileSync(join(repo, path), "utf8");
  const start = text.indexOf(`pub const ${constant}:`);
  assert.ok(start >= 0, `${constant} in ${path}`);
  const end = text.indexOf(";", start);
  const body = text.slice(text.indexOf("=", start) + 1, end);
  return [...body.matchAll(/"([^"]*)"/g)].map((match) => match[1] ?? "");
}

test("the live browser tools agree across the contract, qdrald's gate and the host", () => {
  const host = rustStrings("crates/qdral-browser-host/src/exposure.rs", "LIVE_BROWSER_MCP_TOOLS");
  assert.deepEqual([...host].sort(), [...LIVE_BROWSER_TOOLS].sort());
  // qdrald lists shapes as (capability, operation) pairs.
  const flat = rustStrings("crates/qdrald/src/profile_gate.rs", "LIVE_BROWSER_SHAPES");
  assert.equal(flat.length % 2, 0);
  const shapes: string[] = [];
  for (let index = 0; index < flat.length; index += 2) {
    shapes.push(`${flat[index]}/${flat[index + 1]}`);
  }
  const contractShapes = LIVE_BROWSER_TOOLS.map((tool) => {
    const entry = TOOL_CONTRACT[tool];
    assert.ok(entry !== undefined, `${tool} has a contract entry`);
    return `${entry.capability}/${entry.operation}`;
  });
  assert.deepEqual(shapes.sort(), contractShapes.sort());
  // Every live tool is local-only.
  for (const tool of LIVE_BROWSER_TOOLS) {
    assert.equal(TOOL_CONTRACT[tool]?.remote, "local_only", tool);
  }
  // Until slice 3b nothing is live.
  assert.deepEqual([...LIVE_BROWSER_TOOLS], []);
});

test("desktop_structured is an explicit list that adding a contract tool cannot widen", () => {
  const desktop = new Set(DESKTOP_STRUCTURED_TOOLS);
  assert.equal(desktop.size, DESKTOP_STRUCTURED_TOOLS.length, "no duplicates");
  const nonLive = Object.keys(TOOL_CONTRACT).filter((tool) => !LIVE_BROWSER_TOOLS.includes(tool));
  // Every non-live contract tool is listed, explicitly, and nothing else.
  assert.deepEqual([...desktop].sort(), nonLive.sort());
  for (const tool of LIVE_BROWSER_TOOLS) {
    assert.ok(!desktop.has(tool), tool);
  }
  for (const tool of SURFACE_PROFILES.core) {
    assert.ok(desktop.has(tool), `core ${tool} is in desktop_structured`);
  }
  assert.ok(Object.isFrozen(DESKTOP_STRUCTURED_TOOLS));
});

test("browser_structured is desktop_structured plus the live browser tools", () => {
  assert.deepEqual(
    [...profileTools("browser_structured")].sort(),
    [...DESKTOP_STRUCTURED_TOOLS, ...LIVE_BROWSER_TOOLS].sort()
  );
});

test("only local configuration selects the profile, and only a local one", () => {
  assert.equal(localProfileFromConfig(undefined), "desktop_structured");
  assert.equal(localProfileFromConfig(""), "desktop_structured");
  assert.equal(localProfileFromConfig("desktop_structured"), "desktop_structured");
  assert.equal(localProfileFromConfig("browser_structured"), "browser_structured");
  for (const value of ["core", "developer", "coordinate_fallback", "Browser_Structured", "__proto__", " browser_structured"]) {
    assert.throws(() => localProfileFromConfig(value), /TOOL_SURFACE_DENIED/, value);
  }
  assert.deepEqual([...LOCAL_TOOL_SURFACE_PROFILES], ["desktop_structured", "browser_structured"]);
  assert.equal(localTransportContext({}).toolSurfaceProfile, "desktop_structured");
  assert.equal(
    localTransportContext({ [TOOL_SURFACE_PROFILE_ENV]: "browser_structured" }).toolSurfaceProfile,
    "browser_structured"
  );
  assert.equal(localTransportContext({ [TOOL_SURFACE_PROFILE_ENV]: "browser_structured" }).transportKind, "stdio");
  assert.throws(() => localTransportContext({ [TOOL_SURFACE_PROFILE_ENV]: "core" }), /TOOL_SURFACE_DENIED/);
});

test("no remote path maps browser_structured", () => {
  assert.ok((UNMAPPED_REMOTE_PROFILES as readonly string[]).includes("browser_structured"));
  for (const profile of Object.values(REMOTE_PROVIDER_PROFILES)) {
    assert.equal(profile, "core");
  }
});

test("the kernel stamps the local profile on a request, and nothing when unset", () => {
  const base = () =>
    buildRequest({ sessionId: "s", workspaceId: "w", capability: "fs.read", operation: "read" });
  assert.equal("profile" in base(), false);
  assert.equal(attachProfile(base(), null).profile, undefined);
  assert.equal(attachProfile(base(), "browser_structured").profile, "browser_structured");
  const line = JSON.parse(JSON.stringify(attachProfile(base(), "desktop_structured"))) as Record<string, unknown>;
  assert.equal(line.profile, "desktop_structured");
});
