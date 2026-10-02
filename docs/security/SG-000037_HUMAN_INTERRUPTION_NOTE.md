# SG-000037 - Human-interruption invalidation security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P10
Grain: SG-000037

## Purpose

SG-000037 establishes the fifth narrow QUNTAL-P10 vision grain on top
of the closed SG-000036 bounded execution registry, the closed
SG-000035 coordinate registry, the closed SG-000034 visual proposal
registry, the closed SG-000033 window-scoped capture registry, the
closed QUNTAL-P09 structured-UIA registry, the SG-000018
replay-resistant foundation, SG-000019 class enforcement, SG-000020
trust and revoke records, and the closed QUNTAL-P08 registry:
human-interruption handling bound to the explicit single-use input
lease through a monotonic interruption epoch, where any material
human physical interaction revokes live lease material so old leases,
pending approvals bound to old epochs, and queued continuations fail
closed.

This grain adds no actuating input authority. It only revokes. Human
physical interaction always wins over Quntal automation: the system
never fights the user for control, never reclaims mouse position
against the user, never refocuses windows against the user, never
replays an interrupted action, and never continues queued, drag, or
keystroke sequences after a human override without a fresh lease and
fresh approval.

## Monotonic interruption epoch

The registry holds one monotonic interruption epoch starting at zero.
Only the explicit human-interruption report increments it, exactly
once per valid report with a bounded physical-source reason of
`human-keyboard`, `human-mouse-move`, `human-mouse-button`,
`human-touch-pen`, `human-foreground-change`, `human-presence`,
`emergency-stop`, or `approval-pending-suspension`. Empty, oversized,
and unknown reasons fail closed as `InvalidRequest` without mutating
the epoch. The epoch never decrements, resets, or wraps, so a lease
granted before an interruption can never become valid again.

## Lease and approval epoch binding

Every input lease minted after this grain records the current epoch,
and every execution revalidates the lease epoch against the live
registry epoch immediately before actuation. Epoch drift fails closed
as `TargetStale` with no silent retargeting. The bounded execution
approval digest binds the epoch alongside the coordinate identity,
generations, coordinates, operation, and lease, so any approval minted
before a material human override fails closed after the override.
Fresh continuation requires a fresh lease, fresh proposal validation
where the canonical digest no longer matches, and fresh approval. No
caller-supplied epoch, reason, origin, or injection field is accepted:
leases, epochs, and origins are server-derived, the execution request
shape still accepts only `coord_id`,
`expected_derivation_generation`, and `operation`, and the policy layer
denies epoch and interruption forgery fields explicitly.

## Quntal synthetic exclusion with fail-closed ambiguity

Input-origin classification distinguishes Quntal synthetic execution
(which never interrupts itself and never revokes its own lease), OS
or other synthetic input, real human physical input, and unknown
input. Human physical input always interrupts. Unknown or ambiguous
input fails closed toward interruption, so a forged synthetic-origin
claim cannot suppress a genuine human override: the epoch check in
the execution path has no bypass flag of any kind. The registry never
calls the report path for its own adapter actuation.

## No surveillance, no hooks, no content

Interruption handling captures no input content. There is no
keystroke recording, no pointer-path logging, no screen-content
capture beyond the closed window-scoped shape, no low-level hook
installation, no global hook persistence, no scheduled polling loop,
and no background input logging. Interruption evidence carries only
the epoch, the bounded reason, the report timestamp, the workspace,
and the policy revision. The interactive session broker derives
reports from narrow user-presence signals (last-input timing with
injected-input filtering, foreground and window interaction, and the
emergency-stop path), never from input content. The emergency stop
cannot be suppressed, delayed, or bypassed through any interruption
path.

## Protected Quntal surfaces and secrets

Interruption handling never exposes approval dialogs, STRONG presence
surfaces, workspace trust controls, emergency revoke, or
security-sensitive Quntal UI as automation targets, and protected
surfaces remain denied regardless of epoch state. No password,
credential, Windows Hello, cookie, token, or session material enters
prompts, records, history, logs, MCP responses, snapshots, or
evidence packets. Interruption evidence carries no input content and
no secret material.

## Windows qualification and honest limits

Live desktop interaction requires an interactive session broker, so
headless contexts report no physical input instead of fabricating it.
The native adapter continues to prove real process identity against
Windows APIs on Windows and reports live execution as unavailable
instead of fabricating actuation. Deterministic epoch policy, lease
epoch binding, digest epoch binding, interruption revalidation,
synthetic exclusion with fail-closed ambiguity, no-replay behavior,
and protected-surface retention are proven through the injected fake
adapter on every platform. These limits are recorded honestly and
must not be read as interactive desktop interruption evidence. The
canonical Windows mechanism of last-input timing with
injected-input filtering is documented as the interactive-broker
source for the report shape.

## Retained behavior

All SG-000036 execution behavior, SG-000035 derivation behavior,
SG-000034 proposal behavior, SG-000033 capture behavior, SG-000027
observation behavior, SG-000028 invoke behavior, SG-000029 value
behavior, SG-000030 select behavior, SG-000031 toggle behavior,
SG-000032 scroll behavior, SG-000018 one-shot, expiry, digest, and
history behavior, SG-000019 SOFT and STRONG class enforcement,
SG-000020 trust and revoke behavior, and the closed P08 registry
behavior are retained unchanged. The existing
`%SystemRoot%\System32\whoami.exe` spawn boundary is unchanged. No
MCP input or interruption tool exists, and the agent cannot reach
execution or interruption capability through its own tool or input
surface. Clipboard and network authority remain P11 work.
