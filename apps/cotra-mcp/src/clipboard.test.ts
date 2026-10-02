import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { clipboardWriteSchema } from "./clipboard_network.js";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");

function toolSources(): string[] {
  return readdirSync(srcDir).filter(
    (name) => name.endsWith(".ts") && !name.endsWith(".test.ts")
  );
}

/**
 * SG-000064: exactly the closed clipboard/read and clipboard/write shapes
 * reach the kernel, only from clipboard_network.ts. Subscription, polling,
 * monitoring, history, watch, and paste shapes are never forwarded.
 */
test("only clipboard_network.ts forwards clipboard shapes, and only read and write", () => {
  const forwarded: string[] = [];
  for (const name of toolSources()) {
    const text = readFileSync(join(srcDir, name), "utf8");
    for (const match of text.matchAll(/capability:\s*["'](clipboard[a-z_.]*)["'],\s*operation:\s*["']([a-z_]+)["']/g)) {
      forwarded.push(`${name}:${match[1] ?? ""}/${match[2] ?? ""}`);
    }
    if (name !== "clipboard_network.ts") {
      assert.doesNotMatch(text, /["']clipboard["'.]/, `${name} must not name a clipboard capability`);
    }
  }
  assert.deepEqual(forwarded.sort(), ["clipboard_network.ts:clipboard/read", "clipboard_network.ts:clipboard/write"]);
  const source = readFileSync(join(srcDir, "clipboard_network.ts"), "utf8");
  assert.doesNotMatch(source, /["'](subscribe|poll|monitor|history|watch|paste)["']/);
  assert.doesNotMatch(source, /setInterval|SendInput|keybd_event/);
});

test("clipboard write text is bounded by UTF-8 bytes before reaching the kernel", () => {
  const schema = clipboardWriteSchema("w");
  assert.equal(schema.safeParse({ text: "notes" }).success, true);
  assert.equal(schema.safeParse({ text: "" }).success, false);
  assert.equal(schema.safeParse({ text: "a".repeat(65_536) }).success, true);
  assert.equal(schema.safeParse({ text: "a".repeat(65_537) }).success, false);
  assert.equal(schema.safeParse({ text: "\u00e9".repeat(32_769) }).success, false);
});
