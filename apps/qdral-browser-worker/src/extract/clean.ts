// SG-000108 slice 6: light cleanup of extracted Markdown, and the text
// safety rules every extracted string passes.
//
// `cleanMarkdown` is ported from browser-use `browser_use/dom/
// markdown_extractor.py` (`_preprocess_markdown_content`) at
// browser-use/browser-use c75e8476e26d18b7617643bc2ae082fae8eae431,
// MIT License, Copyright (c) 2024 Gregor Zunic; the full notice is in
// docs/aladdin/notices/browser-use-MIT.txt. Modifications for Aladdin:
// TypeScript; the donor's lazy blob regexes are replaced by linear scans;
// blank lines between blocks are kept (one at most), where the donor drops
// them all and so merges paragraphs; lines inside code fences are kept
// verbatim; trailing whitespace is trimmed by a scan (a regex anchored at
// the line end is quadratic on long whitespace runs).

// Controls other than newline and tab, and invisible characters that hide
// text or carry hidden data: bidi controls, zero-width marks, the BOM, the
// soft hyphen, the combining grapheme joiner, Hangul, Khmer and Mongolian
// fillers and selectors, the blank braille pattern, variation selectors,
// interlinear annotation marks and tag characters. Written as escapes.
const CONTROLS = /[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g;
const INVISIBLE =
  /[\u{61c}\u{ad}\u{34f}\u{115f}\u{1160}\u{17b4}\u{17b5}\u{180b}-\u{180f}\u{200b}-\u{200f}\u{202a}-\u{202e}\u{2060}-\u{2064}\u{2066}-\u{206f}\u{2800}\u{3164}\u{fe00}-\u{fe0f}\u{feff}\u{ffa0}\u{fff9}-\u{fffb}\u{e0000}-\u{e007f}\u{e0100}-\u{e01ef}]/gu;
const LONE_SURROGATES = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/g;

/// Make text safe to carry as page data: well formed (a lone surrogate
/// becomes U+FFFD), CRLF and CR as LF, without controls other than newline
/// and tab, and without invisible format characters.
export function safeText(text: string): string {
  return text.replace(LONE_SURROGATES, "\ufffd").replace(/\r\n?/g, "\n").replace(CONTROLS, "").replace(INVISIBLE, "");
}

/// Upper bound on one JSON blob the cleanup removes (beyond it the text
/// stays: removal is a convenience, not a guarantee).
const MAX_BLOB = 65_536;

/// Remove the spans `[start, end)` from `text` (sorted, non-overlapping).
function cut(text: string, spans: Array<[number, number]>): string {
  if (spans.length === 0) return text;
  const parts: string[] = [];
  let at = 0;
  for (const [start, end] of spans) {
    parts.push(text.slice(at, start));
    at = end;
  }
  parts.push(text.slice(at));
  return parts.join("");
}

/// Inline code holding a JSON object (`` `{"…}` `` or `` `{word…}` ``).
/// Linear: each backtick is examined once. (The donor uses a lazy regex
/// across the whole document.)
function codeJsonSpans(text: string): Array<[number, number]> {
  const spans: Array<[number, number]> = [];
  let open = text.indexOf("`{");
  while (open >= 0) {
    const close = text.indexOf("`", open + 1);
    if (close < 0) break;
    const inner = text.slice(open + 1, close);
    const second = inner[1] ?? "";
    if (inner.endsWith("}") && inner.length - 1 <= MAX_BLOB && (second === '"' || /\w/.test(second))) {
      spans.push([open, close + 1]);
      open = text.indexOf("`{", close + 1);
    } else {
      open = text.indexOf("`{", open + 1);
    }
  }
  return spans;
}

/// Objects `{"<key>":` + at least 100 characters other than `}` + `}`,
/// where `<key>` is `$type` (the donor's `$type` pattern) or a nested
/// object's key of 5 to 256 characters followed by `{` (its nested
/// pattern). Linear: the next `}` is found once per region, not per start.
function stateObjectSpans(text: string): Array<[number, number]> {
  const spans: Array<[number, number]> = [];
  let nextBrace = -1;
  let from = 0;
  for (let start = text.indexOf('{"', from); start >= 0; start = text.indexOf('{"', from)) {
    from = start + 2;
    let body = -1;
    if (text.startsWith('{"$type":', start)) {
      body = start + 9;
    } else {
      const quote = text.indexOf('"', start + 2);
      const keyLength = quote - (start + 2);
      if (quote > 0 && keyLength >= 5 && keyLength <= 256 && text.startsWith('":{', quote)) body = quote + 3;
    }
    if (body < 0) continue;
    if (nextBrace < body) nextBrace = text.indexOf("}", body);
    if (nextBrace < 0) break;
    const length = nextBrace - body;
    if (length >= 100 && length <= MAX_BLOB) {
      spans.push([start, nextBrace + 1]);
      from = nextBrace + 1;
    }
  }
  return spans;
}

/// Remove SPA state blobs and tidy blank lines: JSON in inline code, long
/// `$type` objects and nested objects, and whole lines that parse as long
/// JSON are removed; whitespace-only lines become blank, and blank runs a
/// single blank line. Linear in the input.
export function cleanMarkdown(content: string): string {
  const withoutCode = cut(content, codeJsonSpans(content));
  const out = cut(withoutCode, stateObjectSpans(withoutCode));
  const kept: string[] = [];
  let fence = 0;
  for (const line of out.split("\n")) {
    // Code fences (as the converter writes them: a run of three or more
    // backticks) keep their lines verbatim.
    const run = backtickRun(line);
    if (fence > 0) {
      kept.push(line);
      if (run >= fence && line.trim().length === run) fence = 0;
      continue;
    }
    // An opening fence's info string never contains a backtick (CommonMark),
    // so an inline code span at a line start does not open one.
    if (run >= 3 && !line.trim().slice(run).includes("`")) {
      fence = run;
      kept.push(line);
      continue;
    }
    const stripped = line.trim();
    if (!stripped) {
      if (kept.length > 0 && kept[kept.length - 1] !== "") kept.push("");
      continue;
    }
    // A long line that parses as JSON (once the converter's escapes are
    // undone) is SPA state, not content; a prefix check alone would drop
    // Markdown links and images.
    const plain = stripped.length > 100 ? unescapeMarkdown(stripped) : "";
    if (plain && (plain[0] === "{" || plain[0] === "[")) {
      try {
        JSON.parse(plain);
        continue;
      } catch {
        // Not JSON: keep it.
      }
    }
    kept.push(trimEnd(line));
  }
  return kept.join("\n").trim();
}

/// The length of the run of backticks a line starts with (after spaces).
function backtickRun(line: string): number {
  let i = 0;
  while (i < line.length && line[i] === " ") i++;
  const start = i;
  while (i < line.length && line[i] === "`") i++;
  return i - start;
}

/// Undo the converter's backslash escapes.
function unescapeMarkdown(text: string): string {
  return text.replace(/\\([\\`*_[\]<>!|~&#+=.)-])/g, "$1");
}

/// Trim trailing whitespace by a backward scan, keeping exactly two spaces
/// before the end (a Markdown hard break).
function trimEnd(line: string): string {
  let end = line.length;
  while (end > 0 && /\s/.test(line[end - 1] ?? "")) end--;
  return line.slice(end) === "  " ? line : line.slice(0, end);
}
