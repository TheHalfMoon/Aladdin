# SG-000024 - Structured DOM actuation security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P08
Grain: SG-000024

## Purpose

SG-000024 establishes structured browser actuation on top of the
SG-000021 isolated automation profile, the SG-000022 typed page lifecycle
with origin binding, and the SG-000023 read-only snapshot observation with
typed node identity: SOFT-approved invoke (click) and value-entry (fill)
actions bound to exact page, page generation, origin, node identity,
expected role and state, document generation, policy revision, requested
action, and bounded value digest, with stale-node fail-closed and no silent
coordinate fallback, while keeping select, toggle, submit, keyboard input,
screenshots, downloads, uploads, password-field fill, personal-profile
mode, debugging, scripting, credential access, MCP browser tools, and
network egress absent.

No browser is launched or attached in this grain. Actuation records
authorized transitions in the Quntal registry only. Actual page rendering or
page loading by a browser engine remains absent and is successor work.

## Node consumption

Actuation consumes SG-000023 server-allocated `nd-` node identities. Every
snapshot persists server-side node records (identity, page, workspace,
profile, origin, page and document generation, index, role, input type,
state, policy revision) under Quntal protected local state. Dispatch
resolves caller-supplied identities through this registry: unknown
identities fail closed with `TargetStale`.

Verification recomputes the node identity against the current profile,
page, generation, origin, document generation, index, and policy revision
and additionally requires the server-recorded role and state to equal the
caller-expected role and state. Stale generations, replaced documents,
origin drift, foreign profiles, foreign pages, forged identities, role or
state mismatch, and policy drift all fail closed. Only `enabled` nodes
actuate; disabled nodes fail closed with `CapabilityDenied`.

Role gating is closed: invoke reaches only `link` and `button` roles and
value-entry reaches only `textbox` roles. Every other role and verb
combination fails closed with `CapabilityDenied`. Password targets fail
closed with `CapabilityDenied`: credential submission remains absent.

## Approval and mutation

Each invoke and each fill requires a fresh SOFT approval with exact digest
binding over workspace, policy revision, profile identity, page identity,
expected origin and generation, document generation, node identity,
expected role and state, requested action, and bounded value material under
the `QUNTAL_BROWSER_ACTUATION_V1` domain. The digest uses length-prefixed
SHA-256 fields. Any material drift invalidates the approval. One-shot
consumption applies: a consumed approval never authorizes a second action.

Every successful actuation bumps the page generation while keeping the
origin, so every previously observed node identity goes stale and the next
action requires a fresh observation. Replayed generations fail closed with
`TargetStale` before any approval prompt and again before mutation.

## Value bounds

Fill values are bounded caller text of at most 4096 bytes. Oversized
values fail closed before any digest is produced. Invoke accepts no value
field. Fill evidence carries only the value digest, never the value
itself. Password-field fill is denied at the node check, so credential
material never enters actuation records.

## Denied authority

The following shapes fail closed as denied capability shapes with no
silent coordinate fallback: select, toggle, type, press, write, snapshot,
and observe under `browser.dom`; snapshot capture and actuation;
screenshots and visual or coordinate control; downloads and uploads;
personal-profile mode; stored credential, cookie, password, and session
access; arbitrary DevTools, CDP commands, and browser scripting; keyboard
input beyond the bounded fill verb; form submission as a standalone verb;
network egress beyond local destination validation and DNS resolution; and
MCP browser tools.

Caller-supplied widening fields fail closed: caller-selected profile
roots, coordinates, selectors, browser argv, scripting fields, credential
material, and caller-supplied approval material are denied. No MCP browser
tool exists: the existing MCP test that forbids browser capabilities on
the MCP surface remains intact. OS-level input injection and UI automation
are not added; actuation is registry-bound structured authorization only.

## Approval and trust retention

SG-000018 one-shot, expiry, digest, history, SG-000019 STRONG enforcement,
SG-000020 trust and revoke, SG-000021 profile and destination, SG-000022
page lifecycle and navigation, and SG-000023 observation behavior are
unchanged. Actuation maps to the SOFT approval class and introduces no
STRONG execution authority. All P06 approval flows and predecessor
regressions still pass with digests unchanged outside actuation labeling.

## Tests

Deterministic unit and security tests prove node binding with server-side
records, stale and foreign denial, role and state mismatch denial,
disabled-node denial, password-target denial, per-action SOFT approval
with digest drift and double-consumption denial, bounded values with
oversized denial, generation-bump invalidation with replay denial,
extended-verb denial with no coordinate fallback, agent-forgery resistance
through the existing MCP browser denial test, and retained replay, expiry,
digest, drift, and predecessor regressions.
