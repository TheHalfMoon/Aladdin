// SG-000108 slice 6: light cleanup of extracted Markdown, and the text
// safety rules every extracted string passes.
//
// `cleanMarkdown` is ported from browser-use `browser_use/dom/
// markdown_extractor.py` (`_preprocess_markdown_content`) at
// browser-use/browser-use c75e8476e26d18b7617643bc2ae082fae8eae431,
// MIT License, Copyright (c) 2024 Gregor Zunic; the full notice is in
// docs/aladdin/notices/browser-use-MIT.txt. Modifications for Aladdin:
// TypeScript; the JSON-blob patterns are bounded (no lazy match across the
// whole document) so a page cannot make them run in quadratic time; blank
// lines between blocks are kept (one at most), where the donor drops them
// all and so merges paragraphs.

// Controls other than newline and tab, and invisible format characters
// (bidi controls, zero-width marks, the BOM, the soft hyphen, interlinear
// annotation marks and tag characters), written as escapes.
const CONTROLS = /[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g;
const INVISIBLE = /[\u{61c}\u{ad}\u{200b}-\u{200f}\u{202a}-\u{202e}\u{2060}-\u{2064}\u{2066}-\u{206f}\u{feff}\u{fff9}-\u{fffb}\u{e0000}-\u{e007f}]/gu;
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

const CODE_JSON = new RegExp(`\`\\{["\\w][^\`]{0,${MAX_BLOB}}\\}\``, "g");
const TYPED_JSON = new RegExp(`\\{"\\$type":[^}]{100,${MAX_BLOB}}\\}`, "g");
const NESTED_JSON = new RegExp(`\\{"[^"]{5,256}":\\{[^}]{100,${MAX_BLOB}}\\}`, "g");

/// Remove SPA state blobs and tidy blank lines: JSON in inline code, long
/// `$type` objects and nested objects, and whole lines that parse as long
/// JSON are removed; whitespace-only lines become blank, and blank runs a
/// single blank line.
export function cleanMarkdown(content: string): string {
  const out = content.replace(CODE_JSON, "").replace(TYPED_JSON, "").replace(NESTED_JSON, "");
  const kept: string[] = [];
  for (const line of out.split("\n")) {
    const stripped = line.trim();
    if (!stripped) {
      if (kept.length > 0 && kept[kept.length - 1] !== "") kept.push("");
      continue;
    }
    // A long line that parses as JSON is SPA state, not content (a prefix
    // check alone would drop Markdown links and images).
    if (stripped.length > 100 && (stripped[0] === "{" || stripped[0] === "[")) {
      try {
        JSON.parse(stripped);
        continue;
      } catch {
        // Not JSON: keep it.
      }
    }
    kept.push(line.replace(/\s+$/, (trailing) => (trailing === "  " ? trailing : "")));
  }
  return kept.join("\n").trim();
}
