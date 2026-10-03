# Structured Browser Exposure Qualification (SG-000062)

Status: CANONICAL DECISION FOR QUNTAL-P16

## Question

SG-000021 through SG-000026 closed a structured browser layer: isolated
profile, origin-bound destinations, page lifecycle and navigation, DOM
snapshot, click and fill actuation, scoped downloads, and scoped uploads.
Before any of it becomes an MCP tool, each shape must be qualified against a
live browser engine. An MCP tool must not claim browser capability that the
implementation does not have.

## Evidence

- `crates/quntal-provider-browser` and `crates/quntald/src/browser.rs` launch no
  browser process, attach to none, and open no DevTools, debugger, WebSocket,
  or socket channel. The only process launches in the crate are test helpers
  that create junctions.
- Pages are records in a Quntal page registry. Navigation validates and
  records the destination; it does not load the page in an engine.
- `build_snapshot` documents and implements "deterministic structural
  metadata bound to the page generation and document generation, never live
  browser content": the nodes come from a fixed template.
- Click and fill bind typed node identities from those snapshots and record
  generation changes; they do not actuate a rendered page.
- Downloads and uploads move bytes the caller supplies under strict
  workspace rules; no page transfer occurs.

## Per-shape decision

| Shape | Engine | Decision |
| --- | --- | --- |
| `browser.profile/status` | model-only | not exposed |
| `browser.destination/validate` | model-only (policy) | not exposed |
| `browser.page/open` | model-only | not exposed |
| `browser.navigation/preview` | model-only | not exposed |
| `browser.navigation/navigate` | model-only | not exposed |
| `browser.snapshot/observe` | model-only (template nodes) | not exposed |
| `browser.dom/click` | model-only | not exposed |
| `browser.dom/fill` | model-only | not exposed |
| `browser.download/preview` | model-only | not exposed |
| `browser.download/download` | model-only | not exposed |
| `browser.upload/preview` | model-only | not exposed |
| `browser.upload/submit` | model-only | not exposed |

No shape qualifies against a live engine, so no browser tool is exposed.
Exposing these shapes would let an AI client believe it browsed a page when
it only exercised policy bookkeeping.

## What remains true

- The policy controls stay canonical and tested (origin binding, SSRF and
  redirect-widening denial, personal-profile denial, scoped transfers) and
  are the specification a future live engine must satisfy.
- Arbitrary JavaScript, generic DevTools or CDP access, debugger
  attachment, personal-profile use, and hidden network primitives remain
  intentionally denied.
- Live structured browser automation is recorded in the parity inventory as
  missing and outside the v0.2 target. It is not part of Desktop Commander's
  feature set, so it does not affect the Desktop Commander replacement goal.
  A future grain must bring a live, isolated engine and qualify every shape
  against it before exposure.

`apps/quntal-mcp/src/browser-qualification.test.ts` pins this decision: no
MCP tool is browser-named, the browser layer launches and attaches nothing,
and every shape above is recorded as not exposed.
