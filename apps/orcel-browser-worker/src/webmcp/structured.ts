// SG-000108 slice 5: the structured data (`<script type="application/ld+json">`)
// of the document a tool's submission opened, read from the engine's DOM
// snapshot (no page script runs). WebMCP's declarative explainer names this
// as the cross-document response of a form tool. The text is untrusted page
// data: bounded, made well formed, and free of controls (other than JSON
// whitespace) and invisible format characters.

import { sanitizeBlock } from "./declarative.js";
import type { DomElement, DomNode } from "./declarative.js";

export const MAX_RESULT_BLOCKS = 8;
export const MAX_RESULT_BLOCK = 8192;

export function structuredData(root: DomElement): string[] {
  const blocks: string[] = [];
  const stack: DomNode[] = [root];
  for (let node = stack.pop(); node && blocks.length < MAX_RESULT_BLOCKS; node = stack.pop()) {
    if (node.kind !== "element") continue;
    if (node.tag === "script" && (node.attrs.get("type") ?? "").trim().toLowerCase() === "application/ld+json") {
      const text = node.children.map((child) => (child.kind === "text" ? child.text : "")).join("");
      // Cut the raw text first, so sanitising never walks a huge block.
      const block = sanitizeBlock(text.slice(0, MAX_RESULT_BLOCK * 4), MAX_RESULT_BLOCK);
      if (block) blocks.push(block);
      continue;
    }
    for (let i = node.children.length - 1; i >= 0; i--) {
      const child = node.children[i];
      if (child) stack.push(child);
    }
  }
  return blocks;
}

/// Keep the longest prefix of `blocks` whose encoded size fits `budget`
/// bytes (each block encoded once).
export function fitBlocks(blocks: readonly string[], budget: number): { kept: string[]; complete: boolean } {
  const kept: string[] = [];
  let used = 0;
  for (const block of blocks) {
    const size = Buffer.byteLength(JSON.stringify(block), "utf8") + (kept.length > 0 ? 1 : 0);
    if (used + size > budget) return { kept, complete: false };
    used += size;
    kept.push(block);
  }
  return { kept, complete: true };
}
