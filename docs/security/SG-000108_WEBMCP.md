# SG-000108 WebMCP: slice 4, read-only discovery

Status: implementation record for slice 4 of SG-000108, following the founder decision of 2026-10-11 (`docs/aladdin/WEB_CAPABILITIES_DECISION.md`). This slice grants no execution: discovered tools are data. Execution is slice 5, with its own approval policy.

## What a caller gets

`BrowserWorker::discover_tools` returns the committed document's WebMCP **declarative** tools: forms that carry both `toolname` and `tooldescription`, following the W3C draft at `webmachinelearning/webmcp` `d0e4e0e`. Each tool has:
- a host-issued `tool_id` (`d<generation>-w<n>`), bound to the document generation. Like SG-000076 node identities, it is invalidated by every navigation attempt (failed or not), activation and download, and by every new discovery, including a failed one.
- its name (`[A-Za-z0-9_.-]{1,64}`, unique in the document; names shaped like host identities, `d<n>-...`, are not tools);
- its title, description and parameter descriptions as `PageText`, which is untrusted page data, never instructions;
- `autosubmit`, and `has_password`;
- parameters in a bounded schema subset: `string`, `number`, `boolean`, or `array` of declared choices. Their constraints are `enum`, `minimum`, `maximum`, `multiple_of`, `pattern` and `format`, each allowed only on its own type.

A page with no declarations yields an empty list, and the caller falls back to observation.

## How it reads the page without running page code

- The worker takes a CDP `DOMSnapshot.captureSnapshot` of the main document. The engine serialises its own DOM tree, so no script runs in the page: page-defined getters, patched prototypes and polyfills are never invoked, and nothing in the page changes. Declarative shadow roots are part of the snapshot.
- The tool rules are adapted from MCP-B's `declarative-forms.ts` (MIT; see `docs/aladdin/notices/WebMCP-org-npm-packages-MIT.txt` and `docs/aladdin/sg000108_webmcp_provenance.json`). They run in the worker on that attribute-only model.
- They read **attributes, not live state**. A value typed into a control is never visible, because the snapshot's input values are ignored. A control's `value` is never emitted. The only values that cross are the author-declared choice values of `option`, `radio` and `checkbox` controls.
- Text in an editable region (`contenteditable`, `role=textbox`) never contributes to a label or description.
- Password-like controls (`type=password`, or `autocomplete` set to `current-password`, `new-password` or `one-time-code`) are never parameters. They flag the tool through `has_password` even when disabled. SG-000077 already refuses filling password fields.
- **The tree model follows the DOM**, and differs from the donor's live-DOM rules where the model needs it:
  - Shadow roots are separate tree scopes. Form ownership, `form=` and `label for=` ids, and disabled fieldsets do not cross them.
  - A wrapping `label` labels only its first labelable descendant.
  - A disabled fieldset spares its first `legend`.
  - Attribute names are matched exactly, so a name set in another case with `setAttributeNS` is not a declaration.
  - Tag names are taken from the snapshot and lowercased. A foreign-namespace element named `FORM` is indistinguishable there and would count; it can only declare page data.

**Native `document.modelContext` is not used.** In Edge 155 it needs an experimental flag that the confinement argv does not admit. Calling a page's own polyfill (`getTools`) would run page code. So the SG-000108 invariant ("no code path evaluates caller-, model- or page-supplied script") holds unchanged for discovery, and the wording amendment named in the decision's open risks is not needed for this slice. It remains a prerequisite for slice 5, whose `executeTool` runs a page handler.

## Bounds and validation

- **Worker.** One discovery replies within 6 s (`DISCOVERY_TOTAL_MS`), inside the host's 10 s observation deadline, so a hostile page cannot end the lifetime through that deadline:
  - the engine's DOM snapshot has a 3 s deadline, and a slower capture fails as `discovery_failed`;
  - the linear tree pass and tool building run under at most 4 s of the remaining time, and are truncated past it;
  - the reply is fitted in linear time: each tool is encoded once, and the longest prefix that fits the 64 KiB frame is kept.

  The worker bounds everything before sending:
  - 32 tools; 32 parameters; names of 64 characters; titles of 128; descriptions of 1024; parameter descriptions of 512; 64 choices of 256 characters each; patterns of 256;
  - 100 000 snapshot nodes;
  - the reply frame's 64 KiB, by dropping tools from the end.

  Text is made well formed (a lone surrogate becomes U+FFFD, so the frame stays valid JSON for the host), has controls and invisible format characters removed, and is cut at whole code points. The invisible characters are bidi controls, zero-width marks, the BOM, the soft hyphen, interlinear annotation marks and Unicode tag characters. Values that must cross unchanged (parameter names, choices, patterns) are refused instead of altered.

  Number schemas carry no `pattern`. Bounds that contradict each other (minimum above maximum, including a `range` minimum above its default maximum of 100) are dropped. Any drop is reported as `truncated`.
