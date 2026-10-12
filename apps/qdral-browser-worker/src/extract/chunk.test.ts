// Ported from browser-use `tests/ci/test_markdown_chunking.py` at
// browser-use/browser-use c75e8476e26d18b7617643bc2ae082fae8eae431,
// MIT License, Copyright (c) 2024 Gregor Zunic (see
// docs/aladdin/notices/browser-use-MIT.txt). The donor's HTML-to-Markdown
// pipeline cases run through Aladdin's own converter in extract.test.ts.

import assert from "node:assert/strict";
import { test } from "node:test";
import { chunkMarkdown } from "./chunk.js";

test("short content is one chunk", () => {
  const content = "# Hello\n\nSome short content.";
  const chunks = chunkMarkdown(content, 100_000);
  assert.equal(chunks.length, 1);
  assert.equal(chunks[0]?.content, content);
  assert.deepEqual([chunks[0]?.index, chunks[0]?.total, chunks[0]?.hasMore], [0, 1, false]);
});

test("empty content is one empty chunk", () => {
  const chunks = chunkMarkdown("", 100);
  assert.equal(chunks.length, 1);
  assert.equal(chunks[0]?.content, "");
  assert.equal(chunks[0]?.hasMore, false);
});

test("chunk offsets cover the whole content without gaps", () => {
  const content = "# Header\n\nParagraph one.\n\n# Header 2\n\nParagraph two.";
  const chunks = chunkMarkdown(content, 20);
  for (let i = 1; i < chunks.length; i++) assert.equal(chunks[i]?.start, chunks[i - 1]?.end, `gap before chunk ${i}`);
  assert.equal(chunks[0]?.start, 0);
  assert.equal(chunks.at(-1)?.end, content.length);
});

test("the last chunk ends at the content length even with a trailing newline", () => {
  for (const content of ["# Header\n\nParagraph one.\n\n# Header 2\n\nParagraph two.\n", "Hello\n", "Hello\n\nWorld\n"]) {
    assert.equal(chunkMarkdown(content, 20).at(-1)?.end, content.length, JSON.stringify(content));
  }
});

test("chunks split at header boundaries", () => {
  const sectionA = "# Section A\n\n" + "x".repeat(50);
  const sectionB = "\n\n# Section B\n\n" + "y".repeat(50);
  const chunks = chunkMarkdown(sectionA + sectionB, sectionA.length + 5);
  assert.ok(chunks.length >= 2);
  assert.ok(chunks[0]?.content.includes("# Section A"));
  assert.ok(chunks[1]?.content.includes("# Section B"));
});

test("a header in an overflowing chunk starts the next chunk", () => {
  const content = `${"A".repeat(600)}\n\n# Section B\n\n${"B".repeat(100)}`;
  const chunks = chunkMarkdown(content, 700);
  assert.ok(chunks.length >= 2);
  assert.ok(chunks[1]?.content.trimStart().startsWith("# Section B"));
  assert.ok(!chunks[0]?.content.includes("# Section B"));
});

test("a header split never leaves a tiny prefix chunk", () => {
  const content = `# Section A\n\n${"A".repeat(30)}\n\n# Section B\n\n${"B".repeat(600)}`;
  const chunks = chunkMarkdown(content, 700);
  assert.ok(chunks[0]?.content.includes("# Section A"));
  assert.ok(chunks[0]?.content.includes("# Section B"));
});

test("a code fence is never split, closed or not", () => {
  const code = "```python\n" + "x = 1\n".repeat(100) + "```";
  const chunks = chunkMarkdown(`# Title\n\n${code}\n\n# Footer\n\nDone.`, 50);
  assert.ok(chunks.some((c) => c.content.includes("```python") && (c.content.split("```python")[1] ?? "").includes("```")));
  const open = chunkMarkdown("# Title\n\n```python\nx = 1\ny = 2", 100_000);
  assert.equal(open.length, 1);
  assert.ok(open[0]?.content.includes("y = 2"));
});

test("table rows are never split, and the header carries into every continuation", () => {
  const rows = Array.from({ length: 50 }, (_, i) => `| item${i} | val${i} |`);
  const content = "# Data\n\n" + ["| Name | Value |", "| --- | --- |", ...rows].join("\n");
  for (const chunk of chunkMarkdown(content, 200)) {
    for (const line of chunk.content.split("\n")) {
      const s = line.trim();
      if (s.startsWith("|") && s.endsWith("|")) assert.ok((s.match(/\|/g) ?? []).length >= 3, s);
    }
  }
  const many = ["| Col1 | Col2 |", "| --- | --- |", ...Array.from({ length: 200 }, (_, i) => `| row${i} | data${i} |`)].join("\n");
  const chunks = chunkMarkdown(many, 200);
  assert.ok(chunks.length >= 3);
  for (let i = 1; i < chunks.length; i++) {
    assert.ok(chunks[i]?.overlap.includes("| Col1 | Col2 |"), `chunk ${i} header`);
    assert.ok(chunks[i]?.overlap.includes("| --- | --- |"), `chunk ${i} separator`);
  }
});

test("list items are never split", () => {
  const items = Array.from({ length: 50 }, (_, i) => `- Item ${i} with some description text`).join("\n");
  for (const chunk of chunkMarkdown("# List\n\n" + items, 200)) {
    for (const line of chunk.content.split("\n")) if (line.trim().startsWith("- ")) assert.ok(line.includes("Item"));
  }
});

test("startFrom returns the chunk containing the offset, nothing past the end, everything at zero", () => {
  const sectionA = "# A\n\nContent A here.";
  const content = sectionA + "\n\n# B\n\nContent B here.";
  const all = chunkMarkdown(content, sectionA.length + 5);
  assert.ok(all.length > 1);
  const from = chunkMarkdown(content, sectionA.length + 5, 5, (all[1]?.start ?? 0) + 1);
  assert.equal(from[0]?.index, all[1]?.index);
  assert.deepEqual(chunkMarkdown("# Hello\n\nWorld.", 100_000, 5, 99_999), []);
  assert.equal(chunkMarkdown("# Hello\n\nWorld.", 100_000, 5, 0).length, 1);
});

test("overlap carries the previous chunk's last lines, never into the first chunk", () => {
  const content = Array.from({ length: 100 }, (_, i) => `Line ${i}`).join("\n");
  const chunks = chunkMarkdown(content, 200, 3);
  if (chunks.length > 1) {
    assert.notEqual(chunks[1]?.overlap, "");
    assert.ok((chunks[1]?.overlap.split("\n").length ?? 0) <= 5);
  }
  assert.equal(chunkMarkdown("# A\n\nSome content.\n\n# B\n\nMore content.", 25)[0]?.overlap, "");
});

test("paragraphs are separate blocks, and a single oversized block is allowed", () => {
  assert.ok(chunkMarkdown("First paragraph with text.\n\nSecond paragraph with more text.", 30).length >= 2);
  const big = "x".repeat(200);
  const chunks = chunkMarkdown(big, 50);
  assert.equal(chunks.length, 1);
  assert.equal(chunks[0]?.content, big);
});

test("splitting stays linear when a chunk holds many tiny blocks", () => {
  const content = Array.from({ length: 60_000 }, (_, i) => (i % 2 === 0 ? "x" : "")).join("\n") + "\n# Late header\n\nTail.";
  const started = Date.now();
  const chunks = chunkMarkdown(content, 50_000);
  assert.ok(Date.now() - started < 1_000, `took ${Date.now() - started} ms`);
  assert.equal(chunks.at(-1)?.end, content.length);
});
