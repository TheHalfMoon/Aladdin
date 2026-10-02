import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { projectFetchBody, webFetchSchema } from "./clipboard_network.js";

const here = dirname(fileURLToPath(import.meta.url));
const srcDir = join(here, "..", "src");

function toolSources(): string[] {
  return readdirSync(srcDir).filter(
    (name) => name.endsWith(".ts") && !name.endsWith(".test.ts")
  );
}

/**
 * SG-000064: exactly the closed network/fetch shape reaches the kernel,
 * only from clipboard_network.ts, with a single url argument. No socket,
 * proxy, method, header, or body primitive exists on the MCP surface.
 */
test("only clipboard_network.ts forwards network/fetch, with only a url", () => {
  const forwarded: string[] = [];
  for (const name of toolSources()) {
    const text = readFileSync(join(srcDir, name), "utf8");
    for (const match of text.matchAll(/capability:\s*["'](network[a-z_.]*)["'],\s*operation:\s*["']([a-z_]+)["']/g)) {
      forwarded.push(`${name}:${match[1] ?? ""}/${match[2] ?? ""}`);
    }
    if (name !== "clipboard_network.ts") {
      assert.doesNotMatch(text, /["']network["'.\/_-]/, `${name} must not name a network capability`);
    }
  }
  assert.deepEqual(forwarded, ["clipboard_network.ts:network/fetch"]);
  const source = readFileSync(join(srcDir, "clipboard_network.ts"), "utf8");
  assert.match(source, /arguments: \{ url \}/);
  assert.doesNotMatch(source, /node:net|node:http|node:https|node:tls|node:dgram|fetch\(|new Socket/);
  assert.doesNotMatch(source, /\b(method|headers|proxy)\s*:/);
});

test("the fetch tool accepts only bounded https URLs", () => {
  const schema = webFetchSchema("w");
  assert.equal(schema.safeParse({ url: "https://example.com/docs" }).success, true);
  for (const url of ["http://example.com", "ftp://example.com", "file:///C:/x", "", `https://e.com/${"a".repeat(2048)}`]) {
    assert.equal(schema.safeParse({ url }).success, false, url);
  }
  assert.equal(schema.safeParse({ url: "https://example.com", method: "POST" }).success, true);
  assert.equal("method" in schema.parse({ url: "https://example.com", method: "POST" }), false);
});

test("fetch bodies project to UTF-8 text or binary metadata only", () => {
  const text = projectFetchBody({ status: 200, body_digest: "d", body: [104, 105] });
  assert.deepEqual(text, { status: 200, body_digest: "d", body_encoding: "utf-8", text: "hi" });
  const binary = projectFetchBody({ status: 200, body_digest: "d", body: [0xff, 0xfe, 0x00] });
  assert.deepEqual(binary, { status: 200, body_digest: "d", body_encoding: "binary-omitted" });
  assert.deepEqual(projectFetchBody({ status: 200, body: "not-bytes" }), { status: 200 });
  assert.deepEqual(projectFetchBody({ status: 200, body: [300] }), { status: 200 });
});
