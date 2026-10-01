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

test("no MCP tool source forwards a UIA capability to the kernel", () => {
  const offenders: string[] = [];
  for (const name of toolSources()) {
    const text = readFileSync(join(srcDir, name), "utf8");
    const lines = text.split("\n");
    lines.forEach((line, index) => {
      if (
        /(["'])uia\./.test(line) ||
        /registerUia/i.test(line) ||
        /uia[-_]tools?/i.test(line) ||
        /UIAutomation/i.test(line)
      ) {
        offenders.push(`${name}:${String(index + 1)}:${line.trim()}`);
      }
    });
  }
  assert.deepEqual(offenders, []);
});

test("no UIA tool is registered on the MCP server surface", () => {
  const text = readFileSync(join(srcDir, "server.ts"), "utf8");
  assert.match(text, /registerGitFetchTools/);
  assert.doesNotMatch(text, /uia/i);
  assert.doesNotMatch(text, /UIAutomation/);
});
