import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { DESKTOP_KERNEL_SHAPES, desktopWindowTreeSchema } from "./desktop.js";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");
const repo = join(here, "..", "..", "..");

/** Tool sources; the SG-000065 contract declares metadata and forwards nothing. */
function toolSources(): string[] {
  return readdirSync(srcDir).filter(
    (name) => name.endsWith(".ts") && !name.endsWith(".test.ts") && name !== "tool_contract.ts"
  );
}

/**
 * SG-000063: exactly two live, read-only desktop shapes reach the kernel,
 * and only from desktop.ts. Actuation, capture, visual, coordinate, input,
 * focus, and termination shapes are never forwarded.
 */
test("only desktop.ts forwards UIA capabilities, and only the two read-only shapes", () => {
  const forwarded: string[] = [];
  for (const name of toolSources()) {
    const text = readFileSync(join(srcDir, name), "utf8");
    for (const match of text.matchAll(/capability:\s*["'](uia\.[a-z_.]+)["'],\s*operation:\s*["']([a-z_]+)["']/g)) {
      forwarded.push(`${name}:${match[1] ?? ""}/${match[2] ?? ""}`);
    }
    if (name !== "desktop.ts") {
      assert.doesNotMatch(text, /["']uia\./, `${name} must not name a UIA capability`);
    }
  }
  assert.deepEqual(forwarded.sort(), ["desktop.ts:uia.tree/observe", "desktop.ts:uia.window/list"]);
  assert.deepEqual(
    DESKTOP_KERNEL_SHAPES.map(([capability, operation]) => `${capability}/${operation}`),
    ["uia.window/list", "uia.tree/observe"]
  );
  const desktop = readFileSync(join(srcDir, "desktop.ts"), "utf8");
  for (const forbidden of [
    /uia\.element["']/,
    /["'](invoke|set_value|select|toggle|scroll|capture|propose|execute|focus|activate|terminate|keyboard|mouse)["']/,
    /SendInput|SetForegroundWindow|keybd_event|mouse_event/
  ]) {
    assert.doesNotMatch(desktop, forbidden);
  }
});

test("the exposed desktop shapes are exactly the live shapes pinned in the provider", () => {
  const lib = readFileSync(join(repo, "crates", "qdral-provider-uia", "src", "lib.rs"), "utf8");
  const live = [...lib.matchAll(/\(\s*"(uia\.[a-z_.]+)",\s*"([a-z_]+)",\s*DesktopShapeQualification::LiveExposed,?\s*\)/g)].map(
    (match) => `${match[1] ?? ""}/${match[2] ?? ""}`
  );
  assert.deepEqual(live.sort(), ["uia.tree/observe", "uia.window/list"]);
});

test("the tree tool binds a typed window identity and the provider bounds", () => {
  const schema = desktopWindowTreeSchema("w");
  const ok = schema.parse({ window_id: "uia-win-0123456789abcdef", window_generation: 3 });
  assert.equal(ok.max_depth, 8);
  assert.equal(ok.max_nodes, 256);
  for (const bad of [
    { window_id: "uia-win-0123456789ABCDEF", window_generation: 3 },
    { window_id: "hwnd:1234", window_generation: 3 },
    { window_id: "uia-win-0123456789abcdef", window_generation: -1 },
    { window_id: "uia-win-0123456789abcdef", window_generation: 3, max_depth: 9 },
    { window_id: "uia-win-0123456789abcdef", window_generation: 3, max_nodes: 0 },
    { window_id: "uia-win-0123456789abcdef", window_generation: 3, max_nodes: 257 }
  ]) {
    assert.equal(schema.safeParse(bad).success, false, JSON.stringify(bad));
  }
});
