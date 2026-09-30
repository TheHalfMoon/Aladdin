import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");

function toolSources(): string[] {
  return readdirSync(srcDir).filter(
    (name) => name.endsWith(".ts") && !name.endsWith(".test.ts")
  );
}

test("no MCP tool source forwards a clipboard capability to the kernel", () => {
  const offenders: string[] = [];
  for (const name of toolSources()) {
    const text = readFileSync(join(srcDir, name), "utf8");
    const lines = text.split("\n");
    lines.forEach((line, index) => {
      if (
        /(["'])clipboard\./.test(line) ||
        /registerClipboard/i.test(line) ||
        /clipboard[-_]tools?/i.test(line)
      ) {
        offenders.push(`${name}:${String(index + 1)}:${line.trim()}`);
      }
    });
  }
  assert.deepEqual(offenders, []);
});

test("no clipboard tool is registered on the MCP server surface", () => {
  const text = readFileSync(join(srcDir, "index.ts"), "utf8");
  assert.match(text, /registerGitFetchTools/);
  assert.doesNotMatch(text, /clipboard/i);
});
