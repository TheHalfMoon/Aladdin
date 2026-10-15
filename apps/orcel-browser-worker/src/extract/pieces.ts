// SG-000108 slice 6: fit an extraction into reply frames. The structure-
// aware chunks are a soft limit (one block larger than the chunk size stays
// whole), so each chunk is hard-split here until its content, as a JSON
// string on the wire, fits one frame. Every piece names its chunk, and the
// host joins a chunk's pieces again: a cut inside a chunk (inside a code
// fence, or just before text that would read as a heading) is a transport
// detail, never a boundary the caller sees. Linear in the Markdown's length.

import type { MarkdownChunk } from "./chunk.js";

/// UTF-8 bytes a code point takes inside a JSON string (escapes counted).
function wireCost(cp: number): number {
  if (cp === 0x22 || cp === 0x5c || cp === 0x0a || cp === 0x0d || cp === 0x09 || cp === 0x08 || cp === 0x0c) return 2;
  if (cp < 0x20) return 6;
  if (cp < 0x80) return 1;
  if (cp < 0x800) return 2;
  // A lone surrogate is escaped as \uXXXX.
  if (cp >= 0xd800 && cp <= 0xdfff) return 6;
  if (cp < 0x10000) return 3;
  return 4;
}

export interface Piece {
  /// The chunk this piece belongs to (non-empty chunks, numbered from 0).
  chunk: number;
  content: string;
}

export interface Pieces {
  pieces: Piece[];
  /// Pieces past `maxPieces` were dropped.
  truncated: boolean;
}

/// The chunks' contents, each split so that no piece takes more than
/// `maxBytes` on the wire. A split prefers the last line break in the second
/// half of the piece, else falls at a code point boundary. Empty chunks are
/// skipped; an empty extraction is one empty piece.
export function toPieces(chunks: readonly MarkdownChunk[], maxBytes: number, maxPieces: number): Pieces {
  if (maxBytes < 16) throw new Error("piece bound too small");
  const pieces: Piece[] = [];
  let chunk = -1;
  const push = (content: string): boolean => {
    if (pieces.length >= maxPieces) return false;
    pieces.push({ chunk, content });
    return true;
  };
  for (const { content: text } of chunks) {
    if (text === "") continue;
    chunk += 1;
    let start = 0;
    let bytes = 0;
    // The position just after the last line break in this piece, and the
    // piece's size up to it.
    let lineEnd = -1;
    let lineBytes = 0;
    let i = 0;
    while (i < text.length) {
      const cp = text.codePointAt(i) ?? 0;
      const width = cp > 0xffff ? 2 : 1;
      const cost = wireCost(cp);
      if (bytes + cost > maxBytes) {
        const atLine = lineEnd > start && lineBytes * 2 >= maxBytes;
        const cut = atLine ? lineEnd : i;
        if (!push(text.slice(start, cut))) return { pieces, truncated: true };
        // What follows the cut opens the next piece; it is at most half a
        // piece, so with the next code point it still fits.
        bytes = atLine ? bytes - lineBytes : 0;
        start = cut;
        lineEnd = -1;
        lineBytes = 0;
      }
      bytes += cost;
      i += width;
      if (cp === 0x0a) {
        lineEnd = i;
        lineBytes = bytes;
      }
    }
    if (start < text.length && !push(text.slice(start))) return { pieces, truncated: true };
  }
  if (pieces.length === 0) pieces.push({ chunk: 0, content: "" });
  return { pieces, truncated: false };
}
