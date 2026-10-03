# SG-000018 — Approval history with replay resistance security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P07
Grain: SG-000018

## Purpose

SG-000018 introduces the first QUNTAL-P07 strong-approval foundation. It does
not add strong user-presence verification, trust changes, or new privileged
authority. It hardens the existing approval broker so that a previous
approval can never authorize a changed operation and an agent cannot forge or
replay approvals through its own tool or process surface.

Every recorded approval binds a fresh nonce, the exact normalized operation
digest, the workspace identity, and the policy revision, and carries a short
expiry with single-operation reuse scope. Consumption verifies all bindings
and marks the nonce spent; any second consumption, digest change, workspace
or policy drift, or expiry fails closed.

## Nonce and one-shot semantics

Nonces are freshly derived per approval request from process identity, clock,
an atomic counter, and the operation digest, and the ledger rejects duplicate
issuance fail-closed. The ledger tracks spent nonces in memory and persists
consumption markers to the append-only history file, so restarts cannot
resurrect a spent approval.

## Expiry and drift

Approvals expire five minutes after issuance. Consumption compares the caller
clock against both the recorded and presented expiry and fails closed when
expired. Workspace identity and policy revision are verified on both the
recorded and presented sides; any drift fails closed as stale state rather
than as a generic error, preserving the existing typed failure discipline.

## History and redaction

The history file stores approval id, nonce, digest, workspace, policy
revision, decision, timestamps, expiry, reuse scope, consumption state, and a
checksum chain over the previous record. It never stores prompt summaries or
targets, which may carry caller data, and it never stores secret material.
 Bounded queries return the most recent redacted summaries. A tampered prefix
invalidates the chain from the tampered record onward, and loading stops at
the first invalid line fail-closed. Consumption persistence failures fail the
operation closed so a retry requires a fresh approval.

Known limitation carried explicitly: the file ledger detects tampering and
stops at the first invalid line, but silent tail truncation of Quntal
protected local state by a party with direct filesystem write access to that
state is not independently detected on reload. Restart replay therefore
relies on the per-user privacy of Quntal protected state. In-process one-shot
enforcement, which is the primary replay boundary for every dispatch path in
this grain, is unaffected.

## Separation

No MCP tool can produce, approve, or replay an approval. The policy engine
explicitly denies approve-like capabilities and operations through the
request surface, and the kernel never exposes an approve entry point.
Agent-spawned child processes receive no approval IPC handles, nonces, or
secrets through the existing sanitized execution environment, and the broker
contract requires an exact digest, workspace, and policy match that a child
cannot satisfy without the unconsumed token held only in quntald memory.

## Retained behavior

All existing fresh-approval flows for file writes, argv process execution,
Git branch, stage, unstage, commit, fetch preview and fetch, and push preview
and push keep their exact digests, prompts, and post-approval revalidation.
The broker hardening adds nonce, expiry, policy binding, token consumption,
and recording around those unchanged flows. Denied and unavailable approvals
are recorded as such and never authorize execution.

## Explicitly denied

Denied in this grain: strong user-presence approval through Windows Hello,
WebAuthn, or equivalent platform verification; workspace trust creation,
modification, or revocation; emergency revoke broadcast beyond recorded
history; persistent approval reuse across operations; approval delegation to
remote or MCP callers; raw secret storage in approval records; new privileged
capabilities; new network, browser, UI automation, clipboard, installer, or
elevation authority; and approval bypass.
