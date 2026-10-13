import assert from "node:assert/strict";
import { test } from "node:test";
import { encodeFrame, MAX_EXTRACT_PIECES, MAX_FRAME_BYTES, MAX_PIECE_BYTES } from "../protocol.js";
import type { MarkdownChunk } from "./chunk.js";
import { toPieces } from "./pieces.js";

const chunk = (content: string): MarkdownChunk => ({ content, index: 0, total: 1, start: 0, end: content.length, overlap: "", hasMore: false });
const wire = (text: string): number => Buffer.byteLength(JSON.stringify(text)) - 2;

test("pieces keep every character, in order, each within the wire bound", () => {
  const shapes = [
    "plain ".repeat(40_000),
    "\u{1f600}".repeat(30_000),
    '"\\'.repeat(40_000),
    "é".repeat(50_000),
    "line of text\n".repeat(10_000),
    "x".repeat(100) + "\n" + "y".repeat(200_000)
  ];
  for (const text of shapes) {
    const { pieces, truncated } = toPieces([chunk(text)], 1000, 10_000);
    assert.equal(truncated, false);
    assert.equal(pieces.join(""), text);
    assert.ok(pieces.every((p) => wire(p) <= 1000 && p.length > 0));
  }
  // A surrogate pair is never split.
  const { pieces } = toPieces([chunk("a" + "\u{1f600}".repeat(10))], 17, 100);
  assert.ok(pieces.every((p) => !/[\ud800-\udbff]$/.test(p)), JSON.stringify(pieces));
});

test("a split prefers a line break in the second half of the piece", () => {
  const text = "a".repeat(700) + "\n" + "b".repeat(700);
  const { pieces } = toPieces([chunk(text)], 1000, 10);
  assert.deepEqual(pieces, ["a".repeat(700) + "\n", "b".repeat(700)]);
  const early = "a".repeat(100) + "\n" + "b".repeat(1500);
  assert.ok((toPieces([chunk(early)], 1000, 10).pieces[0]?.length ?? 0) > 900, "an early break is not used");
});

test("chunks stay separate, empty ones are skipped, and the piece count is bounded", () => {
  assert.deepEqual(toPieces([chunk("a"), chunk(""), chunk("b")], 1000, 10), { pieces: ["a", "b"], truncated: false });
  assert.deepEqual(toPieces([chunk("")], 1000, 10), { pieces: [""], truncated: false });
  const many = toPieces([chunk("z".repeat(10_000))], 100, 5);
  assert.deepEqual(many, { pieces: Array<string>(5).fill("z".repeat(100)), truncated: true });
});

test("the largest piece still fits a reply frame with the longest URL", () => {
  const { pieces } = toPieces([chunk('"\u0001'.repeat(100_000))], MAX_PIECE_BYTES, MAX_EXTRACT_PIECES);
  const frame = encodeFrame({
    frame: "extracted",
    ext_id: 2 ** 31,
    url: "https://x.test/" + "a".repeat(2033),
    truncated: true,
    piece: 95,
    total: 96,
    content: pieces[0] ?? ""
  });
  assert.ok(frame.length <= MAX_FRAME_BYTES + 4, `${frame.length}`);
});

test("splitting is linear", () => {
  const time = (n: number): number => {
    const text = "ab\n".repeat(n);
    const started = process.hrtime.bigint();
    toPieces([chunk(text)], 64, 1_000_000);
    return Number(process.hrtime.bigint() - started);
  };
  time(10_000);
  const small = time(100_000);
  const large = time(400_000);
  assert.ok(large < small * 10, `${small} vs ${large}`);
});
