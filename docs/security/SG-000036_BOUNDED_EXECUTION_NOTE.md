# SG-000036 - Bounded coordinate execution security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P10
Grain: SG-000036

## Purpose

SG-000036 establishes the fourth narrow QDRAL-P10 vision grain on top
of the closed SG-000035 coordinate registry, the closed SG-000034
visual proposal registry, the closed SG-000033 window-scoped capture
registry, the closed QDRAL-P09 structured-UIA registry, the SG-000018
replay-resistant foundation, SG-000019 class enforcement, SG-000020
trust and revoke records, and the closed QDRAL-P08 registry: a single
approved bounded execution shape bound to server-derived coordinate
identity and expected derivation generation with click-only
operation, an explicit server-derived single-use input lease, fresh
SOFT approval digest binding including the lease, immediate
pre-execution stale-coordinate revalidation, window-confined
actuation, and protected Qdral approval-surface exclusion.

This is the strongest P10 authority and is treated accordingly: one
fixed operation, one exact window, one exact coordinate, one lease,
one approval, one execution. No keyboard, drag, multi-click, raw
synthetic input, lease reuse, standing session, monitor or desktop
scope, clipboard, network, or elevation exists in the execution
provider, policy, or dispatch layer. A missing or denied lease is
never execution authority; human interruption remains successor
work.

## Server-derived identity remains the core control

The execution shape accepts only `coord_id`,
`expected_derivation_generation`, and `operation`, where operation is
fixed to `click`. Caller-supplied coordinates, window handles, lease
identities, approval material, secret fields, keyboard and drag
fields, raw input fields, and fallback directives are explicit
`CapabilityDenied` or `InvalidRequest` widening denials in the policy
layer. The raw `uia.input/keyboard`, `uia.input/mouse`, and
`uia.input/sendinput` shapes stay denied. Callers can only present
one server-allocated typed coordinate identity plus its expected
derivation generation and the fixed operation.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`. Non-click
operations fail closed as `CapabilityDenied` before any lease is
minted.

## Explicit single-use input lease

Every execution carries an explicit server-derived single-use input
lease minted from the fully validated coordinate binding. The lease
binds the exact coordinate identity, derivation generation, owning
proposal, frame, window, process, coordinates, click-only operation,
window handle, expiry, workspace, and policy revision. The lease id
is bound into the approval digest, so approval granted for one lease
can never authorize another. The lease is consumed exactly once at
successful execution. Replayed leases fail closed as stale.
Unconsumed leases expire after 60 seconds and are swept; grants
beyond the live-lease bound fail closed, so no standing input
session can accumulate. Leases whose identity chain drifts are
revoked automatically: replaced windows, superseded processes,
regenerated frames, and policy drift all invalidate live leases.
Post-revoke execution is denied by the approval-epoch gate, which is
proven by the SG-000018 one-shot and SG-000020 epoch suites.
Unapproved, unconsumed leases are inert: execution is reachable only
through the approval-gated dispatch path, and no MCP execution tool
exists.

## Approval binding

Every execution carries fresh per-execution SOFT approval with
one-shot exact-digest binding over workspace, policy revision,
coordinate identity, derivation generation, proposal, frame, window,
process, generations, coordinates, operation, lease, and action
material. The digest is computed from the server-resolved binding
immediately before approval, approval is consumed one-shot with
nonce and expiry, dispatch revalidates the entire identity chain
immediately before the adapter call, and the lease is consumed
exactly once on success. Any material drift after approval fails
closed. Adapter failure does not consume the lease, but the approval
is already one-shot consumed, so a retry requires fresh approval. No
caller-supplied approval boolean is accepted, no persistent reuse
exists, and no remote delegation exists.

## Window confinement and stale fail-closed behavior

Execution actuates only the exact owning window handle at the exact
bound coordinates, with no monitor or desktop widening. HWND
ownership is revalidated immediately before the adapter call, so a
reused handle behind a replaced window fails closed. Replay,
expiry, revocation, wrong pairings, protected surfaces, workspace
changes, and policy revision drift all fail closed without silent
retargeting and without keyboard, drag, clipboard, network, or
elevation fallback.

## Protected Qdral surfaces and secrets

Execution can never target approval dialogs, STRONG presence
surfaces, workspace trust controls, emergency revoke, or
security-sensitive Qdral UI, because coordinate identities of
protected surfaces can never exist, execution re-checks protected
status on the owning window, and execution is confined to the exact
owning window. Fail closed on uncertainty. P10 input authority must
never click approve, deny, presence, trust, revoke, or security
configuration surfaces. No password, credential, Windows Hello,
cookie, token, or session material enters prompts beyond the bounded
request itself, and records, history, logs, MCP responses, snapshots,
and evidence packets carry only digests and metadata, never secret
bytes.

## Windows qualification and honest limits

Live desktop execution requires an interactive session broker, which
remains successor work alongside human interruption, so the native
adapter reports execution as unavailable instead of fabricating
actuation. The native adapter continues to prove real process
identity against Windows APIs on Windows, and deterministic lease
grant, single-use consumption, expiry, operation confinement,
approval digest shape, chain revalidation, and protected-surface
behavior is proven through the injected fake adapter on every
platform. These limits are recorded honestly and must not be read as
interactive desktop actuation evidence.

## Retained behavior

All SG-000035 derivation behavior, SG-000034 proposal behavior,
SG-000033 capture behavior, SG-000027 observation behavior,
SG-000028 invoke behavior, SG-000029 value behavior, SG-000030 select
behavior, SG-000031 toggle behavior, SG-000032 scroll behavior,
SG-000018 one-shot, expiry, digest, and history behavior, SG-000019
SOFT and STRONG class enforcement, SG-000020 trust and revoke
behavior, and the closed P08 registry behavior are retained
unchanged. The existing `%SystemRoot%\System32\whoami.exe` spawn
boundary is unchanged. No MCP input tool exists, and the agent
cannot reach execution capability through its own tool or input
surface. Human interruption remains successor work.
