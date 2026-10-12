# SG-000108 WebMCP: slice 4, read-only discovery

Status: implementation record for slice 4 of SG-000108, following the founder decision of 2026-10-11 (`docs/aladdin/WEB_CAPABILITIES_DECISION.md`). This slice grants no execution: discovered tools are data. Execution is slice 5, with its own approval policy.

## What a caller gets

`BrowserWorker::discover_tools` returns the committed document's WebMCP **declarative** tools: forms that carry both `toolname` and `tooldescription`, following the W3C draft at `webmachinelearning/webmcp` `d0e4e0e`. Each tool has:
- a host-issued `tool_id` (`d<generation>-w<n>`), bound to the document generation. It is invalidated by every navigation, activation and download, exactly like SG-000076 node identities.
- its name (`[A-Za-z0-9_.-]{1,64}`, unique in the document);
- its title, description and parameter descriptions as `PageText`, which is untrusted page data, never instructions;
- `autosubmit`, and `has_password`;
- parameters in a bounded schema subset: `string`, `number`, `boolean`, or `array` of declared choices. Their constraints are `enum`, `minimum`, `maximum`, `multiple_of`, `pattern` and `format`, each allowed only on its own type.

A page with no declarations yields an empty list, and the caller falls back to observation.

## How it reads the page without running page code

- The worker takes a CDP `DOMSnapshot.captureSnapshot` of the main document. The engine serialises its own DOM tree, so no script runs in the page: page-defined getters, patched prototypes and polyfills are never invoked, and nothing in the page changes. Declarative shadow roots are part of the snapshot.
- The tool rules are adapted from MCP-B's `declarative-forms.ts` (MIT; see `docs/aladdin/notices/WebMCP-org-npm-packages-MIT.txt` and `docs/aladdin/sg000108_webmcp_provenance.json`). They run in the worker on that attribute-only model.
- They read **attributes, not live state**. A value typed by the user is never visible, and a control's `value` is never emitted. The only values that cross are the author-declared choice values of `option`, `radio` and `checkbox` controls.
- Password controls are omitted, and they flag the tool through `has_password`. SG-000077 already refuses filling password fields.

**Native `document.modelContext` is not used.** In Edge 155 it needs an experimental flag that the confinement argv does not admit. Calling a page's own polyfill (`getTools`) would run page code. So the SG-000108 invariant ("no code path evaluates caller-, model- or page-supplied script") holds unchanged for discovery, and the wording amendment named in the decision's open risks is not needed for this slice. It remains a prerequisite for slice 5, whose `executeTool` runs a page handler.

## Bounds and validation

- **Worker.** It bounds everything before sending:
  - 32 tools; 32 parameters; names of 64 characters; titles of 128; descriptions of 1024; parameter descriptions of 512; 64 choices of 256 characters each; patterns of 256;
  - 100 000 snapshot nodes;
  - the reply frame's 64 KiB, by dropping tools from the end.

  Strings are cut at code points, never inside a surrogate pair, and controls are removed. Any drop is reported as `truncated`.
- **Host.** `webmcp::validate_tools` re-checks every bound and shape:
  - exact keys (`deny_unknown_fields`);
  - name syntax and uniqueness;
  - a non-zero, unique form node per tool;
  - no control characters;
  - type-consistent constraints;
  - finite numbers, `minimum <= maximum`, and `multiple_of > 0`.

  A record outside these is a protocol violation. It is not repaired.
- The snapshot must be of exactly the committed URL. A page that moved itself is stale.

## Evidence

Native test `webmcp_declarative_tools_are_discovered_read_only_and_bound_to_the_document` (local, Edge 155, HTTPS fixture `/tools`):
- **Discovered:** `search_reports`, `login`, `contact` and `shadow_tool`, in tree order. The tool declared inside a declarative shadow root is included.
- **Not tools:** the duplicate `search_reports` and the form without a description.
- **Parameters:** `search_reports` has `q` (required) and `year` (choices 2026 and 2025). Its prefilled `value` and the hidden CSRF token never reach the host.
- **Password:** `login` is flagged `has_password`, and its password field is not a parameter. Neither `alice` nor the password value reaches the host.
- **Hostile text:** `contact`'s prompt-injection description is carried as `PageText`, unchanged.
- **Staleness:** identities are `d<generation>-w<n>`, and after navigating to a page with no tools the earlier identity is gone.
- **No page code ran.** The page's own script installs a polyfill-style `document.modelContext` and patches `Element.prototype.getAttribute`. Either one fetches a trap URL if anything calls it, and the fixture log shows no trap request. **Control:** on `/tools-control`, the same patched `getAttribute`, called by the page itself, does reach the fixture.

Unit tests:
- the worker's `webmcp/declarative.test.ts` covers schema rules, values, passwords, duplicates, disabled, read-only, nested and `form=`-owned controls, bounds, code-point cuts and control removal, and the snapshot model including shadow roots;
- the host's `webmcp.rs` and `discovery_frames_are_exact` cover validation and the frame shapes.
