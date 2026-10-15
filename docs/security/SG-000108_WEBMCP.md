# SG-000108 WebMCP: slice 4, read-only discovery

Status: implementation record for slice 4 of SG-000108, following the founder decision of 2026-10-11 (`docs/orcel/WEB_CAPABILITIES_DECISION.md`). This slice grants no execution: discovered tools are data. Execution is slice 5, with its own approval policy.

## What a caller gets

`BrowserWorker::discover_tools` returns the committed document's WebMCP **declarative** tools: forms that carry both `toolname` and `tooldescription`, following the W3C draft at `webmachinelearning/webmcp` `d0e4e0e`. Each tool has:
- a host-issued `tool_id` (`d<generation>-w<n>`), bound to the document generation. Like SG-000076 node identities, it is invalidated by every navigation attempt (failed or not), activation and download, and by every new discovery, including a failed one.
- its name (`[A-Za-z0-9_.-]{1,64}`, unique in the document; names shaped like host identities, `d<n>-...`, are not tools);
- its title, description and parameter descriptions as `PageText`, which is untrusted page data, never instructions;
- `autosubmit`, and `has_password`;
- parameters in a bounded schema subset: `string`, `number`, `boolean`, or `array` of declared choices. Their constraints are `enum`, `minimum`, `maximum`, `multiple_of`, `pattern` and `format`, each allowed only on its own type.

A page with no declarations yields an empty list, and the caller falls back to observation.

## How it reads the page without running page code

