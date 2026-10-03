# SG-000033 - Bounded window-scoped screenshot capture security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P10
Grain: SG-000033

## Purpose

SG-000033 establishes the first narrow QDRAL-P10 vision grain on top of
the closed QDRAL-P09 structured-UIA registry, the SG-000018
replay-resistant foundation, SG-000019 class enforcement, SG-000020 trust
and revoke records, and the closed QDRAL-P08 registry: a single approved
window-scoped capture shape bound to server-derived process identity,
typed window identity, expected window generation, fixed target-window
scope, workspace scope, and policy revision, with fresh SOFT approval
digest binding, server-allocated typed frame identity with capture
generation and geometry, immediate pre-capture stale-target
revalidation, and protected Qdral approval-surface exclusion.

No other new vision or input authority exists in this grain. Visual
target proposals, coordinate proposals, bounded input execution, input
leases, monitor scope, desktop scope, caller-selected regions, mouse
input, keyboard input, `SendInput`, focus changes, clipboard access,
network egress, and elevation have no function in the capture provider,
policy, or dispatch layer. A missing or denied frame is never actionable
and must never become coordinate authority; coordinate fallback remains
successor work. No silent escalation exists when the target window is
stale, protected, or unavailable.

## Server-derived identity remains the core control

The capture shape accepts only `window_id`,
`expected_window_generation`, and `scope`, where scope is fixed to
`target-window`. Caller-supplied PID, window handle, frame identity,
geometry, approval material, secret fields, monitor scope, desktop
scope, region selectors, visual proposal fields, coordinate fields, and
fallback directives are explicit `CapabilityDenied` or `InvalidRequest`
widening denials in the policy layer. Callers can only present one
server-allocated typed window identity plus its expected generation and
the fixed scope.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`. Widened scopes
fail closed as `CapabilityDenied` before any adapter access.

## Frame identity and stale-frame protection

Every successful capture mints a server-allocated typed frame identity
bound to the exact window, capture generation, geometry, payload
digest, workspace, and policy revision. Frames are evidence for
successor visual grains and never grant input execution authority.
Replayed, foreign, generation-drifted, and policy-drifted frames fail
closed as stale through the read-only frame check, and a frame whose
owning window or process drifted is stale and never actionable again.
Evidence carries only identities, generations, scope, pixel format,
geometry, payload digest, the bounded pixel payload, and the approval
record.

## Approval binding

Every capture carries fresh per-action SOFT approval with one-shot
exact-digest binding over workspace, policy revision, process identity,
process generation, window identity, window generation, capture scope,
and action material. The digest is computed from the server-resolved
binding immediately before approval, approval is consumed one-shot with
nonce and expiry, and dispatch revalidates the same binding set
immediately before the adapter call. Any material drift after approval
fails closed, including process restarts that advance the process
generation. No caller-supplied approval boolean is accepted, no
persistent reuse exists, and no remote delegation exists.

## Stale fail-closed behavior

PID reuse, process restarts, destroyed windows, reused window handles,
process and window mismatches, protected surfaces, workspace changes,
scope widening, oversized payloads, geometry mismatches, and policy
revision drift all fail closed without silent retargeting. Oversized
captures fail closed with no silent downscaling that changes evidence
semantics, and the result explicitly reports that no truncation
occurred. A successful capture never advances the window tree
generation because capture is read-only; element identities are
unaffected.

## Protected Qdral surfaces and secrets

Capture can never target approval dialogs, STRONG presence surfaces,
workspace trust controls, emergency revoke, or security-sensitive Qdral
UI. Protected surfaces are omitted from window listings, denied on
direct observation, and denied on capture. Fail closed on uncertainty.
No password, credential, Windows Hello, cookie, token, or session
material enters prompts beyond the bounded request itself, and records,
history, logs, MCP responses, snapshots, and evidence packets carry
only digests and metadata, never secret bytes.

## Windows qualification and honest limits

The native adapter proves real process identity against Windows APIs on
Windows: the real PID, the real executable digest, the real session
identity, and the real process creation time as the start generation.
Live desktop capture requires an interactive session broker, which
remains successor work, so the native adapter reports capture as
unavailable instead of fabricating pixels. Deterministic capture
binding, frame identity, stale protection, scope confinement, payload
bounds, approval digest shape, and protected-surface behavior is proven
through the injected fake adapter on every platform. These limits are
recorded honestly and must not be read as interactive desktop capture
evidence.

## Retained behavior

All SG-000027 observation behavior, SG-000028 invoke behavior, SG-000029
value behavior, SG-000030 select behavior, SG-000031 toggle behavior,
SG-000032 scroll behavior, SG-000018 one-shot, expiry, digest, and
history behavior, SG-000019 SOFT and STRONG class enforcement, SG-000020
trust and revoke behavior, and the closed P08 registry behavior are
retained unchanged. The existing `%SystemRoot%\System32\whoami.exe`
spawn boundary is unchanged. No MCP UIA or screenshot tool exists, and
the agent cannot reach capture capability through its own tool or input
surface. Visual proposals, coordinate proposals, bounded input
execution, and human interruption remain successor work.
