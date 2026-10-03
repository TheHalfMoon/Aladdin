# SG-000035 - Proposal-only coordinate derivation security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P10
Grain: SG-000035

## Purpose

SG-000035 establishes the third narrow QDRAL-P10 vision grain on top
of the closed SG-000034 visual proposal registry, the closed SG-000033
window-scoped capture registry, the closed QDRAL-P09 structured-UIA
registry, the SG-000018 replay-resistant foundation, SG-000019 class
enforcement, SG-000020 trust and revoke records, and the closed
QDRAL-P08 registry: a single approved proposal-only derivation shape
bound to server-derived proposal identity and expected proposal
generation, with deterministic frame-relative coordinates computed
from the exact bounded proposal region, server-allocated typed
coordinate identity, fresh SOFT approval digest binding, immediate
pre-derivation stale-proposal revalidation, and protected Qdral
approval-surface exclusion.

No input authority exists in this grain. Bounded input execution,
input leases, mouse input, keyboard input, `SendInput`, focus changes,
raw coordinate requests, caller-supplied coordinates, monitor scope,
desktop scope, clipboard access, network egress, and elevation have no
function in the derivation provider, policy, or dispatch layer.
Derivation never calls any input API. A missing or denied coordinate
identity is never actionable and must never become input authority;
input execution remains successor work. No silent escalation exists
when the proposal is stale, protected, or unavailable.

## Server-derived identity remains the core control

The derivation shape accepts only `proposal_id` and
`expected_proposal_generation`. Callers supply no coordinates.
Caller-supplied coordinate identities, coordinate fields, region
fields, approval material, secret fields, raw request fields, monitor
and desktop fields, input fields, and fallback directives are explicit
`CapabilityDenied` or `InvalidRequest` widening denials in the policy
layer. The raw `uia.coordinates/request` shape stays denied. Callers
can only present one server-allocated typed proposal identity plus
its expected proposal generation.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`. Caller
coordinates can never enter the derivation path because no coordinate
argument exists.

## Coordinate identity and stale protection

Derived coordinates are deterministic frame-relative evidence
geometry: the exact bounded region center computed with integer
division, always inside the frame. Every successful derivation mints
a server-allocated typed coordinate identity bound to the exact
proposal, derivation generation, coordinates, workspace, and policy
revision. Coordinate identities are evidence for the successor input
grain and never grant input execution authority. Replayed, foreign,
generation-drifted, and policy-drifted identities fail closed as
stale through the read-only coordinate check, and an identity whose
owning proposal, frame, window, or process drifted is stale and never
actionable again. Evidence carries only identities, generations,
coordinates, frame geometry, and the approval record.

## Approval binding

Every derivation carries fresh per-action SOFT approval with one-shot
exact-digest binding over workspace, policy revision, proposal
identity, proposal generation, frame identity, capture generation,
owning window, window generation, derived coordinates, and action
material. The digest is computed from the server-resolved binding
immediately before approval, approval is consumed one-shot with nonce
and expiry, and dispatch revalidates the same binding set
immediately before minting. Any material drift after approval fails
closed. No caller-supplied approval boolean is accepted, no
persistent reuse exists, and no remote delegation exists.

## Stale fail-closed behavior

Replayed proposals, generation drift, replaced windows, superseded
processes, wrong pairings, protected surfaces, workspace changes, and
policy revision drift all fail closed without silent retargeting. The
result explicitly reports that no truncation occurred. A successful
derivation never mutates proposal, frame, window, tree, or process
state because derivation is non-actuating; proposal and frame
identities are unaffected.

## Protected Qdral surfaces and secrets

Coordinates can never be derived for approval dialogs, STRONG
presence surfaces, workspace trust controls, emergency revoke, or
security-sensitive Qdral UI, because frames and proposals of
protected surfaces can never exist and direct derivation paths
re-check protected status. Fail closed on uncertainty. No password,
credential, Windows Hello, cookie, token, or session material enters
prompts beyond the bounded request itself, and records, history,
logs, MCP responses, snapshots, and evidence packets carry only
digests and metadata, never secret bytes.

## Windows qualification and honest limits

Derivation is pure registry computation over already-bound proposal
metadata, so no live desktop surface is required and no coordinates
are measured against a live screen. The native adapter continues to
prove real process identity against Windows APIs on Windows, and
deterministic derivation binding, coordinate identity, stale
protection, caller-coordinate denial, approval digest shape, and
protected-surface behavior is proven through the injected fake
adapter on every platform. These limits are recorded honestly and
must not be read as interactive desktop evidence.

## Retained behavior

All SG-000034 proposal behavior, SG-000033 capture behavior,
SG-000027 observation behavior, SG-000028 invoke behavior, SG-000029
value behavior, SG-000030 select behavior, SG-000031 toggle behavior,
SG-000032 scroll behavior, SG-000018 one-shot, expiry, digest, and
history behavior, SG-000019 SOFT and STRONG class enforcement,
SG-000020 trust and revoke behavior, and the closed P08 registry
behavior are retained unchanged. The existing
`%SystemRoot%\System32\whoami.exe` spawn boundary is unchanged. No
MCP coordinate tool exists, and the agent cannot reach derivation
capability through its own tool or input surface. Bounded input
execution with input leases and human interruption remain successor
work.
