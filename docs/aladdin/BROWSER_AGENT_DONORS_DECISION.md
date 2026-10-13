# Browser-agent donors: browser-use, Stagehand, Skyvern, Firecrawl

Status: founder request (2026-10-12) to study these projects and copy what is valuable, revised the same day by the founder's attested permission to copy the source of all donors (`SOURCE_PERMISSIONS.md`). This record grants no authority: every capability still needs its own exact-head qualified slice inside SG-000108, and every new tool its own approval policy.

## Donor pins

All four are cloned read-only at exact commits under `C:\Users\Shehr\donors` (Firecrawl since 2026-10-11). No donor script, install step or test was run; the probes described below ran Aladdin's own code.

| Donor | Commit | License | Disposition |
|---|---|---|---|
| [browser-use/browser-use](https://github.com/browser-use/browser-use/tree/c75e8476e26d18b7617643bc2ae082fae8eae431) | `c75e8476e26d18b7617643bc2ae082fae8eae431` | MIT, © 2024 Gregor Zunic (LICENSE blob `1ea3836c`) | **Port the extraction pipeline now** (below). The rest is reference. |
| [browserbase/stagehand](https://github.com/browserbase/stagehand/tree/f261c38b4dc50c39485f924e8599c1d3c6e37dc7) | `f261c38b4dc50c39485f924e8599c1d3c6e37dc7` | MIT, © 2024 Browserbase Inc. (LICENSE blob `683c709c`) | **Adapt later**: the CDP WebMCP client and the cross-frame accessibility snapshot. Nothing is imported in this change. |
| [Skyvern-AI/skyvern](https://github.com/Skyvern-AI/skyvern/tree/f5f3f28a9ce0021a1db24286928a7bff6582a1eb) | `f5f3f28a9ce0021a1db24286928a7bff6582a1eb` | AGPL-3.0 at the root (LICENSE blob `0ad25db4`). Some files and packages carry other licenses: Apache-2.0 files with SPDX headers (e.g. `skyvern/forge/sdk/api/llm/ui_tars_llm_caller.py`), the Apache-2.0 package `skyvern/cli/mcpb/claude_desktop`, and a third-party MIT dataset (`evaluation/datasets/ODYSSEYS_ATTRIBUTION.md`) | **May be adapted under the founder's permission** (`SOURCE_PERMISSIONS.md`). No module fits Aladdin's architecture yet, so nothing is imported (see "Skyvern and the AGPL"). |
| [firecrawl/firecrawl](https://github.com/firecrawl/firecrawl/tree/f84845c864b6e2af22791c18fcf89ac2a21f1434) | `f84845c864b6e2af22791c18fcf89ac2a21f1434` | AGPL-3.0 (engine) | **Adapt next, under the founder's permission**: `apps/api/native/src/crawler.rs` for the slice 7 crawl queue (see "Firecrawl, after the permission"). |

## How each maps onto Aladdin

Aladdin's browser stack fixes the frame:
- the Rust host is the policy authority (SG-000074..79);
- a private worker drives one confined engine with playwright-core over the host-owned pipe;
- no caller-, model- or page-supplied script is evaluated;
- the model is outside the browser layer (`ALADDIN_AI_SERVER_ARCHITECTURE.md`): Aladdin serves tools, it does not run a second planner.

browser-use, Stagehand and Skyvern are, at their core, an LLM agent loop wrapped around a browser. That loop is the part Aladdin does not take; the value is in the browser-side mechanics. Firecrawl is different: a hosted scraping and crawling service, whose value for Aladdin is its crawl-scope and sitemap logic.

### browser-use (Python)

| Module | What it does | Verdict |
|---|---|---|
| `dom/markdown_extractor.py`: chunking, cleanup | Structure-aware Markdown chunks (code fences, tables and list items never split; table headers carried; header-preferred splits); SPA state-blob removal | **Ported** to `extract/chunk.ts` and `extract/clean.ts`, with the donor's tests. |
| `dom/serializer/html_serializer.py` | Snapshot tree to HTML, skipping non-content elements, hidden JSON `<code>`, inline `data:` images; table header normalisation | **Adapted** into `extract/markdown.ts`, which converts Aladdin's content model straight to Markdown (the donor needs the Python `markdownify` package). |
| `dom/service.py`, `dom/enhanced_snapshot.py`: visibility, layout parsing | CSS visibility from DOMSnapshot layout styles | **Adapted** into `extract/model.ts`. The donor's `Runtime.evaluate` calls are not used, and live input values are never read. |
| `dom/serializer/clickable_elements.py`, `paint_order.py`, `serializer.py` | Interactivity heuristics, occlusion filtering, indexed element list for an LLM | Reference for a later observation slice. Aladdin observes through Playwright's accessibility snapshot today. |
| `browser/watchdogs/*` | Download, crash, popup, permission handling | Reference. The host already supervises these. |
| `agent/`, `llm/`, `tools/`, `controller/`, `mcp/`, `sandbox/`, cloud, telemetry | Agent loop, model providers, hosted runtime | Rejected: a second planner and model stack. |

### Stagehand (TypeScript, v4)

| Module | What it does | Verdict |
|---|---|---|
| `packages/extension/understudy/page.ts`: WebMCP | Native WebMCP over the CDP `WebMCP` domain (`WebMCP.enable`, `toolsAdded`/`toolsRemoved`, `invokeTool`, `toolResponded`, `cancelInvocation`) | **Adapt later.** It is the native route that slices 4 and 5 deliberately did not take: Edge 155 exposes it only behind an experimental flag the SG-000074 argv allowlist does not admit. Admitting that flag is a reviewed argv amendment; this client would follow it. |
| `understudy/a11y/snapshot/*` (`capture.ts`, `a11yTree.ts`, `domTree.ts`, `treeFormatUtils.ts`) | Hybrid DOM plus accessibility snapshot built mainly from `DOM.getDocument`, `DOM.describeNode`, `DOM.getFrameOwner` and `Accessibility.getFullAXTree` (with `Runtime.enable`), stitched across frames and out-of-process iframes, with XPath maps. Its focus path resolves nodes through `Runtime.evaluate` and `Runtime.callFunctionOn` (`resolveObjectIdForXPath` in `a11yTree.ts`, `activeElement.ts`) | **Adapt later**, for observation across iframes, without the `Runtime` focus path (SG-000108 forbids page-context evaluation). Aladdin admits one destination per lifetime today, so cross-origin frames are not yet in scope. |
| `understudy/domainPolicy.ts` | Allow and block lists turned into `Fetch` patterns | Rejected. SG-000075 (host-decided requests, pinned resolution, no route) is stronger. |
| `extension/` runtime: service worker, content script, `llm/`, `inference.ts`, act/extract/observe, `cacheService.ts` | A Chrome extension that runs the agent and LLM calls in the browser | Rejected. `WEB_CAPABILITIES_DECISION.md` rules out extensions and a second channel into the browser, and the model stays outside the browser. Action caching is noted as an idea below. |

### Skyvern (Python, AGPL-3.0)

| Module | What it does | Verdict |
|---|---|---|
| `skyvern/webeye/scraper/domUtils.js` | A 4,088-line script injected into the page to build an element tree | Rejected regardless of license: it evaluates script in the page. |
| `skyvern/core/script_generations/*` | Compiles a successful agent run into a runnable Python program (libcst) and replays it | Rejected as code: Aladdin never runs generated programs. The idea of replaying a verified run deterministically is noted below. |
| `skyvern/forge/` (agent, workflow blocks, copilot), credentials vault, database, UI | A whole hosted product | Behavioural reference only. |

## Skyvern and the AGPL

Skyvern is AGPL-3.0 and Aladdin is Apache-2.0. On 2026-10-12 the founder attested official permission to copy and use the source code of every donor (`SOURCE_PERMISSIONS.md`), which lifts the license barrier for Skyvern and Firecrawl. On 2026-10-13 the founder confirmed the permissions explicitly and recorded that the documents stay confidential. The permission gate is satisfied by `docs/aladdin/grants/2026-10-13-founder-source-authorization.md`.

The license barrier was not the only one. With the permission in place, every Skyvern module was re-checked against the architecture:
- `webeye/scraper/domUtils.js` is a page-injected script, and SG-000108 forbids evaluating it.
- `core/script_generations` generates and runs programs, which Aladdin never does.
- `webeye/actions/handler.py`, `forge/` and the rest are Python, built around its own LLM loop and hosted product, and duplicate the host's typed actuation (SG-000077).

No Skyvern module is imported, by architecture rather than license. Its ideas are listed below, and any future Skyvern import names `SOURCE_PERMISSIONS.md` in its provenance.

## Firecrawl, after the permission

Firecrawl's engine (AGPL-3.0) was reference-only until the founder's permission of 2026-10-12. Its native Rust crate, `apps/api/native` (`firecrawl_rs`, napi bindings), is the closest fit for the planned slice 7 crawl queue, which lives in Aladdin's Rust host (`WEB_CAPABILITIES_DECISION.md`):

| Module | What it does | Verdict |
|---|---|---|
| `src/crawler.rs`: `filter_links`, `filter_url`, `validate_regexes` | Crawl-scope filtering: include and exclude patterns (regex), depth, backward crawling, subdomains, external links, robots.txt (`texting_robots`), with denial reasons | **Adapt in slice 7.** It drops the napi bindings and the tokio blocking-task wrappers. Its admitted-origin and SG-000075 checks stay the host's. Patterns are compiled with size limits. |
| `src/crawler.rs`: `parse_sitemap_xml`, `process_sitemap`, `exceeds_element_depth` | Sitemap and sitemap-index parsing (`roxmltree`), with a guard against deeply nested XML | **Adapt in slice 7**, for map. |
| `src/html.rs`: `extract_links`, `extract_metadata`, `extract_base_href`, `post_process_markdown` | Raw-HTML parsing (`lol_html`, `kuchikiki`) | Reference. Aladdin reads pages through the engine's DOMSnapshot, not raw HTML. The metadata fields (title, description, Open Graph) are a target for the slice 6 wiring. |
| `src/document.rs`, `src/pdf.rs` | Office and PDF to Markdown | Not now: SG-000078 keeps downloads inert, and opening or extracting them is out of scope. |
| `scrapeURL/lib/promptInjectionGuard.ts`, LLM extract, the hosted API, queues, billing | Model calls and service infrastructure | Rejected: the model is outside the browser layer, and Aladdin is not a hosted service. |

Its crates (`regex`, `url`, `texting_robots`, `roxmltree`) are separate works under their own licenses and are admitted with the slice 7 import.

For the MIT donors no grant is needed, and renaming is allowed. The copyright and permission notice must travel with the copied code, so `docs/aladdin/notices/browser-use-MIT.txt` holds it, and each ported file names its source path and commit.

## What is imported in this change

The browser-use extraction pipeline, ported to TypeScript as a pure library in `apps/qdral-browser-worker/src/extract/`. It is the Markdown half of slice 6 (native extraction) in `WEB_CAPABILITIES_DECISION.md`:
- `model.ts`: the rendered content of the main document, from a `DOMSnapshot.captureSnapshot` taken with seventeen computed styles (`CONTENT_STYLES`) and layout boxes. No page script runs, and live form values are never read. Hidden text, the usual way to smuggle instructions to a model, is dropped:
  - text without a layout object, or with hidden computed visibility;
  - transparent or near-transparent text (alpha below 0.01, by colour or text fill, in any colour syntax), text with a zero font size, text with a zero-area box or one past the document's top or left edge;
  - subtrees with opacity, or filter opacity, below 0.01;
  - text and attribute text inside a box that clips its content to at most four pixels, by overflow or by paint containment (the "visually hidden" pattern). The root and body never count as clipping, because their overflow applies to the viewport. Every enclosing clip is tracked, and only descendants that escape all of them stay, following the containing-block rules: a fixed-position descendant escapes a clip unless an element with a transform, perspective, filter, backdrop filter, layout or paint containment, a size container, or a matching will-change lies between; an absolutely positioned one escapes unless a positioned element or such an element lies between;
  - non-content elements and form controls.

  Attribute text (an image's alt, a link's href) is used only from elements that are themselves rendered and visible. Text the engine skipped rendering under `content-visibility: auto` sets `truncated`, so the loss is reported, not silent.
- `markdown.ts`: content model to Markdown.
  - Page text is escaped wherever a character could form syntax, so it can never form a Markdown link or image, HTML, a heading, list, quote, rule, table or strikethrough; only the structure Aladdin emits is live (the donor disables escaping). One exception remains: a renderer with GFM's autolink extension turns bare `http(s)://`, `www.` and e-mail text into links. Those can only be http(s) or mail links, never `javascript:`, and the output is meant for a model, not for rendering. Characters that cannot form syntax in place (an intraword `_`, a `!` not before `[`, an `&` not starting an entity) are left alone, so identifiers and URLs in plain text stay readable.
  - Links and images are opt-in and kept only as resolved http(s) URLs, without credentials, with the characters that end a link destination encoded.
  - Code fences and spans are longer than any backtick run they contain.
  - Tables keep at most 64 columns, and their sections are walked iteratively (up to 50 levels).
  - Every emitted part is charged to the size bound (default 1,000,000 characters) and the time bound (default 4 s), and nesting past 50 levels is flattened.
- `clean.ts`: state-blob cleanup by linear scans (the donor's lazy regexes can be made super-linear), with code-fence lines kept verbatim and trailing whitespace trimmed by a scan. Also the text safety rule: well formed, no controls, no invisible characters that hide text or carry data (bidi controls, zero-width marks, fillers, variation selectors, tag characters).
- `chunk.ts`: structure-aware chunking. A fence opens only on a CommonMark fence line (no backtick in its info string, so an inline code span at a line start never opens one), closes only on a line of at least as many backticks, and splits use running prefix sizes.
- `index.ts`: `extractMarkdown(snapshot, options)`.

It adds no frame, no host API and no tool. The worker frame, the host-side bounds and the untrusted-page-text labelling come with the slice 6 wiring, after slice 5 (#311) merges, because both change the worker protocol generation. The full provenance is in `docs/aladdin/sg000108_browser_donors_provenance.json`.

### Evidence

- **Unit tests** on CDP-shaped snapshots:
  - `extract/chunk.test.ts` (15 tests): the donor's chunking cases, ported, plus linear-time cases (the chunker has no budget of its own; its input is bounded by the 1,000,000-character output, and it was measured linear: 0.28 s at that size, 0.62 s at twice it, on the slowest shape);
  - `extract/extract.test.ts` (18 tests): conversion, tables, opt-in links and images, every hidden-content rule, cleanup (with a hostile input on which the donor's nested-object regex takes 1.8 s for a tenth of it), escaping of page-injected syntax (rules and setext headings included) with identifiers left readable, URL encoding, fences, a code span that must not swallow the page into one chunk, table and nesting bounds, invisible characters, and a 200,000-character non-breaking-space run, and deep quotes, lists and inline nesting around a large text.
- **Real-engine tests**, `extract/extract.native.test.ts` (5 tests, Windows, the installed Edge, offline `setContent` with every request aborted):
  - 30 hidden-content cases leak nothing: `display: none`, `visibility: hidden`, opacity 0 and 0.001, `filter: opacity(0)`, the `hidden` attribute, a zero font size, transparent text in `transparent`, `oklch(… / 0)` and `color(srgb … / 0)`, a transparent text fill, off-screen positioning to the left, one-pixel, two-pixel and zero-height clipped boxes (and an image and positioned text inside one), nested clips, a transformed clip box with fixed and absolutely positioned children, a `contain: paint` box, a size container with a fixed child, near-zero colour alpha, a closed `<details>`, `<noscript>`, hidden images and an image in a hidden link, a prefilled input, and a textarea default;
  - visible text, a visible child of a hidden parent, `display: contents` text, a shown image, and absolutely positioned and fixed children escaping a zero-height clip are kept, as is a single-page app whose body clips and is zero height;
  - closed shadow trees and slotted text come out in rendering order;
  - page-injected Markdown and HTML stay literal;
  - off-screen `content-visibility: auto` content is reported as truncated;
  - a 200,000 `&nbsp;` run and a 4,000 by 4,000 table: the extraction itself, timed apart from the engine launch, stays within its 4 s budget.

### Known limits

- Only the main document is read. Iframe documents and SVG `<text>` are not.
- `aria-hidden` text is kept, because it is visible on screen.
- Text the engine does not render (off-screen `content-visibility: auto`) is lost and only reported. The slice 6 wiring decided not to render it first, because scrolling runs page code; see `docs/security/SG-000108_EXTRACTION.md`.
- Visual tricks the snapshot cannot see stay possible: text covered by another element, text whose colour matches its background, text clipped by `clip-path` or hidden by a mask (`mask-image`), and text positioned far to the right or far below. Absolute positioning enlarges the document itself, so such text cannot be told apart from a wide or long page without dropping legitimate content.
- A link wrapped around block content (`<a><div>…</div></a>`) keeps its text but loses its target, because the inline box has no area (content is dropped, never leaked).
- A relative link or image on a document without an http(s) base URL is kept as text only.
- The chunk size is a soft limit, as in the donor: a single block larger than it (one long paragraph, a list without blank lines) stays one chunk. The slice 6 wiring hard-splits every chunk into frame-sized pieces (`extract/pieces.ts`).
- The state-blob cleanup, like the donor's, removes from a `{"$type":` opener up to the next `}`, which can span several blocks.

## Ideas worth carrying, not code

- **Deterministic replay of a verified run** (Skyvern's script generation, Stagehand's action cache). For Aladdin this would be a recorded sequence of host-approved typed actions, replayed only with fresh approvals and fresh identities, never a generated program.
- **Field-level secret placeholders** (browser-use `sensitive_data`). The model sees a placeholder and the host injects the value at fill time. SG-000077 refuses password filling today; any change needs its own grain.
- **Interactivity heuristics and occlusion filtering** (browser-use) for elements the accessibility tree misses, as a later observation refinement.
