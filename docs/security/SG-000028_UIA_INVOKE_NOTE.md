# SG-000028 - Structured UIA InvokePattern actuation security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P09
Grain: SG-000028

## Purpose

SG-000028 establishes the first narrow QDRAL-P09 structured actuation grain
on top of the SG-000027 read-only observation registry, the SG-000018
replay-resistant foundation, SG-000019 class enforcement, SG-000020 trust
and revoke records, and the closed QDRAL-P08 registry: a single approved
InvokePattern shape bound to server-derived process identity, typed window
identity, typed element identity, expected tree generation, expected control
type, expected Invoke pattern support, expected enabled state, workspace
scope, and policy revision, with fresh SOFT approval digest binding,
immediate pre-actuation stale-target revalidation, and protected Qdral
approval-surface exclusion.

No other actuation authority exists in this grain. Value setting, text
entry, select, toggle, scroll, focus, click fallback, keyboard input, mouse
input, `SendInput`, coordinate requests, screenshots, clipboard access,
network egress, and elevation have no function in the provider, the policy,
or the dispatch layer. A missing or denied UIA element is reported as stale
or denied and must never become coordinate authority; coordinate fallback is
P10 work.

## Server-derived identity remains the core control

The invoke shape accepts only `element_id`, `expected_tree_generation`,
and `expected_control_type`. Caller-supplied PID, window handle, UIA runtime
id, selector, coordinate, approval material, secret fields, value fields,
and fallback directives are explicit `CapabilityDenied` or `InvalidRequest`
widening denials in the policy layer. Callers can only present one
server-allocated typed element identity plus its expected generation and
expected control type:

- `uia-el-` plus 16 lowercase hex characters, bound to owning window
  identity, UIA runtime identity, tree generation, control type, workspace,
  and policy revision.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`.

## Invoke eligibility

Only `Button`, `Hyperlink`, `MenuItem`, and `SplitButton` control types
with `Invoke` pattern support and enabled state actuate. All other control
types, unsupported patterns, disabled elements, password and secret bearing
elements, and protected Qdral surfaces are denied as `CapabilityDenied`.
Invoke failure never falls back to mouse, keyboard, `SendInput`,
coordinates, screenshots, or elevation.

## Approval binding

Every invoke carries fresh per-action SOFT approval with one-shot
exact-digest binding over workspace, policy revision, process identity,
process generation, window identity, window generation, element identity,
tree generation, control type, and action material. The digest is computed
from server-resolved binding immediately before approval, approval is
consumed one-shot with nonce and expiry, and dispatch revalidates the same
binding set immediately before the adapter call. Any material drift after
approval fails closed. No caller-supplied approval boolean is accepted, no
persistent reuse exists, and no remote delegation exists.

## Stale fail-closed behavior

PID reuse, process restarts, destroyed windows, reused window handles,
process and window mismatches, disappeared elements, replaced elements,
role-changed elements, regenerated trees, disabled elements, protected
surfaces, workspace changes, and policy revision drift all fail closed
without silent retargeting. A successful invoke advances the owning window
tree generation and removes its elements so stale identities cannot be
replayed.

## Protected Qdral surfaces

Invoke can never target approval dialogs, STRONG presence surfaces,
workspace trust controls, emergency revoke, or security-sensitive Qdral UI.
Protected windows are omitted from listings and denied on direct invoke.
Fail closed on uncertainty. The model must not approve its own actions
through UI Automation.

## Windows qualification and honest limits

The native adapter proves real process identity against Windows APIs on
Windows: the real PID, the real executable digest, the real session
identity, and the real process creation time as the start generation. Live
desktop invoke requires an interactive session broker, which remains
successor work, so the native adapter reports invoke as unavailable instead
of fabricating actuation. Deterministic invoke binding, stale protection,
eligibility, approval digest shape, and protected-surface behavior is proven
through the injected fake adapter on every platform. These limits are
recorded honestly and must not be read as interactive desktop actuation
evidence.

## Retained behavior

All SG-000027 observation behavior, SG-000018 one-shot, expiry, digest, and
history behavior, SG-000019 SOFT and STRONG class enforcement, SG-000020
trust and revoke behavior, and the closed P08 registry behavior are retained
unchanged. The existing `%SystemRoot%\System32\whoami.exe` spawn boundary
is unchanged. No MCP UIA tool exists, and the agent cannot reach UIA
capability through its own tool or input surface. Value, select, toggle,
and scroll patterns remain successor work.
