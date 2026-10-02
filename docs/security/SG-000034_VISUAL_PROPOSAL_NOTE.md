# SG-000034 - Non-actuating visual target proposals security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P10
Grain: SG-000034

## Purpose

SG-000034 establishes the second narrow QUNTAL-P10 vision grain on top
of the closed SG-000033 window-scoped capture registry, the closed
QUNTAL-P09 structured-UIA registry, the SG-000018 replay-resistant
foundation, SG-000019 class enforcement, SG-000020 trust and revoke
records, and the closed QUNTAL-P08 registry: a single approved
non-actuating proposal shape bound to server-derived frame identity,
expected capture generation, and a bounded region inside the exact
frame geometry, with server-allocated typed proposal identity, fresh
SOFT approval digest binding, immediate pre-proposal stale-frame
revalidation, and protected Quntal approval-surface exclusion.

No other new vision or input authority exists in this grain. Coordinate
proposals, coordinate derivation, bounded input execution, input
leases, mouse input, keyboard input, `SendInput`, focus changes,
monitor scope, desktop scope, clipboard access, network egress, and
elevation have no function in the proposal provider, policy, or
dispatch layer. A missing or denied proposal is never actionable and
must never become coordinate authority; coordinate derivation remains
successor work. No silent escalation exists when the frame is stale,
protected, or unavailable.

## Server-derived identity remains the core control

The proposal shape accepts only `frame_id`,
`expected_capture_generation`, `x`, `y`, `width`, and `height`, where
the region is bounded inside the exact frame geometry.
Caller-supplied frame bytes, proposal identities, region widening
fields, coordinate fields, approval material, secret fields, monitor
scope, desktop scope, model inference fields, and fallback directives
are explicit `CapabilityDenied` or `InvalidRequest` widening denials in
the policy layer. Callers can only present one server-allocated typed
frame identity plus its expected capture generation and bounded
region.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`. Empty,
oversized, frame-external, and overflowing regions fail closed as
`InvalidRequest` before any approval is bound.

## Proposal identity and stale protection

Every successful proposal mints a server-allocated typed proposal
identity bound to the exact frame, proposal generation, bounded
region, workspace, and policy revision. Proposals are evidence for
successor coordinate grains and never grant input execution
authority. Replayed, foreign, generation-drifted, and policy-drifted
proposals fail closed as stale through the read-only proposal check,
and a proposal whose owning frame, window, or process drifted is stale
and never actionable again. Evidence carries only identities,
generations, the bounded region, frame geometry, and the approval
record. Proposal regions are evidence geometry, not screen input
coordinates: no execution path consumes a region without a successor
grain authorizing it.

## Approval binding

Every proposal carries fresh per-action SOFT approval with one-shot
exact-digest binding over workspace, policy revision, frame identity,
capture generation, owning window, window generation, owning process,
process generation, bounded region, and action material. The digest is
computed from the server-resolved binding immediately before
approval, approval is consumed one-shot with nonce and expiry, and
dispatch revalidates the same binding set immediately before minting.
Any material drift after approval fails closed. No caller-supplied
approval boolean is accepted, no persistent reuse exists, and no
remote delegation exists.

## Stale fail-closed behavior

Replayed frames, generation drift, replaced windows, superseded
processes, wrong pairings, protected surfaces, workspace changes,
region widening, and policy revision drift all fail closed without
silent retargeting. Oversized proposals fail closed with no silent
region expansion, and the result explicitly reports that no truncation
occurred. A successful proposal never mutates frame, window, tree, or
process state because proposing is non-actuating; frame and element
identities are unaffected.

## Protected Quntal surfaces and secrets

Proposals can never target approval dialogs, STRONG presence surfaces,
workspace trust controls, emergency revoke, or security-sensitive Quntal
UI, because frames of protected surfaces can never be captured and
direct proposal paths re-check protected status. Fail closed on
uncertainty. No password, credential, Windows Hello, cookie, token, or
session material enters prompts beyond the bounded request itself,
and records, history, logs, MCP responses, snapshots, and evidence
packets carry only digests and metadata, never secret bytes.

## Windows qualification and honest limits

Proposals are pure registry computations over already-captured frame
metadata, so no live desktop surface is required and no pixels are
fabricated. The native adapter continues to prove real process
identity against Windows APIs on Windows, and deterministic proposal
binding, proposal identity, stale protection, region confinement,
approval digest shape, and protected-surface behavior is proven
through the injected fake adapter on every platform. These limits are
recorded honestly and must not be read as interactive desktop
evidence.

## Retained behavior

All SG-000033 capture behavior, SG-000027 observation behavior,
SG-000028 invoke behavior, SG-000029 value behavior, SG-000030 select
behavior, SG-000031 toggle behavior, SG-000032 scroll behavior,
SG-000018 one-shot, expiry, digest, and history behavior, SG-000019
SOFT and STRONG class enforcement, SG-000020 trust and revoke
behavior, and the closed P08 registry behavior are retained
unchanged. The existing `%SystemRoot%\System32\whoami.exe` spawn
boundary is unchanged. No MCP visual or proposal tool exists, and the
agent cannot reach proposal capability through its own tool or input
surface. Coordinate proposals, bounded input execution with input
leases, and human interruption remain successor work.
