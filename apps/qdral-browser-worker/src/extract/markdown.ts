// SG-000108 slice 6: rendered page content as Markdown.
//
// The skip and table rules are adapted from browser-use `browser_use/dom/
// serializer/html_serializer.py` (`HTMLSerializer`: non-content elements,
// hidden inline-JSON `<code>`, inline `data:` images, header-row table
// normalisation) at browser-use/browser-use
// c75e8476e26d18b7617643bc2ae082fae8eae431, MIT License, Copyright (c) 2024
// Gregor Zunic; the full notice is in docs/aladdin/notices/browser-use-MIT.txt.
// The donor serialises to HTML and converts it with the Python `markdownify`
// package; Aladdin converts its own content model directly (ATX headings,
// `-` bullets, GFM tables), with no new dependency. No markdownify code is
// copied. Unlike the donor (which disables escaping), page text is escaped:
// it can never form a link, image, HTML, heading, list or quote, so only the
// structure Aladdin emits is live Markdown.

import { cleanMarkdown, safeText } from "./clean.js";
import type { ContentElement, ContentNode } from "./model.js";

export interface MarkdownOptions {
  /// The document URL links and images resolve against.
  baseUrl: string;
  /// Keep links as `[text](url)` (http and https only); otherwise text.
  links: boolean;
  /// Keep images as `![alt](url)` (http and https only); otherwise alt text.
  images: boolean;
  /// Stop (truncated) once this many characters have been produced.
  maxChars: number;
  /// Stop (truncated) at this time (Date.now()).
  deadline: number;
}

export interface MarkdownResult {
  markdown: string;
  truncated: boolean;
}

const BLOCKS = new Set([
  "#root",
  "#shadow-root",
  "html",
  "body",
  "main",
  "article",
  "section",
  "div",
  "p",
  "header",
  "footer",
  "nav",
  "aside",
  "address",
  "figure",
  "figcaption",
  "details",
  "summary",
  "form",
  "fieldset",
  "legend",
  "dl",
  "dt",
  "dd",
  "center",
  "hgroup",
  "search",
  "caption"
]);
const HEADINGS = new Map([
  ["h1", 1],
  ["h2", 2],
  ["h3", 3],
  ["h4", 4],
  ["h5", 5],
  ["h6", 6]
]);
const BLOCK_LIKE = new Set(["pre", "blockquote", "ul", "ol", "table", "hr", "li"]);
/// Nesting beyond this is flattened into plain text (iteratively).
const MAX_DEPTH = 100;
/// Columns kept in one table.
const MAX_COLUMNS = 64;

function attr(element: ContentElement, name: string): string | undefined {
  return element.attrs.get(name);
}

function collapse(text: string): string {
  return text.replace(/[ \t\n\r\f]+/g, " ");
}

