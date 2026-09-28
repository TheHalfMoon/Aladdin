# SG-000023 - Read-only DOM and accessibility observation security note

Status: IMPLEMENTATION CANDIDATE
Program: COTRA-P08
Grain: SG-000023

## Purpose

SG-000023 establishes read-only structured browser observation on top of
the SG-000021 isolated automation profile and the SG-000022 typed page
lifecycle with origin binding: a single snapshot entry point that returns
bounded document metadata, DOM structure, and accessibility roles, names,
states, and safe values through server-allocated typed node identities,
while keeping DOM actuation, downloads, uploads, personal-profile mode,
debugging, scripting, credential access, MCP browser tools, and network
egress absent.

No browser is launched or attached in this grain. Snapshots are computed
read-only from the Cotra page registry only. Actual page rendering or page
loading by a browser engine remains absent and is successor work.

## Snapshot binding

`browser.snapshot/observe` binds one known page with an expected origin and
an expected generation. The page must be active with a non-empty current
origin on the isolated automation profile, in the caller workspace, and at
the current policy revision. Open pages with no document fail closed with
`TargetStale`. Unknown pages fail closed with `TargetStale`. Foreign
workspaces fail closed with `WorkspaceDenied`. Foreign profile identities
fail closed with `CapabilityDenied`. Origin drift fails closed with
`TargetStale`. Generation drift fails closed with `TargetStale`. Policy
revision drift fails closed with `TargetStale`.

Snapshot observation requires no fresh approval: it is a bounded local read
of an already-authorized active page and origin. No persistent reuse or
remote delegation is introduced.

## Typed node identity

Every node identity is server-allocated with an `nd-` prefix. The digest
binds profile identity, page identity, page generation, current origin,
document generation, node index, and policy revision under the
`COTRA_BROWSER_NODE_V1` domain. The snapshot identity uses an `ss-` prefix
and binds page identity, page generation, document generation, origin,
profile identity, and policy revision under `COTRA_BROWSER_SNAPSHOT_V1`.

Validation recomputes the node identity against the current page, profile,
origin, generation, and policy revision. Stale page generations, replaced
documents, origin drift, foreign profiles, foreign pages, forged
identities, and policy drift all fail closed with `TargetStale`, so later
structured actuation cannot accidentally target a stale or replaced
element. Raw browser-internal handles are never exposed as durable
authority.

## Bounds and data minimization

Snapshots are bounded by explicit `max_nodes` (1 to 200, default 50),
`max_depth` (1 to 8, default 4), and `max_bytes` (256 to 65536, default
16384). Missing values fall back to the defaults. Out-of-range values fail
closed as invalid requests. Serialized snapshots that exceed `max_bytes`
fail closed with `OutputLimit`. Observation cannot become unrestricted page
scraping.

Password input values always redact to `[redacted]`. Cookie, credential,
and session stores are unreachable: no snapshot node carries them and no
snapshot, evidence, log, or MCP response contains them. Arbitrary
JavaScript evaluation is denied. The snapshot role set is intentionally
small and structural (document, heading, link, button, paragraph, textbox)
with bounded names and values.

## Denied authority

The following shapes fail closed as denied capability shapes with no
silent coordinate fallback: DOM click, fill, type, press, select, submit,
toggle, and keyboard input; snapshot capture and actuation; screenshots
and visual or coordinate control; downloads and uploads; personal-profile
mode; stored credential, cookie, password, and session access; arbitrary
DevTools, CDP commands, and browser scripting; network egress beyond local
destination validation and DNS resolution; and MCP browser tools.

Caller-supplied widening fields fail closed: caller-selected profile
roots, caller-selected node identities and selectors, browser argv,
scripting fields, credential material, and caller-supplied approval
material are denied. No MCP browser tool exists: the existing MCP test
that forbids browser capabilities on the MCP surface remains intact.

## Approval and trust retention

SG-000018 one-shot, expiry, digest, history, SG-000019 STRONG enforcement,
SG-000020 trust and revoke, SG-000021 profile and destination, and
SG-000022 page lifecycle and navigation behavior are unchanged. Snapshot
observation maps to the SOFT approval class as a local read and introduces
no STRONG execution authority. All P06 approval flows and predecessor
regressions still pass with digests unchanged outside observation labeling.

## Tests

Deterministic unit and security tests prove snapshot binding to one active
page, typed node identity with stale, wrong-profile, wrong-origin, wrong
generation, forged-identity, and policy-drift denial, bounded responses
with oversized denial, password redaction with secret absence, actuation
denial with no coordinate fallback, agent-forgery resistance through the
existing MCP browser denial test, and retained replay, expiry, digest,
drift, and double-consumption enforcement with predecessor regressions.
