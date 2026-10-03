# SG-000032 - Structured UIA ScrollPattern actuation security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P09
Grain: SG-000032

## Purpose

SG-000032 establishes the fifth and final narrow QUNTAL-P09 structured
actuation grain on top of the SG-000027 observation registry, the
SG-000028 invoke registry, the SG-000029 value registry, the SG-000030
select registry, the SG-000031 toggle registry, the SG-000018
replay-resistant foundation, SG-000019 class enforcement, SG-000020 trust
and revoke records, and the closed QUNTAL-P08 registry: a single approved
ScrollPattern shape bound to server-derived process identity, typed
window identity, typed element identity, expected tree generation,
expected control type, expected Scroll pattern support, expected enabled
state, expected current scroll position, bounded requested direction,
bounded requested amount, workspace scope, and policy revision, with
fresh SOFT approval digest binding including scroll position, direction,
and amount, immediate pre-actuation stale-target revalidation, and
protected Quntal approval-surface exclusion.

No other new actuation authority exists in this grain. Invoke remains
closed under SG-000028, value remains closed under SG-000029, select
remains closed under SG-000030, and toggle remains closed under
SG-000031. Focus, keyboard input including PageUp, PageDown, Home, End,
and arrow keys, mouse input including wheel, touch simulation, gesture
synthesis, `SendInput`, coordinate requests, screenshots, clipboard
access, network egress, and elevation have no function in the scroll
provider, policy, or dispatch layer. A missing or denied UIA element is
reported as stale or denied and must never become coordinate authority;
coordinate fallback is P10 work. No wheel, touch, keyboard-paging, or
mouse fallback exists when Scroll is unavailable, and no repeat count or
indefinite scrolling exists: at most one bounded scroll executes per
approval.

## Server-derived identity remains the core control

The scroll shape accepts only `element_id`,
`expected_tree_generation`, `expected_control_type`, `direction`,
`amount`, `expected_horizontal_percent`, and
`expected_vertical_percent`. Caller-supplied PID, window handle, UIA
runtime id, selector, coordinate, repeat count, approval material,
secret fields, and fallback directives are explicit `CapabilityDenied`
or `InvalidRequest` widening denials in the policy layer. Direction is
limited to `up`, `down`, `left`, and `right`; amount is bounded between
1 and 100; expected scroll positions are bounded between 0 and 100.
Callers can only present one server-allocated typed element identity
plus its expected generation, expected control type, bounded direction,
bounded amount, and expected scroll position.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`.
Out-of-range directions, amounts, and positions fail closed as
`InvalidRequest` before any approval is bound.

## Scroll eligibility and data safety

Only `ScrollBar`, `Pane`, `List`, and `Tree` control types with `Scroll`
pattern support and enabled state actuate. All other control types,
unsupported patterns, disabled elements, password and secret bearing
elements, and protected Quntal surfaces are denied as `CapabilityDenied`.
Expected current scroll position is enforced and any position drift
fails closed without silent retargeting. Evidence carries only
identities, generations, control type, expected scroll position,
requested direction, requested amount, and approval record.

## Approval binding

Every scroll carries fresh per-action SOFT approval with one-shot
exact-digest binding over workspace, policy revision, process identity,
process generation, window identity, window generation, element identity,
tree generation, control type, expected scroll position, requested
direction, requested amount, and action material. The digest is computed
from the server-resolved binding immediately before approval, approval
is consumed one-shot with nonce and expiry, and dispatch revalidates
the same binding set immediately before the adapter call. Any material
drift after approval fails closed. No caller-supplied approval boolean
is accepted, no persistent reuse exists, and no remote delegation
exists.

## Stale fail-closed behavior

PID reuse, process restarts, destroyed windows, reused window handles,
process and window mismatches, disappeared elements, replaced elements,
role-changed elements, regenerated trees, disabled elements, protected
surfaces, workspace changes, scroll-position drift, and policy revision
drift all fail closed without silent retargeting. A successful scroll
advances the owning window tree generation and removes its elements so
stale identities cannot be replayed.

## Protected Quntal surfaces and secrets

Scroll can never target approval dialogs, STRONG presence surfaces,
workspace trust controls, emergency revoke, or security-sensitive Quntal
UI. Password and secret bearing elements are denied as targets, and
current secret values never enter records, history, logs, MCP responses,
snapshots, or evidence packets. Fail closed on uncertainty.

## Windows qualification and honest limits

The native adapter proves real process identity against Windows APIs on
Windows: the real PID, the real executable digest, the real session
identity, and the real process creation time as the start generation.
Live desktop scroll requires an interactive session broker, which
remains successor work, so the native adapter reports scroll as
unavailable instead of fabricating actuation. Deterministic scroll
binding, stale protection, eligibility, direction and amount bounds,
expected-position enforcement, approval digest shape, and
protected-surface behavior is proven through the injected fake adapter
on every platform. These limits are recorded honestly and must not be
read as interactive desktop actuation evidence.

## Retained behavior

All SG-000027 observation behavior, SG-000028 invoke behavior, SG-000029
value behavior, SG-000030 select behavior, SG-000031 toggle behavior,
SG-000018 one-shot, expiry, digest, and history behavior, SG-000019 SOFT
and STRONG class enforcement, SG-000020 trust and revoke behavior, and
the closed P08 registry behavior are retained unchanged. The existing
`%SystemRoot%\System32\whoami.exe` spawn boundary is unchanged. No MCP
UIA tool exists, and the agent cannot reach UIA capability through its
own tool or input surface. No further P09 actuation pattern remains
after scroll; P09 exit is successor work.
