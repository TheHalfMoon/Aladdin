import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { LIVE_BROWSER_TOOLS } from "./tool_contract.js";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");

function toolSources(): string[] {
  return readdirSync(srcDir).filter(
    (name) => name.endsWith(".ts") && !name.endsWith(".test.ts")
  );
}

test("no MCP tool source forwards a browser capability to the kernel", () => {
  const offenders: string[] = [];
  for (const name of toolSources()) {
    const text = readFileSync(join(srcDir, name), "utf8");
    const lines = text.split("\n");
    lines.forEach((line, index) => {
      // SG-000108 slice 3a: the governed exposure tables in tool_contract.ts
      // name the live browser tool list (`LIVE_BROWSER_TOOLS`, empty until
      // slice 3b, which browser-exposure.test.ts ties to qdrald and the
      // host). No capability string or registration is allowed anywhere.
      const table = name === "tool_contract.ts" && /LIVE_BROWSER_TOOLS/.test(line);
      if (
        /(["'])browser\./.test(line) ||
        /registerBrowser/i.test(line) ||
        (!table && /browser[-_]tools?/i.test(line))
      ) {
        offenders.push(`${name}:${String(index + 1)}:${line.trim()}`);
      }
    });
  }
  assert.deepEqual(offenders, []);
  // Until slice 3b no live browser tool exists to forward.
  assert.deepEqual([...LIVE_BROWSER_TOOLS], []);
});

test("no browser tool is registered on the MCP server surface", () => {
  const text = readFileSync(join(srcDir, "server.ts"), "utf8");
  assert.match(text, /registerGitFetchTools/);
  // SG-000108 slice 3a: server.ts names the `browser_structured` profile
  // (selected by local configuration); it registers and forwards no
  // browser tool or capability.
  const withoutProfile = text.replace(/`?browser_structured`?/g, "");
  assert.doesNotMatch(withoutProfile, /browser/i);
});