- **Shared vectors.** `apps/qdral-browser-worker/src/webmcp/shared-vectors.json` pins the worker's output for awkward markup. The worker's tests require its output to equal the vectors, and the host's tests require `validate_tools` to accept every one, so a disagreement between the two sides fails in CI.
- **Host.** `webmcp::validate_tools` re-checks every bound and shape:
  - exact keys (`deny_unknown_fields`);
  - name syntax and uniqueness;
  - a non-zero, unique form node per tool;
  - no control characters, and none of the invisible format characters above;
  - no names shaped like host identities;
  - type-consistent constraints;
  - finite numbers, `minimum <= maximum`, and `multiple_of > 0`.

  A record outside these is a protocol violation. It is not repaired.
- The snapshot must be of exactly the committed URL. The worker compares the snapshot's own document URL with the page URL before and after the capture, and the host compares the result with the committed URL. A page that moved itself is stale.

## Known limits for slice 5

- For an `<option>` without a `value`, the reported choice is its text after bounding: controls and invisible marks are removed, and whitespace is collapsed. HTML collapses only ASCII whitespace, so the reported choice can differ from what the form would submit. Execution must match against the real option, never against this string.
- The host's shared-vector test reads the worker's `shared-vectors.json` through a monorepo-relative `include_str!`. That ties the crate's tests to the repository layout, and they would not build from a packaged crate.

## Deviation from the decision's slice plan

The decision's step for slice 4 says to "detect native `document.modelContext`" and to "report typed unavailable" for a page without WebMCP. This slice does neither:
- Native `document.modelContext` needs an experimental flag that the confinement argv does not admit. Detecting a page polyfill would mean calling page code.
- A page without declarations yields an empty list rather than an error, so the caller falls back to observation.

The fixture also covers ordinary pages rather than oversized declarations. Those are covered by unit tests and the shared vectors.

## Evidence

Native test `webmcp_declarative_tools_are_discovered_read_only_and_bound_to_the_document` (local, Edge 155, HTTPS fixture `/tools`):
- **Discovered:** `search_reports`, `login`, `contact` and `shadow_tool`, in tree order. The tool declared inside a declarative shadow root is included.
- **Not tools:** the duplicate `search_reports` and the form without a description.
- **Parameters:** `search_reports` has `q` (required) and `year` (choices 2026 and 2025). Its prefilled `value` and the hidden CSRF token never reach the host.
- **Password:** `login` is flagged `has_password`, and its password field is not a parameter. Neither `alice` nor the password value reaches the host.
- **Hostile text:** `contact`'s prompt-injection description is carried as `PageText`, unchanged.
- **Staleness:** identities are `d<generation>-w<n>`, and after navigating to a page with no tools the earlier identity is gone.
- **Awkward markup:** a lone surrogate set by script arrives as U+FFFD, and the lifetime continues. A number field with `pattern` and inverted bounds stays inside the host's subset, and `contenteditable` text in a label is not a description.
- **No page code ran.** The page's own script installs these traps, each of which fetches a trap URL if triggered:
  - a polyfill-style `document.modelContext`;
  - a patched `Element.prototype.getAttribute`;
  - armed after load: a MutationObserver, custom-element constructor, connected and attribute-changed callbacks, and focus, blur and selectionchange listeners.

  After discovery the test waits one second, and the fixture log shows no trap request. **Control:** on `/tools-control`, the same patched `getAttribute`, an armed MutationObserver and an armed focus listener, triggered by the page itself, all reach the fixture.

  The traps live in the page's main world, so they cannot detect evaluation in an isolated world. Discovery does no evaluation in any world; it issues only `DOMSnapshot.captureSnapshot`. Callbacks triggered by layout, such as `document.fonts` events after the snapshot's style and layout pass, are not trapped.

Unit tests:
- the worker's `webmcp/declarative.test.ts` covers schema rules, values, passwords, duplicates, disabled, read-only, nested and `form=`-owned controls, bounds, code-point cuts and control removal, and the snapshot model including shadow roots;
- the host's `webmcp.rs` and `discovery_frames_are_exact` cover validation and the frame shapes.
