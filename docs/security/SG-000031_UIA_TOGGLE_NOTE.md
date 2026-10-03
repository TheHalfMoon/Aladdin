# SG-000031 - Structured UIA TogglePattern actuation security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P09
Grain: SG-000031

## Purpose

SG-000031 establishes the fourth narrow QUNTAL-P09 structured actuation
grain on top of the SG-000027 observation registry, the SG-000028 invoke
registry, the SG-000029 value registry, the SG-000030 select registry,
the SG-000018 replay-resistant foundation, SG-000019 class enforcement,
SG-000020 trust and revoke records, and the closed QUNTAL-P08 registry: a
single approved TogglePattern shape bound to server-derived process
identity, typed window identity, typed element identity, expected tree
generation, expected control type, expected Toggle pattern support,
expected enabled state, expected current toggle state, requested target
toggle state, workspace scope, and policy revision, with fresh SOFT
approval digest binding including both toggle states, immediate
pre-actuation stale-target revalidation, and protected Quntal
approval-surface exclusion.

No other new actuation authority exists in this grain. Invoke remains
closed under SG-000028, value remains closed under SG-000029, and select
remains closed under SG-000030. Scroll, focus, keyboard input, mouse
input, `SendInput`, Invoke substitution, click, Space, and Enter
substitution, coordinate requests, screenshots, clipboard access, network
egress, and elevation have no function in the toggle provider, policy, or
dispatch layer. A missing or denied UIA element is reported as stale or
denied and must never become coordinate authority; coordinate fallback is
P10 work. No Invoke, click, Space, Enter, keyboard, or mouse fallback
exists when Toggle is unavailable.

## Server-derived identity remains the core control

The toggle shape accepts only `element_id`,
`expected_tree_generation`, `expected_control_type`,
`expected_toggled`, and `toggled`. Caller-supplied PID, window handle,
UIA runtime id, selector, coordinate, approval material, secret fields,
and fallback directives are explicit `CapabilityDenied` or
`InvalidRequest` widening denials in the policy layer. Callers can only
present one server-allocated typed element identity plus its expected
generation, expected control type, expected toggle, and requested
toggle. There is no blind inversion: the expected pre-state must match
the live toggle state or the request fails closed.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`.

## Toggle eligibility and data safety

Only `CheckBox` and `RadioButton` control types with `Toggle` pattern
support and enabled state actuate. All other control types, unsupported
patterns, disabled elements, password and secret bearing elements, and
protected Quntal surfaces are denied as `CapabilityDenied`. Expected
current toggle is enforced and any state drift fails closed without
silent retargeting. Evidence carries only identities, generations,
control type, expected toggle, requested toggle, and approval record.

## Approval binding

Every toggle carries fresh per-action SOFT approval with one-shot
exact-digest binding over workspace, policy revision, process identity,
process generation, window identity, window generation, element identity,
tree generation, control type, expected toggle, requested toggle, and
action material. The digest is computed from the server-resolved
binding immediately before approval, approval is consumed one-shot with
nonce and expiry, and dispatch revalidates the same binding set
immediately before the adapter call. Any material drift after approval
fails closed. No caller-supplied approval boolean is accepted, no
persistent reuse exists, and no remote delegation exists.

## Stale fail-closed behavior

PID reuse, process restarts, destroyed windows, reused window handles,
process and window mismatches, disappeared elements, replaced elements,
role-changed elements, regenerated trees, disabled elements, protected
surfaces, workspace changes, toggle-state drift, and policy revision
drift all fail closed without silent retargeting. A successful toggle
advances the owning window tree generation and removes its elements so
stale identities cannot be replayed.

## Protected Quntal surfaces and secrets

Toggle can never target approval dialogs, STRONG presence surfaces,
workspace trust controls, emergency revoke, or security-sensitive Quntal
UI. Password and secret bearing elements are denied as targets, and
current secret values never enter records, history, logs, MCP responses,
snapshots, or evidence packets. Fail closed on uncertainty.

## Windows qualification and honest limits

The native adapter proves real process identity against Windows APIs on
Windows: the real PID, the real executable digest, the real session
identity, and the real process creation time as the start generation.
Live desktop toggle requires an interactive session broker, which remains
successor work, so the native adapter reports toggle as unavailable
instead of fabricating actuation. Deterministic toggle binding, stale
protection, eligibility, expected-state enforcement, approval digest
shape, and protected-surface behavior is proven through the injected
fake adapter on every platform. These limits are recorded honestly and
must not be read as interactive desktop actuation evidence.

## Retained behavior

All SG-000027 observation behavior, SG-000028 invoke behavior, SG-000029
value behavior, SG-000030 select behavior, SG-000018 one-shot, expiry,
digest, and history behavior, SG-000019 SOFT and STRONG class
enforcement, SG-000020 trust and revoke behavior, and the closed P08
registry behavior are retained unchanged. The existing
`%SystemRoot%\System32\whoami.exe` spawn boundary is unchanged. No MCP
UIA tool exists, and the agent cannot reach UIA capability through its
own tool or input surface. Scroll pattern remains successor work.
