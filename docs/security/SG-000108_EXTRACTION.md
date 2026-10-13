# SG-000108 slice 6: native page extraction

Slice 6 of `docs/aladdin/WEB_CAPABILITIES_DECISION.md` reads the committed page's rendered content as Markdown. It wires the extraction library ported from browser-use (`apps/qdral-browser-worker/src/extract/`, recorded in `docs/aladdin/BROWSER_AGENT_DONORS_DECISION.md`) into the worker protocol and the host.

## Decision

- **No extraction dependency is admitted.** The library is Aladdin's own port: pure TypeScript with no packages. The "extraction dependency is admitted separately" step in the slice plan is therefore not needed. HTML and structured output stay out of this slice:
  - HTML would hand page markup to a caller, which observation and Markdown deliberately avoid;
  - structured data already crosses as JSON-LD blocks on execution results (slice 5).
- **No page script runs, and nothing on the page is touched.** The worker takes one `DOMSnapshot.captureSnapshot` with seventeen computed styles and layout boxes. The engine serialises its own DOM; nothing is scrolled, clicked, focused or rendered first.

  Text the engine has not rendered (off-screen `content-visibility: auto`) is therefore not read, and the result is marked `truncated`. Rendering it first would mean scrolling the page, which runs page code (scroll and intersection handlers). That is a page interaction, not a read.
- **Hidden text is dropped,** by the library's rules: no layout object, hidden visibility, transparent or near-transparent text, zero size, clipped "visually hidden" boxes, near-zero opacity, non-content elements and form controls. Hidden text is the usual way to smuggle instructions to a model.
- **The host API, not an MCP tool.** `BrowserWorker::extract_page(ExtractOptions)` is a host API. Exposing it to a model goes through SG-000079 like any other page text and is not part of this slice. No MCP-facing tool is added.

## Protocol (worker generation 8)

- **Request:** `extract {ext_id, links, images}`. The worker reads the primary page and converts it to Markdown within a 4 s CPU budget, after a snapshot bounded to 5 s, with at most 1,000,000 characters.
- **Splitting:** the Markdown is split into structure-aware chunks (headers, code fences, tables and list items stay whole where they can). Each chunk is then hard-split into pieces of at most 56 KiB on the wire, preferring a line break in the second half of the piece.
  - Every piece names its chunk, and the host joins a chunk's pieces again. A cut is therefore a transport detail, never a boundary the caller sees. Without the join, a cut inside an oversized code fence, or one a page aims exactly before `# …`, would make page text live syntax in a chunk read on its own.
  - At most 96 pieces are sent, and dropping any sets `truncated`. Only the last chunk is then cut short, and it may end mid-block (inside an open fence, which reads as code to its end); nothing follows it.
- **Reply:** `extracted {ext_id, url, truncated, piece, total, chunk, content}` carries piece 0. The host then asks `extract_more {ext_id, piece}` for each later piece, in order.
- **Retention:** the worker keeps the remaining pieces only until the last one is read, or until a new extraction replaces them.
- **Failure:** any failure is `extraction_failed {ext_id}`.

## Host checks

- **A committed document is required.** The URL the worker's snapshot was taken of (checked before and after the capture) must equal it, or the result is stale and refused. Every piece is of that one snapshot. The document's identity is its URL: a same-URL reload cannot be told apart.
- **Pieces arrive in order,** each naming the same URL, `truncated` flag and total (at most 96). Chunk numbers start at 0 and step by one: a piece continues the last chunk or opens the next.
- **Size:** at most 56 Ki characters per piece and 1,000,000 in all.
- **Text safety:** no control characters except line feed, carriage return and tab, and no bidi controls or invisible format characters. A violation is a protocol violation, never a silent repair.
- **Result:** a `PageExtraction {url, document_generation, truncated, chunks}`, whose chunks are `PageText`: untrusted page text, data to read, never an instruction, with no authority. An empty page has no chunks.
- **Timeouts:** the first piece is waited for up to 20 s. The worker's happy-path worst case is about 10 s: a 5 s snapshot, its detach, the 4 s budget, then linear chunking. Later pieces are already computed and get the 10 s reply timeout.
- **Huge pages:** a page of more than roughly 200,000 elements cannot be snapshotted within 5 s (the judge measured 9 s at 400,000 and 26 s at 1,000,000), so it fails as `extraction_failed`. The engine keeps serialising in the background, so a request right after it can time out on the host, which then terminates the worker. This is bounded and fails closed; discovery has the same exposure with a smaller snapshot.
- **Protocol violations:** a protocol violation inside `extract_page` returns `ProtocolViolation` and leaves the worker running, as discovery does. The worker drops its retained pieces at the next extraction.

## Markdown safety

The library escapes page text wherever a character could form syntax. Only the structure Aladdin emits is live, so a page cannot form a link, image, HTML, a heading, list, quote, rule, table or strikethrough.

Links and images are opt-in and kept only as resolved http(s) URLs without credentials. The library's records list the remaining exception: GFM autolinks of bare http(s), `www.` and e-mail text.

## Limits

These are carried over from the library:
- only the main document is read (no iframe documents, no SVG `<text>`);
- `aria-hidden` text is kept;
- visual tricks the snapshot cannot see stay possible: covered text, text coloured like its background, `clip-path` or mask clipping, and text positioned far to the right or below;
- a link wrapped around block content loses its target.

Also:
- **Chunk joins:** the chunks are structure-aware sections, not a byte-exact split. The line break between two chunks is not repeated in either.
- **History changes:** a page that changed its URL with `pushState` or `replaceState` (most single-page apps) cannot be extracted after it does: the URL no longer equals the committed one, so the result is stale. This fails closed, as discovery does.
- **Retained pieces:** they belong to one snapshot. A page that changes, or even navigates itself, after the snapshot is not re-read, and the result is that snapshot.

## Evidence

**Native test** `a_page_is_extracted_as_untrusted_markdown_in_pieces_without_hidden_text` (Windows, Edge, through the HTTPS fixture):
- Before any navigation, extraction is refused.
- `/read` has 160 sections of non-ASCII text, more than one piece. It comes back in several chunks, from the heading through the last section to the closing paragraph, with the non-ASCII text intact and not truncated.
- **Hidden text:** `display:none`, transparent and one-pixel-clipped text never appears.
- **Injected syntax:** a page-injected `[click me](javascript:…)` and `<script>` stay escaped, and a prompt-injection sentence is only text. A `<pre>` of about 78,000 characters, larger than one frame, comes back as one chunk, and the link injected at its end stays inside its fence.
- **Links:** they are absent by default. With links and images on, they are present as resolved `https://fixture.test/…` URLs.
- **No requests:** the extraction makes no request (the fixture log is unchanged).
- **A new document:** after navigating, the next extraction is of the new document, one generation later, and the lifetime goes on.

**Unit tests:**
- `extract/pieces.test.ts`:
  - every character kept, in order, within the wire bound, for ASCII, emoji, escapes, two-byte text and long lines;
  - surrogate pairs never split;
  - an oversized fence and an aimed heading cut join back into their chunks;
  - line-break preference;
  - the piece-count bound;
  - the largest piece fitting one frame with a 2 KiB URL;
  - linear time.
- `worker.test.ts`: paging, the release after the last piece, replacement, typed failure, and the launch requirement.
- `protocol.test.ts`: exact frames.
- Host `extraction.rs`: order, agreement, chunk joining and numbering, the bounds, text safety and the empty page. Host `worker_launch.rs`: exact frames.