- The worker takes a CDP `DOMSnapshot.captureSnapshot` of the main document. The engine serialises its own DOM tree, so no script runs in the page: page-defined getters, patched prototypes and polyfills are never invoked, and nothing in the page changes. Open, closed and declarative shadow trees are part of the snapshot, flattened: Edge reports no shadow-root node, only a `shadowRootType` mark on every node inside a shadow tree. The model puts a shadow-root boundary where a marked node sits under an unmarked one. A shadow tree nested in another one cannot be told apart from its parent tree, so it is modelled as part of it. (Slice 4 as first merged assumed shadow-root nodes. Its synthetic tests did too, so a shadow-tree field could be counted as an outer form's parameter. Corrected in slice 5.)
- The tool rules are adapted from MCP-B's `declarative-forms.ts` (MIT; see `docs/orcel/notices/WebMCP-org-npm-packages-MIT.txt` and `docs/orcel/sg000108_webmcp_provenance.json`). They run in the worker on that attribute-only model.
- They read **attributes, not live state**. A value typed into a control is never visible, because the snapshot's input values are ignored. A control's `value` is never emitted. The only values that cross are the author-declared choice values of `option`, `radio` and `checkbox` controls.
- Text in an editable region (`contenteditable`, `role=textbox`) never contributes to a label or description.
- Password-like controls (`type=password`, or `autocomplete` set to `current-password`, `new-password` or `one-time-code`) are never parameters. They flag the tool through `has_password` even when disabled. SG-000077 already refuses filling password fields.
- **The tree model follows the DOM**, and differs from the donor's live-DOM rules where the model needs it:
  - Shadow roots are separate tree scopes (reconstructed from the snapshot's `shadowRootType` marks; see above). Form ownership, `form=` and `label for=` ids, and disabled fieldsets do not cross them.
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
- **Shared vectors.** `apps/orcel-browser-worker/src/webmcp/shared-vectors.json` pins the worker's output for awkward markup. The worker's tests require its output to equal the vectors, and the host's tests require `validate_tools` to accept every one, so a disagreement between the two sides fails in CI.
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

# Slice 5: approval-gated execution of declarative tools

Status: implementation record for slice 5 of SG-000108 (worker protocol generation 7). Execution is available as a host API only. **No MCP-facing tool is added**: exposing page tools to a model is a later change under SG-000079, with its own approval policy.

## What executes

`BrowserWorker::execute_tool` runs a tool discovered in slice 4 only when all of these hold. Anything else is refused before the approval is checked, so nothing is spent:
- the form declares `toolautosubmit`. Without it, WebMCP leaves submission to the user, and Orcel's isolated browser has no user to submit;
- it carries no password-like field;
- its `action` crossed exactly;
- its method is GET or POST;
- it has no hidden parameters, which cannot be entered;
- its structure is **self-contained**, as discovery reports it. Execution finds fields and the submit button by position in the form, and the browser submits by form ownership, so the two must agree:
  - every control and button below the form is its own (none carries `form=`);
  - none of its own lies outside it;
  - no shadow tree below it holds a control or button;
  - no submit button overrides the form (`form`, `formaction`, `formmethod`, `formenctype`, `formtarget`, `formnovalidate`);
  - it has a default submit button;
  - it opens no new browsing context (`target` empty or `_self`, or, without one, the document's `<base target>`);
  - it posts URL-encoded;
  - the form itself does not sit inside a shadow tree.

## How approval binds

- A new SG-000077 action, `execute` (role `webmcp-tool`), is required. A click or download approval is refused.
- The target's `state` is `execution_binding`: the tool name, a digest of its discovered record, the method, the resolved action, and the canonical arguments with keys sorted. The one-shot approval therefore covers every value the caller supplies, the method, and where it goes. The form's own fixed names (below) are submitted with the page's values, which the approval does not cover.
- An approval for other arguments, a forged approval and undeclared arguments are all refused unspent.

## What happens

1. **Fresh discovery.** The host discovers the page's tools again. The tool's record must be identical to the one the approval was given for, otherwise execution is refused as "the tool changed".
2. **Argument validation.** Arguments are checked host-side against the bounded schema:
   - types, required parameters and declared choices;
   - number bounds and step;
   - string length (1024 code points), with no controls (newlines and tabs are allowed in a textarea) and no invisible format characters.

   `pattern` and `format` are not evaluated by the host. The engine's native constraint validation enforces them when the form is submitted, and a form that fails validation opens no document, so the execution is not verified.
3. **Action.** The form's `action` is resolved against the committed page. It may be empty, absolute `https://`, or root-relative, in the form the engine requests unchanged: URL characters only, no backslash, no whitespace, no dot segments. Anything else is refused before dispatch, as is an action on another origin. The host then expects exactly one page-initiated document request:
   - for POST, the exact action;
   - for GET, the action's path.
4. **The submitted request is checked.** Each route request now carries its method, content type and body (a URL-encoded body of printable ASCII up to 16 KiB; otherwise none). The one expected request must use the bound method, and its query (GET) or body (POST) must give each approved field exactly its approved values:
   - a checked checkbox exactly once, an unchecked one not at all;
   - textarea line breaks as the engine's CRLF;
   - no other value on a declared parameter the caller did not supply;
   - a select or radio parameter, and any parameter the page already fills (`has_default`: a value, a checked box or a selected option, from the attributes or the engine's live state at discovery), is always supplied, since the page's value would be submitted otherwise. This is refused before dispatch, so the approval is not spent.

   Discovery also reports the form's **fixed names**: the names of its hidden, read-only and other non-parameter controls, of its default button (an image button's `name.x`/`name.y`, or `x`/`y`), and of any `dirname`, but never their values. The default button is the first button whose type is not `button` or `reset`, as HTML defines it: an invalid type submits, including one with surrounding spaces, which HTML does not trim. Those names may carry any value, such as a CSRF token. Any other name is refused, and so is a frame document while the submission is pending. A popup is refused during any pending activation, and the worker fails such a click at once (`denied`) instead of waiting out the navigation timeout (#313). A refused submission spends the expectation, so no later request can satisfy it. A page script that rewrites a field or adds one, a method override and a different form's submission are all refused at the request.

   A form submits UTF-8 only when its `accept-charset` is empty or starts with UTF-8, and the engine reports the document's encoding (`encodingName` in the snapshot, which reflects the header, a meta and the engine's default) as UTF-8. An undeclared or unreported encoding is not UTF-8. A form that does not submit UTF-8 executes only ASCII arguments; others are refused before dispatch.
5. **Dispatch.** `dispatch` consumes the approval.
6. **Filling.** The worker locates the form by `toolname` (exactly one match) and enters each field through Playwright's typed actions (`fill`, `selectOption`, `setChecked`, `check`), on host-built selectors with quoted names, within one 10 s filling budget. Discovery stays read-only; only execution enters values. The worker then clicks the form's default submit button.
7. **Verification.** A submission that opens no document is not a verified execution. Examples are a page script that calls `preventDefault`, or a field that fails native validation. A script that navigates elsewhere is refused at the request.
8. **Result.** The result is the opened document's URL and status, plus its `<script type="application/ld+json">` blocks (WebMCP's cross-document response). They are read from the engine's DOM snapshot without running script, and bounded to 8 blocks of 8192 code points as untrusted `PageText`. The blocks are fitted into one reply frame, and `structured_complete` is false when blocks were dropped or the snapshot failed. An outcome that still cannot be sent becomes a typed failure. The document is committed one generation later, and every earlier node and tool identity is stale. The host waits up to 90 s for an execution, above the worker's worst case.

## Page code

Orcel evaluates no page-, model- or caller-supplied script. It calls no page-registered function (`executeTool`, `getTools`) and no `respondWith`. The page's own submit handlers do run when the form is submitted, exactly as they run for any SG-000077 click; that is how the page does what its tool declares. The SG-000108 invariant therefore holds as written, and declarative execution needs no amendment. Executing imperative tools (`document.modelContext.executeTool`) would invoke a page-registered handler directly. It stays out of scope until that amendment is made.

## Known limits

- An `<option>` without a `value` is matched by its reported text. If that differs from the submitted value (see slice 4), the selection fails, which fails closed.
- A POST whose body is larger than 16 KiB, or not URL-encoded printable ASCII, cannot be checked and is refused.
- The fixed names' values are submitted as the page sets them. They are the page's own data, not the caller's, and the approval covers only the declared parameters. A submit handler may change a fixed value; it cannot add a name.
- `has_default` reflects the page at discovery. A script that fills a field afterwards is not seen; the request check then refuses the submission, after the approval is spent.
- The charset is the engine's own, so a header-only or late meta charset is seen as the engine applies it. A page script cannot change the encoding after load.
- A form inside a shadow tree is discovered but never executed, and nested shadow trees are modelled as part of their parent tree.

## Evidence

Native test `webmcp_tools_execute_only_with_approval_and_return_the_verified_result` (local, Edge 155, HTTPS fixture `/exec`):
- **GET.** `search_reports` with `{"q":"rust","year":"2025","exact":true}` opens `/search?q=rust&year=2025&exact=on&csrf=csrf-123`; the fixture log is the ground truth, and it shows the hidden field submitted by the browser. The returned JSON-LD echoes the query.
- **Refused unspent:** a forged approval, an approval for other arguments, and an undeclared argument.
- **Refused after use:** the stale tool identity, and separately a replay of the spent approval on a fresh identity.
- **POST.** `post_note` with a multi-line note opens exactly `/note`. The fixture received a 12-character note ("hello", CRLF, "world"), and the result is complete.
- **Not executable:** `needs_user` (no autosubmit) gets no binding. `escape` (action on `other.test`) is refused before dispatch, with its approval unspent.
- **No document:** `script_handled` (`preventDefault`) opens nothing, so it is not verified.
- **Elsewhere:** `redirecting` (script moves to `/elsewhere`) is refused at the request, and its approval is spent.
- **Structure:** `decoy` (a decoy field, with the real one owned from outside), `foreign_button` (its button submits another form), `shadowed` (a field in a shadow tree) and `method_override` (`formmethod` on the button) are not self-contained and get no binding.
- **Rewritten:** `rewriting`, whose submit handler replaces the approved value, is refused at the request.
- **Fixed names:** `hidden_token` reports `csrf` as fixed and submits the page's token. `adding`, whose submit handler appends a field, is refused at the request. `latin` (`accept-charset="iso-8859-1"`) refuses a non-ASCII argument with no binding and binds an ASCII one. The exec page declares UTF-8 and its tools report it; on `/tools`, which declares no charset, no tool does.
- **Defaults and browser names:** `defaulted` (a prefilled field) gets no binding without that field and submits the approved value with it (`PAGEYEAR` never sent). `browser_names` submits `q.dir=ltr` and `img.x`/`img.y`; `odd_button` (`type="bogus"`) submits its button name.
- **Popup:** an approved link with `target="_blank"` fails within 15 s, typed, and the popup's document is never requested.
- **Large result:** `big_result` returns 8 blocks of emoji too large for one frame. The outcome keeps fewer blocks, marks them incomplete, and the lifetime goes on.
- **Changed:** `mutating`, whose description changes after discovery, is refused as changed, with its approval unspent.
- **Never requested,** per the fixture log: `other.test`, `/elsewhere`, the refused arguments, the page defaults and other-form values (`PAGEDEFAULT`, `OTHERFORM`), the rewritten value, the script-added field (`SCRIPTADDED`), the page's prefilled value (`PAGEYEAR`), the popup's document, any non-ASCII argument to `latin`, and the changed tool's submission. The page's own token was submitted (`csrf=PAGETOKEN&q=tokened`).

The host's unit tests cover:
- the argument rules, executability, action resolution (canonical form) and result bounds;
- the submission check (methods, exact values, checkboxes, CRLF, cut or non-urlencoded bodies, malformed escapes);
- how an approval maps to a submission.

The worker's unit tests cover:
- the generation-7 frames;
- the request detail;
- the structural self-containment rules, on synthetic trees and on a snapshot flattened the way Edge reports shadow trees;
- an outcome too large to send becoming a typed failure.
