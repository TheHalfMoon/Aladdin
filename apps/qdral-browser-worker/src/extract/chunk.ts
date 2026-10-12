// SG-000108 slice 6: structure-aware Markdown chunking.
//
// Ported from browser-use `browser_use/dom/markdown_extractor.py`
// (`chunk_markdown_by_structure`, `_parse_atomic_blocks`) at
// browser-use/browser-use c75e8476e26d18b7617643bc2ae082fae8eae431,
// MIT License, Copyright (c) 2024 Gregor Zunic; the full notice is in
// docs/aladdin/notices/browser-use-MIT.txt and the import record in
// docs/aladdin/sg000108_browser_donors_provenance.json. Modifications for
// Aladdin: TypeScript; names in Aladdin style; offsets count UTF-16 code
// units (JavaScript string indices) rather than Python code points; the
// chunk size and overlap are bounded by the caller.

export interface MarkdownChunk {
  content: string;
  index: number;
  total: number;
  /// Offsets into the full Markdown (UTF-16 code units, end exclusive).
  start: number;
  end: number;
  /// Trailing lines of the previous chunk (and a continued table's header).
  overlap: string;
  hasMore: boolean;
}

type BlockType = "header" | "code_fence" | "table" | "list_item" | "paragraph" | "blank";

interface AtomicBlock {
  type: BlockType;
  lines: string[];
  start: number;
  end: number;
}

const TABLE_ROW = /^\s*\|.*\|\s*$/;
const LIST_ITEM = /^(\s*)([-*+]|\d+[.)]) /;
const LIST_CONTINUATION = /^(\s{2,}|\t)/;

/// Phase 1: walk lines and group them into unsplittable blocks.
function parseAtomicBlocks(content: string): AtomicBlock[] {
  const lines = content.split("\n");
  const blocks: AtomicBlock[] = [];
  let i = 0;
  let offset = 0;
  const at = (n: number): string => lines[n] ?? "";

  while (i < lines.length) {
    const line = at(i);
    const lineLength = line.length + 1;

    if (!line.trim()) {
      blocks.push({ type: "blank", lines: [line], start: offset, end: offset + lineLength });
      offset += lineLength;
      i++;
      continue;
    }

    if (line.trim().startsWith("```")) {
      const fence = [line];
      let fenceEnd = offset + lineLength;
      i++;
      while (i < lines.length) {
        const next = at(i);
        fence.push(next);
        fenceEnd += next.length + 1;
        i++;
        if (next.trim().startsWith("```") && fence.length > 1) break;
      }
      blocks.push({ type: "code_fence", lines: fence, start: offset, end: fenceEnd });
      offset = fenceEnd;
      continue;
    }

    if (line.trimStart().startsWith("#")) {
      blocks.push({ type: "header", lines: [line], start: offset, end: offset + lineLength });
      offset += lineLength;
      i++;
      continue;
    }

    // A table's header and separator stay together; each data row is its
    // own block, so a table may split between rows.
    if (TABLE_ROW.test(line)) {
      const header = [line];
      let headerEnd = offset + lineLength;
      i++;
      if (i < lines.length && TABLE_ROW.test(at(i)) && at(i).includes("---")) {
        header.push(at(i));
        headerEnd += at(i).length + 1;
        i++;
      }
      blocks.push({ type: "table", lines: header, start: offset, end: headerEnd });
      offset = headerEnd;
      while (i < lines.length && TABLE_ROW.test(at(i))) {
        const row = at(i);
        blocks.push({ type: "table", lines: [row], start: offset, end: offset + row.length + 1 });
        offset += row.length + 1;
        i++;
      }
      continue;
    }

    if (LIST_ITEM.test(line)) {
      const items = [line];
      let listEnd = offset + lineLength;
      i++;
      while (i < lines.length) {
        const next = at(i);
        if (LIST_ITEM.test(next) || (next.trim() && LIST_CONTINUATION.test(next))) {
          items.push(next);
          listEnd += next.length + 1;
          i++;
          continue;
        }
        break;
      }
      blocks.push({ type: "list_item", lines: items, start: offset, end: listEnd });
      offset = listEnd;
      continue;
    }

    const paragraph = [line];
    let paragraphEnd = offset + lineLength;
    i++;
    while (i < lines.length && at(i).trim()) {
      const next = at(i);
      if (next.trimStart().startsWith("#") || next.trim().startsWith("```") || TABLE_ROW.test(next) || LIST_ITEM.test(next)) break;
      paragraph.push(next);
      paragraphEnd += next.length + 1;
      i++;
    }
    blocks.push({ type: "paragraph", lines: paragraph, start: offset, end: paragraphEnd });
    offset = paragraphEnd;
  }

  // Each line counted its newline; content without a final newline (or
  // ending in one) must not overshoot its length.
  const last = blocks[blocks.length - 1];
  if (last && content && last.end > content.length) last.end = content.length;
  return blocks;
}

