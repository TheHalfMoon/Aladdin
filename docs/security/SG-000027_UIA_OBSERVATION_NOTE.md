# SG-000027 - Read-only Windows UI Automation observation security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P09
Grain: SG-000027

## Purpose

SG-000027 establishes the first QUNTAL-P09 Windows UI Automation foundation
on top of the SG-000018 replay-resistant foundation, SG-000019 STRONG
enforcement, SG-000020 trust and revoke records, and the closed QUNTAL-P08
structured-browser registry: a read-only UI Automation observation
capability with server-derived process identity, typed window identity,
typed element identity, bounded tree observation, stale-identity
fail-closed behavior, protected Quntal approval-surface exclusion, and
password and secret redaction.

No actuation authority exists in this grain. Invoke, click, value setting,
text entry, select, toggle, scroll, focus, keyboard input, mouse input,
`SendInput`, coordinate requests, screenshots, clipboard access, network
egress, and elevation have no function in the provider, the policy, or the
dispatch layer. A missing UIA element is reported as stale or denied and
must never become coordinate authority; coordinate fallback is P10 work.

## Server-derived identity is the core security control

No UIA shape accepts a PID, window handle, UIA runtime id, selector,
coordinate, approval material, or secret field of any kind. Those fields
are explicit `CapabilityDenied` widening denials in the policy layer, so a
caller can never name the process, window, or element to observe. Callers
can only present server-allocated typed identities:

- `uia-proc-` plus 16 lowercase hex characters, bound to PID, executable
  digest, process generation, session identity, workspace, and policy
  revision;
- `uia-win-` plus 16 lowercase hex characters, bound to owning process
  identity, window handle, window generation, session, desktop, workspace,
  and policy revision;
- `uia-el-` plus 16 lowercase hex characters, bound to owning window
  identity, UIA runtime identity, tree generation, control type, workspace,
  and policy revision.

Well-formed but unknown identities fail closed as `TargetStale`.
Malformed identities fail closed as `InvalidRequest`.

## Stale fail-closed behavior

PID reuse, process restarts, destroyed windows, reused window handles,
process and window mismatches, regenerated trees, disappeared elements,
role-changed elements, workspace changes, and policy revision drift all
fail closed without silent retargeting. A superseded process record can
never be observed through its old identity, a replaced window can never be
observed through its old generation, and an element of a regenerated tree
can never be observed through its old tree generation.

## Bounded observation and redaction

Tree observation is bounded by window count, tree depth, node count,
response size, and string length, and every response carries explicit
truncation reporting instead of silent unlimited scraping. Password and
secret bearing controls are detected by control type, automation id, and
accessible name, and their values are redacted: evidence carries
`redacted` as `true` and no value bytes. Raw values never enter records,
history, logs, MCP responses, snapshots, or evidence packets.

## Protected Quntal surfaces

Windows carrying Quntal approval markers are omitted from window listings
and counted as `protected_omitted`, never returned. Direct observation of
a protected surface fails closed as `CapabilityDenied`, so the agent
cannot inspect trusted approval material in a way that undermines STRONG
presence, approval decisions, trust changes, emergency revoke, or
protected credential handling. Fail closed on ambiguity.

## Windows qualification and honest limits

The native adapter proves real process identity against Windows APIs on
Windows: the real PID, the real executable digest, the real session
identity, and the real process creation time as the start generation.
Live desktop window and tree enumeration requires an interactive session
broker, which is successor work, so the native adapter reports the
desktop as unavailable instead of fabricating windows. Deterministic
observation, stale protection, redaction, bounds, and protected-surface
behavior is proven through the injected fake adapter on every platform.
These limits are recorded honestly and must not be read as interactive
desktop evidence.

## Retained behavior

All SG-000018 one-shot, expiry, digest, and history behavior, SG-000019
STRONG enforcement, SG-000020 trust and revoke behavior, and the closed
P08 registry behavior are retained unchanged. The existing
`%SystemRoot%\System32\whoami.exe` spawn boundary is unchanged. No MCP
UIA tool exists, and the agent cannot reach UIA capability through its own
tool or input surface.
