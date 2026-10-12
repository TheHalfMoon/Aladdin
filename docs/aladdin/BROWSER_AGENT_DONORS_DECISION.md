# Browser-agent donors: browser-use, Stagehand, Skyvern

Status: founder request (2026-10-12) to study these three projects and copy what is valuable. This record grants no authority: every capability still needs its own exact-head qualified slice inside SG-000108, and every new tool its own approval policy.

## Donor pins

All three are cloned read-only at exact commits under `C:\Users\Shehr\donors`. No donor script, install step or test was run; the probes described below ran Aladdin's own code.

| Donor | Commit | License | Disposition |
|---|---|---|---|
| [browser-use/browser-use](https://github.com/browser-use/browser-use/tree/c75e8476e26d18b7617643bc2ae082fae8eae431) | `c75e8476e26d18b7617643bc2ae082fae8eae431` | MIT, © 2024 Gregor Zunic (LICENSE blob `1ea3836c`) | **Port the extraction pipeline now** (below). The rest is reference. |
| [browserbase/stagehand](https://github.com/browserbase/stagehand/tree/f261c38b4dc50c39485f924e8599c1d3c6e37dc7) | `f261c38b4dc50c39485f924e8599c1d3c6e37dc7` | MIT, © 2024 Browserbase Inc. (LICENSE blob `683c709c`) | **Adapt later**: the CDP WebMCP client and the cross-frame accessibility snapshot. Nothing is imported in this change. |
| [Skyvern-AI/skyvern](https://github.com/Skyvern-AI/skyvern/tree/f5f3f28a9ce0021a1db24286928a7bff6582a1eb) | `f5f3f28a9ce0021a1db24286928a7bff6582a1eb` | AGPL-3.0 for the whole repository (LICENSE blob `0ad25db4`; no sub-package carries another license) | **Behavioural reference only**. No code is copied (see "Skyvern and the AGPL"). |

## How each maps onto Aladdin

Aladdin's browser stack fixes the frame:
- the Rust host is the policy authority (SG-000074..79);
- a private worker drives one confined engine with playwright-core over the host-owned pipe;
- no caller-, model- or page-supplied script is evaluated;
- the model is outside the browser layer (`ALADDIN_AI_SERVER_ARCHITECTURE.md`): Aladdin serves tools, it does not run a second planner.

All three donors are, at their core, an LLM agent loop wrapped around a browser. That loop is the part Aladdin does not take. The value is in the browser-side mechanics.

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
| `understudy/a11y/snapshot/*` (`capture.ts`, `a11yTree.ts`, `domTree.ts`, `treeFormatUtils.ts`) | Hybrid DOM plus accessibility snapshot using only `DOM.getDocument`, `DOM.describeNode` and `Accessibility.getFullAXTree`, stitched across frames and out-of-process iframes, with XPath maps | **Adapt later**, for observation across iframes. Aladdin admits one destination per lifetime today, so cross-origin frames are not yet in scope. |
| `understudy/domainPolicy.ts` | Allow and block lists turned into `Fetch` patterns | Rejected. SG-000075 (host-decided requests, pinned resolution, no route) is stronger. |
| `extension/` runtime: service worker, content script, `llm/`, `inference.ts`, act/extract/observe, `cacheService.ts` | A Chrome extension that runs the agent and LLM calls in the browser | Rejected. `WEB_CAPABILITIES_DECISION.md` rules out extensions and a second channel into the browser, and the model stays outside the browser. Action caching is noted as an idea below. |

### Skyvern (Python, AGPL-3.0)

| Module | What it does | Verdict |
|---|---|---|
| `skyvern/webeye/scraper/domUtils.js` | A 4,088-line script injected into the page to build an element tree | Rejected regardless of license: it evaluates script in the page. |
| `skyvern/core/script_generations/*` | Compiles a successful agent run into a runnable Python program (libcst) and replays it | Rejected as code: Aladdin never runs generated programs. The idea of replaying a verified run deterministically is noted below. |
| `skyvern/forge/` (agent, workflow blocks, copilot), credentials vault, database, UI | A whole hosted product | Behavioural reference only. |

## Skyvern and the AGPL

Skyvern is AGPL-3.0 and Aladdin is Apache-2.0. The founder's request says permission is held to copy and rename the source. `SOURCE_REUSE_MATRIX.md` applies the same rule here as to Firecrawl: AGPL code is copied only after a **written grant from the copyright holders** (Skyvern-AI) is filed in the repository and verified at source admission. No such grant is on file, so no Skyvern code is copied. If the grant is filed, module-level reuse can be reconsidered. The useful parts are Python and page-injected script, so the realistic yield would still be design ideas rather than code.

For the two MIT donors no grant is needed, and renaming is allowed. The copyright and permission notice must travel with the copied code, so `docs/aladdin/notices/browser-use-MIT.txt` holds it, and each ported file names its source path and commit.

## What is imported in this change

The browser-use extraction pipeline, ported to TypeScript as a pure library in `apps/qdral-browser-worker/src/extract/`. It is the Markdown half of slice 6 (native extraction) in `WEB_CAPABILITIES_DECISION.md`:
- `model.ts`: the rendered content of the main document from a `DOMSnapshot.captureSnapshot` taken with three computed styles (`display`, `visibility`, `opacity`). No page script runs. A text node is content only if the engine rendered it (it has a layout object) and its computed visibility is visible. Subtrees with opacity 0 are dropped, as are non-content elements and form controls. Hidden text, the usual way to smuggle instructions to a model, never reaches the caller. Live form values are never read.
- `markdown.ts`: content model to Markdown. Links and images are opt-in and kept only as resolved http(s) URLs. Output is bounded in size (default 1,000,000 characters) and time (default 4 s), and nesting past 100 levels is flattened. Every step is linear in the input (the donor's lazy blob regexes and quadratic chunk splitting are replaced).
- `clean.ts`: state-blob cleanup, plus the text safety rule (well formed, no controls, no invisible format characters).
- `chunk.ts`: structure-aware chunking.
- `index.ts`: `extractMarkdown(snapshot, options)`.

It adds no frame, no host API and no tool. The worker frame, the host-side bounds and the untrusted-page-text labelling come with the slice 6 wiring, after slice 5 (#311) merges, because both change the worker protocol generation. The full provenance is in `docs/aladdin/sg000108_browser_donors_provenance.json`.

### Evidence

- **Unit tests.** `extract/chunk.test.ts` holds the donor's chunking cases, ported (13 tests). `extract/extract.test.ts` (8 tests) runs on CDP-shaped snapshots and covers:
  - headings, emphasis, line breaks, nested and numbered lists, quotes, code blocks, rules;
  - tables (header rows, escaped pipes, short rows, captions);
  - opt-in links and images;
  - every hidden-content rule;
  - state-blob cleanup;
  - shadow trees;
  - size, time and chunking bounds;
  - 5,000-deep nesting.
- **Real Edge 155 probe** (offline `setContent`, Aladdin's built module on a real `DOMSnapshot`; the probe stayed outside the tree):
  - the expected Markdown came out;
  - dropped: `display: none`, `visibility: hidden`, `opacity: 0`, the `hidden` attribute, `<noscript>`, a closed `<details>`, a prefilled input, a value typed by script, and a textarea default;
  - kept: a visible child of a `visibility: hidden` parent, `display: contents` text, and closed-shadow-root content.

### Known limits

- Only the main document is read. Iframe documents are not.
- Slotted light-DOM text follows its shadow host's content instead of appearing at its slot.
- `aria-hidden` and visually hidden ("screen-reader only") text is kept, because it is rendered.
- A relative link or image on a document without an http(s) base URL is kept as text only.

## Ideas worth carrying, not code

- **Deterministic replay of a verified run** (Skyvern's script generation, Stagehand's action cache). For Aladdin this would be a recorded sequence of host-approved typed actions, replayed only with fresh approvals and fresh identities, never a generated program.
- **Field-level secret placeholders** (browser-use `sensitive_data`). The model sees a placeholder and the host injects the value at fill time. SG-000077 refuses password filling today; any change needs its own grain.
- **Interactivity heuristics and occlusion filtering** (browser-use) for elements the accessibility tree misses, as a later observation refinement.