const blockText = (block: AtomicBlock): string => block.lines.join("\n");
const size = (block: AtomicBlock): number => block.end - block.start;

function tableHeader(block: AtomicBlock): string | null {
  if (block.lines.length < 2) return null;
  const [header, separator] = block.lines;
  if (header === undefined || separator === undefined) return null;
  return separator.includes("---") || separator.includes("- -") ? `${header}\n${separator}` : null;
}

/// Split Markdown into structure-aware chunks: blocks accumulate up to
/// `maxChars` (a single larger block stays whole: a soft limit), a split
/// prefers a header boundary that leaves at least half a chunk behind, and
/// each later chunk carries the previous chunk's last `overlapLines` lines
/// (plus a continued table's header). `startFrom` returns the chunks from the
/// one containing that offset.
export function chunkMarkdown(content: string, maxChars = 100_000, overlapLines = 5, startFrom = 0): MarkdownChunk[] {
  if (!content) return [{ content: "", index: 0, total: 1, start: 0, end: 0, overlap: "", hasMore: false }];
  if (startFrom >= content.length) return [];
  const blocks = parseAtomicBlocks(content);
  if (blocks.length === 0) return [];

  const raw: AtomicBlock[][] = [];
  let current: AtomicBlock[] = [];
  let currentSize = 0;
  for (const block of blocks) {
    if (currentSize + size(block) > maxChars && current.length > 0) {
      // Prefix sizes once per split (the donor sums inside the loop, which
      // is quadratic in the blocks of a chunk).
      const prefix: number[] = [0];
      for (const b of current) prefix.push((prefix[prefix.length - 1] ?? 0) + size(b));
      let split = current.length;
      for (let j = current.length - 1; j > 0; j--) {
        if (current[j]?.type !== "header") continue;
        if ((prefix[j] ?? 0) >= maxChars * 0.5) {
          split = j;
          break;
        }
      }
      raw.push(current.slice(0, split));
      current = current.slice(split);
      currentSize = current.reduce((sum, b) => sum + size(b), 0);
    }
    current.push(block);
    currentSize += size(block);
  }
  if (current.length > 0) raw.push(current);

  const chunks: MarkdownChunk[] = [];
  let lastTableHeader: string | null = null;
  raw.forEach((chunkBlocks, index) => {
    const first = chunkBlocks[0];
    const final = chunkBlocks[chunkBlocks.length - 1];
    if (!first || !final) return;
    let overlap = "";
    const previous = raw[index - 1];
    if (previous) {
      const previousLines = previous.map(blockText).join("\n").split("\n");
      const trailing = overlapLines > 0 ? previousLines.slice(-overlapLines) : [];
      if (first.type === "table" && lastTableHeader) {
        const combined = lastTableHeader.split("\n");
        for (const line of trailing) if (!combined.includes(line)) combined.push(line);
        overlap = combined.join("\n");
      } else if (overlapLines > 0) {
        overlap = trailing.join("\n");
      }
    }
    // Keep the last seen header so a table spanning three or more chunks
    // carries it into every continuation.
    for (const block of chunkBlocks) {
      if (block.type !== "table") continue;
      const header = tableHeader(block);
      if (header !== null) lastTableHeader = header;
    }
    chunks.push({
      content: chunkBlocks.map(blockText).join("\n"),
      index,
      total: raw.length,
      start: first.start,
      end: final.end,
      overlap,
      hasMore: index < raw.length - 1
    });
  });

  if (startFrom > 0) {
    const from = chunks.findIndex((chunk) => chunk.end > startFrom);
    return from < 0 ? [] : chunks.slice(from);
  }
  return chunks;
}