/// Escape page text so Markdown and HTML syntax in it stays literal, but
/// only where a character can form syntax, so identifiers and URLs in plain
/// text stay readable: `\ ` * [ ] < > |` always; `_` only at a word edge
/// (intraword `_` never forms emphasis); `!` only before `[`; `~` only when
/// doubled; `&` only before an entity-like name.
function escapeInline(text: string): string {
  return text
    .replace(/[\\`*[\]<>|]/g, (ch) => `\\${ch}`)
    .replace(/(?<![\p{L}\p{N}])_|_(?![\p{L}\p{N}])/gu, "\\_")
    .replace(/!(?=\\\[)/g, "\\!")
    .replace(/~(?=~)/g, "\\~")
    .replace(/&(?=#?[A-Za-z0-9]+;)/g, "\\&");
}

/// Escape what would start a block construct at the start of a line: an ATX
/// heading, a list marker, a quote, an ordered-list number (escaped as
/// `1\.`, since a backslash before a digit is not an escape), and a line of
/// only `-`, `=`, `_` or `*` (a thematic break or setext underline).
function escapeLineStart(line: string): string {
  if (/^\s*([-=_*])(\s*\1)*\s*$/.test(line)) return line.replace(/^(\s*)(\S)/, "$1\\$2");
  return line
    .replace(/^(\s*)(\d{1,9})([.)])(?=\s|$)/, "$1$2\\$3")
    .replace(/^(\s*)(#{1,6}(?=\s|$)|[-+](?=\s|$)|>)/, "$1\\$2");
}

/// The longest run of `ch` in `text`.
function longestRun(text: string, ch: string): number {
  let longest = 0;
  let run = 0;
  for (const c of text) {
    run = c === ch ? run + 1 : 0;
    if (run > longest) longest = run;
  }
  return longest;
}

/// All text below `node`, iteratively (for flattening deep nesting and for
/// code, where the text is literal).
function plainText(node: ContentNode): string {
  const parts: string[] = [];
  const stack: ContentNode[] = [node];
  for (let next = stack.pop(); next; next = stack.pop()) {
    if (next.kind === "text") {
      parts.push(next.text);
      continue;
    }
    for (let i = next.children.length - 1; i >= 0; i--) {
      const child = next.children[i];
      if (child) stack.push(child);
    }
  }
  return parts.join("");
}

class Stop extends Error {}

class Converter {
  private produced = 0;
  truncated = false;
  private readonly blockMemo = new WeakMap<ContentElement, boolean>();

  constructor(private readonly options: MarkdownOptions) {}

  /// Charge `chars` produced characters; stop past the size or time bound.
  private budget(chars: number): void {
    this.produced += chars;
    if (this.produced > this.options.maxChars || Date.now() > this.options.deadline) {
      this.truncated = true;
      throw new Stop();
    }
  }

  /// Escaped, collapsed page text, charged to the budget.
  private text(raw: string): string {
    this.budget(raw.length);
    return escapeInline(collapse(safeText(raw)));
  }

  private url(raw: string | undefined): string | undefined {
    if (!raw || raw.trim().toLowerCase().startsWith("data:")) return undefined;
    try {
      const url = new URL(raw.trim(), this.options.baseUrl);
      if (url.protocol !== "https:" && url.protocol !== "http:") return undefined;
      url.username = "";
      url.password = "";
      // Characters that would end or break a Markdown link destination.
      return url.href.replace(/[ ()<>\\]/g, (ch) => `%${ch.charCodeAt(0).toString(16).toUpperCase().padStart(2, "0")}`);
    } catch {
      return undefined;
    }
  }

  /// Inline Markdown for `node`'s content. `<br>` becomes "\n" (made a hard
  /// break when the paragraph is emitted); other whitespace collapses.
  inline(node: ContentNode, depth: number): string {
    if (node.kind === "text") return this.text(node.text);
    if (depth > MAX_DEPTH) return this.text(plainText(node));
    const inner = (): string => node.children.map((child) => this.inline(child, depth + 1)).join("");
    const wrap = (mark: string): string => {
      const text = inner();
      const core = text.trim();
      if (!core) return text;
      const lead = text.startsWith(" ") ? " " : "";
      const trail = text.endsWith(" ") ? " " : "";
      return `${lead}${mark}${core}${mark}${trail}`;
    };
    switch (node.tag) {
      case "br":
        return "\n";
      case "strong":
      case "b":
        return wrap("**");
      case "em":
      case "i":
        return wrap("*");
      case "del":
      case "s":
        return wrap("~~");
      case "code": {
        if (this.hiddenData(node)) return "";
        const raw = plainText(node);
        this.budget(raw.length);
        const text = collapse(safeText(raw)).trim();
        if (!text) return "";
        // A fence longer than any backtick run inside, padded when the text
        // starts or ends with a backtick.
        const fence = "`".repeat(longestRun(text, "`") + 1);
        const pad = text.startsWith("`") || text.endsWith("`") ? " " : "";
        return `${fence}${pad}${text}${pad}${fence}`;
      }
      case "a": {
        const text = inner();
        const href = this.options.links && node.shown ? this.url(attr(node, "href")) : undefined;
        if (href) this.budget(href.length);
        return href && text.trim() ? `[${text.trim()}](${href})` : text;
      }
      case "img": {
        if (!node.shown) return "";
        const src = attr(node, "src") ?? "";
        if (src.trim().toLowerCase().startsWith("data:")) return "";
        const alt = this.text(attr(node, "alt") ?? "").trim();
        const url = this.options.images ? this.url(src) : undefined;
        if (url) this.budget(url.length);
        return url ? `![${alt}](${url})` : alt;
      }
      default:
        return inner();
    }
  }

  /// The donor's rule for SPA state carried in hidden `<code>` (the style
  /// rule is covered by visibility; the id patterns are the donor's).
  private hiddenData(node: ContentElement): boolean {
    const id = (attr(node, "id") ?? "").toLowerCase();
    return id.includes("bpr-guid") || id.includes("data") || id.includes("state");
  }

  /// Block Markdown for `node`, appended to `out`.
  blocks(node: ContentElement, out: string[], depth: number): void {
    let line: string[] = [];
    const flush = (): void => {
      const text = line.join("");
      line = [];
      const paragraph = text
        .split("\n")
        .map((part) => escapeLineStart(part.replace(/ {2,}/g, " ").trim()))
        .filter(Boolean)
        .join("  \n");
      if (paragraph) out.push(paragraph);
    };
    for (const child of node.children) {
      if (child.kind === "text" || !this.isBlock(child)) {
        line.push(this.inline(child, depth + 1));
        continue;
      }
      flush();
      this.block(child, out, depth + 1);
    }
    flush();
  }

  /// Known block elements, and any other element (a custom element, a
  /// shadow host) with a block among its children: it lays out as a block.
  private isBlock(element: ContentElement): boolean {
    const known = this.blockMemo.get(element);
    if (known !== undefined) return known;
    // Iterative post-order, so deep nesting cannot overflow the stack.
    const order: ContentElement[] = [];
    const stack: ContentElement[] = [element];
    for (let next = stack.pop(); next; next = stack.pop()) {
      if (this.blockMemo.has(next)) continue;
      order.push(next);
      for (const child of next.children) if (child.kind === "element") stack.push(child);
    }
    for (let i = order.length - 1; i >= 0; i--) {
      const node = order[i];
      if (!node || this.blockMemo.has(node)) continue;
      const block =
        BLOCKS.has(node.tag) ||
        HEADINGS.has(node.tag) ||
        BLOCK_LIKE.has(node.tag) ||
        node.children.some((child) => child.kind === "element" && this.blockMemo.get(child) === true);
      this.blockMemo.set(node, block);
    }
    return this.blockMemo.get(element) ?? false;
  }

  private block(element: ContentElement, out: string[], depth: number): void {
    if (depth > MAX_DEPTH) {
      const text = this.text(plainText(element)).trim();
      if (text) out.push(escapeLineStart(text));
      return;
    }
    const level = HEADINGS.get(element.tag);
    if (level !== undefined) {
      const text = collapse(this.inline(element, depth)).trim();
      if (text) out.push(`${"#".repeat(level)} ${text}`);
      return;
    }
    switch (element.tag) {
      case "hr":
        out.push("---");
        return;
      case "pre": {
        const raw = plainText(element);
        this.budget(raw.length);
        const text = safeText(raw).replace(/\n$/, "");
        if (!text.trim()) return;
        // A fence longer than any backtick run inside cannot be closed by
        // the content.
        const fence = "`".repeat(Math.max(3, longestRun(text, "`") + 1));
        out.push(`${fence}\n${text}\n${fence}`);
        return;
      }
      case "blockquote": {
        const inner: string[] = [];
        this.blocks(element, inner, depth);
        const text = inner.join("\n\n");
        if (text) out.push(text.split("\n").map((part) => (part ? `> ${part}` : ">")).join("\n"));
        return;
      }
      case "ul":
      case "ol":
        this.list(element, out, depth);
        return;
      case "table":
        this.table(element, out, depth);
        return;
      default:
        this.blocks(element, out, depth);
    }
  }

  private list(element: ContentElement, out: string[], depth: number): void {
    const ordered = element.tag === "ol";
    let number = Number.parseInt(attr(element, "start") ?? "1", 10);
    if (!Number.isFinite(number) || number < 0 || number > 999_999_999) number = 1;
    const items: string[] = [];
    for (const child of element.children) {
      if (child.kind !== "element" || child.tag !== "li") {
        // Stray content in a list: kept as its own item.
        const text = (child.kind === "text" ? this.text(child.text) : this.text(plainText(child))).trim();
        if (text) items.push(`- ${text}`);
        continue;
      }
      const inner: string[] = [];
      this.blocks(child, inner, depth + 1);
      const marker = ordered ? `${number++}. ` : "- ";
      const indent = " ".repeat(marker.length);
      const body = inner.join("\n\n").split("\n");
      const [first = "", ...rest] = body;
      items.push([`${marker}${first}`, ...rest.map((part) => (part ? `${indent}${part}` : ""))].join("\n"));
    }
    if (items.length > 0) out.push(items.join("\n"));
  }

  private table(element: ContentElement, out: string[], depth: number): void {
    // Rows in order, whether directly under the table or in its sections,
    // found iteratively (a page can nest sections without limit).
    const rows: Array<{ cells: string[]; header: boolean }> = [];
    let headed = false;
    const stack: Array<{ node: ContentElement; inHead: boolean; level: number }> = [{ node: element, inHead: false, level: 0 }];
    const pending: Array<{ node: ContentElement; inHead: boolean; level: number }> = [];
    while (stack.length > 0) {
      const { node, inHead, level } = stack.pop() as { node: ContentElement; inHead: boolean; level: number };
      for (const child of node.children) {
        if (child.kind !== "element") continue;
        if (child.tag === "thead" || child.tag === "tbody" || child.tag === "tfoot") {
          if (level < MAX_DEPTH) pending.push({ node: child, inHead: child.tag === "thead", level: level + 1 });
          continue;
        }
        if (child.tag === "tr") {
          const cells = child.children.filter((c): c is ContentElement => c.kind === "element" && (c.tag === "td" || c.tag === "th")).slice(0, MAX_COLUMNS);
          const header = inHead || (rows.length === 0 && cells.length > 0 && cells.every((c) => c.tag === "th"));
          headed ||= header;
          rows.push({ header, cells: cells.map((cell) => collapse(this.inline(cell, depth + 1)).trim()) });
        } else if (child.tag === "caption") {
          const caption = collapse(this.inline(child, depth + 1)).trim();
          if (caption) out.push(escapeLineStart(caption));
        }
      }
      // Sections in document order.
      for (let i = pending.length - 1; i >= 0; i--) {
        const section = pending[i];
        if (section) stack.push(section);
      }
      pending.length = 0;
    }
    if (rows.length === 0) return;
    // The donor's normalisation: a header row (thead, or a first row of th)
    // leads; without one the first row serves as the header.
    const headerAt = headed ? rows.findIndex((row) => row.header) : 0;
    const [header] = rows.splice(Math.max(0, headerAt), 1);
    if (!header) return;
    const width = rows.reduce((most, row) => Math.max(most, row.cells.length), Math.max(header.cells.length, 1));
    const line = (cells: string[]): string => {
      const text = `| ${Array.from({ length: width }, (_, i) => cells[i] ?? "").join(" | ")} |`;
      this.budget(text.length);
      return text;
    };
    const lines = [line(header.cells), line(Array.from({ length: width }, () => "---"))];
    for (const row of rows) lines.push(line(row.cells));
    out.push(lines.join("\n"));
  }
}

/// Convert a content model to Markdown: escaped, safe text (well formed, no
/// controls or invisible marks), cleaned of SPA state blobs, bounded in size
/// and time.
export function toMarkdown(root: ContentElement, options: MarkdownOptions): MarkdownResult {
  const converter = new Converter(options);
  const out: string[] = [];
  try {
    converter.blocks(root, out, 0);
  } catch (error) {
    if (!(error instanceof Stop)) throw error;
  }
  let markdown = cleanMarkdown(out.join("\n\n"));
  let truncated = converter.truncated;
  if (markdown.length > options.maxChars) {
    markdown = Array.from(markdown).slice(0, options.maxChars).join("");
    truncated = true;
  }
  return { markdown, truncated };
}
