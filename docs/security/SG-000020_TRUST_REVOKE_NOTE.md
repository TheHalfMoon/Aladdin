# SG-000020 — Workspace trust with history UX and emergency revoke security note

Status: IMPLEMENTATION CANDIDATE
Program: QDRAL-P07
Grain: SG-000020

## Purpose

SG-000020 adds workspace trust management, class-labeled history UX, and
emergency revoke on the SG-000018 replay-resistant foundation and SG-000019
STRONG enforcement. Trust changes are explicit, auditable, and STRONG-gated
with fail-closed revocation, without weakening one-shot, expiry, digest
binding, or STRONG presence.

No new network, browser, UI automation, clipboard, installer, elevation,
persistent reuse, remote delegation, or synthetic-input authority is
introduced. New privileged capability is workspace trust management only,
through dedicated PRIVILEGED APIs with STRONG presence.

## Trust model

Trust state is per-workspace records in Qdral protected local state with
a checksum chain over workspace identity, trust flag, revision,
provenance method, timestamp, and policy revision. Unknown workspaces
report untrusted fail-closed. Corrupt or missing state reports untrusted
fail-closed and never authorizes trust.

Trust grant sets trusted true. Trust revoke sets trusted false. Each
change advances a per-workspace revision. History queries return bounded
redacted entries without secrets.

The agent cannot declare trust by request alone. The capability itself
selects grant or revoke with no caller-supplied trust boolean. Trust
changes require a fresh STRONG approval bound to workspace, requested
transition, current trust state and revision, policy revision, and exact
digest. State drift between approval and application fails closed.

## STRONG gating

Trust grant, trust revoke, and emergency revoke require the STRONG class
with platform-mediated presence through the broker-held verifier and a
suspended input lease. A soft button never satisfies these actions.
Unverified, cancelled, timed-out, malformed, or unavailable presence
fails closed without downgrade.

Provenance records the verifier method only. No biometric material, PIN,
secret, or credential enters prompts, records, history, logs, MCP
responses, or evidence packets.

## Emergency revoke

Emergency revoke advances a global approval epoch through a STRONG
presence check. All tokens issued before the revoke carry the old epoch
and fail closed on consume. Revoke records are checksum-chained,
tamper-evident, and never authorize execution. Tokens issued after the
revoke carry the new epoch and remain usable with fresh approval.

Revoke requires STRONG presence, so the agent cannot silently revoke to
deny service and cannot silently undo revoke. Use after revoke fails
closed. Replay of revoke events cannot authorize operations.

## History UX

Approval history queries return bounded redacted records with class,
presence outcome, method, epoch, and revoke labels. Trust history
queries return bounded redacted trust transitions. Both reject tampered
records by checksum failure and stop at the first invalid line
fail-closed. Limits are bounded between 1 and 200. No secrets,
biometric material, or unrelated user data are exposed.

Class labels come from trusted enforcement records, never from caller
claims. A WEAK approval never appears as STRONG.

## Separation

No MCP trust-approve tool exists. The policy engine authorizes only the
typed trust and history shapes and denies approve-like, legacy
privileged, and destructive shapes. Agent-spawned children receive no
approval handles, nonces, epochs, or secrets. Synthetic input, UI
automation, coordinate proposals, and child processes cannot satisfy
trust changes or revoke.

## Retained behavior

All SG-000018 nonce, expiry, one-shot, digest, workspace, policy, and
history behaviors are retained with ledger schema
`qdral-approval-ledger-v3` covering epoch and revoke flags. All SG-000019
STRONG enforcement, fail-closed presence, and class-labeled history are
retained. P06 file, process, and Git flows keep exact digests and
revalidation outside trust labeling. Denied and unavailable trust and
revoke decisions are recorded and never authorize execution.

## Explicitly denied

Denied in this grain: privileged capabilities outside workspace trust
management; per-operation digest, prompt, or summary changes outside
trust and revoke labeling; new network, browser, UI automation,
clipboard, or installer authority; persistent approval reuse; approval
delegation to remote or MCP callers; synthetic-input generation or input
injection; elevation.
