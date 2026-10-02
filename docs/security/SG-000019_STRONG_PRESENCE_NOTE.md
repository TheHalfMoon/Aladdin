# SG-000019 — Strong user-presence approval class security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P07
Grain: SG-000019

## Purpose

SG-000019 introduces the STRONG approval class on top of the SG-000018
replay-resistant foundation. PRIVILEGED operations and selected DESTRUCTIVE
operations map to STRONG and require platform-mediated local user-presence
verification. Existing file, process, and Git operations retain the SOFT
class with digests, prompts, revalidation, and history behavior unchanged
apart from the class label and policy revision.

No trust changes, persistent approval reuse, remote delegation, new
privileged capabilities, new network, browser, UI automation, clipboard,
installer, elevation, or synthetic-input authority are introduced.

## SOFT versus STRONG contract

The broker contract distinguishes two explicit classes:

- SOFT: local explicit approval through the existing Windows system-modal
  button. Used for lower-risk writes and execution that already require
  approval.
- STRONG: platform-mediated presence verification bound to the existing
  nonce, expiry, one-shot, digest, workspace, and policy bindings. Used for
  PRIVILEGED and selected DESTRUCTIVE operations.

A soft button alone never satisfies STRONG. The broker branches on the
prompt class before any platform call. STRONG never calls the SOFT button
path. Consume verifies that the recorded class, presented token class, and
expected class all match. Any class drift fails closed as stale state.
STRONG never silently downgrades to SOFT.

Absence of the strong mechanism fails closed. Cancellation fails closed.
Timeout fails closed. Presence failure fails closed. Malformed presence
results fail closed. Replay fails closed. Stale requests fail closed.
Approval drift, workspace drift, action drift, and scope drift fail closed.

## Platform verification boundary

Presence verification uses a typed local adapter seam:

- `PresenceVerifier` with capability detection through `is_available`
  and bounded verification through `verify`.
- `PresenceContext` carries only nonce, digest, workspace, and action.
  No secrets, biometric material, PINs, or credential material enter the
  context.
- `PresenceAttestation` carries only method, verification time, nonce,
  and digest. No secrets enter the attestation, ledger, history, logs,
  MCP responses, or evidence packets.
- `InputLeaseGuard` proves that the global automation input lease is
  suspended during STRONG verification. The guard is created only inside
  the broker crate. External callers cannot synthesize the guard and
  cannot self-produce presence proof.
- The broker holds the verifier privately. No MCP tool can produce,
  approve, or replay an approval. The policy engine denies approve-like
  capabilities through the request surface. The agent cannot state
  `strong_presence = true` through any public API. Only the verifier
  result bound to the exact nonce and digest can satisfy STRONG.

Windows Hello style verification is the target where feasible:

- On Windows, `WindowsHelloPresenceVerifier` checks
  `UserConsentVerifier::CheckAvailabilityAsync` without showing UI and
  verifies through `RequestVerificationAsync` with a bounded message
  containing only a nonce prefix, digest prefix, action, and workspace.
- The verification runs with a bounded timeout. Timeout fails closed.
- In non-interactive sessions including CI, availability reports false
  and STRONG fails closed without showing UI and without claiming
  unverified success.
- On non-Windows platforms, the default verifier is unavailable and
  STRONG fails closed.

No Windows Hello qualification is claimed from mocks. Deterministic tests
use `TestPresenceVerifier`. The single Windows-gated test exercises the
real adapter only to report availability explicitly or to prove
fail-closed unavailability. It never asserts verified success in CI.

Sensitive OS material never enters MCP responses, audit history, logs,
PR descriptions, CI logs, or evidence packets. Verification status only
is recorded as `verified-strong`, `denied`, or `unavailable` with the
method name.

## Policy classification

`approval_class_for` maps capability and operation to a class without
granting new authority:

- PRIVILEGED shapes including `policy.change`, `workspace.trust_change`,
  `service.install`, `credential_binding_change`, and `elevation.request`
  map to STRONG and remain denied in this grain.
- Selected DESTRUCTIVE shapes including `fs.delete/delete`,
  `git.reset/reset_hard`, `process.kill/kill_tree`,
  `browser.clear/clear_profile`, and `git.push/force-push` map to STRONG
  and remain denied in this grain.
- Existing file, process, Git mutation, fetch, and push shapes retain
  SOFT.

Authorization still denies PRIVILEGED and selected DESTRUCTIVE shapes.
Classification does not widen authority. The quntald dispatch layer
additionally denies any STRONG-class request fail-closed, so no STRONG
execution path is authorized in this grain. STRONG success is proven at
the broker layer with test verifiers, not by granting new production
capabilities.

## History and redaction

The ledger schema is `quntal-approval-ledger-v2`. First run with this
grain creates a new ledger. Version one records are not read and do not
authorize execution. Records carry class,
presence outcome, and presence method in addition to the SG-000018
fields. The checksum chain covers the new fields. History queries return
redacted summaries with class and presence but without summaries,
targets, secrets, or biometric material.

Tampering with class or presence invalidates the checksum chain from the
tampered record onward. Loading stops at the first invalid line
fail-closed. Consumption persistence failures fail the operation closed
so a retry requires a fresh approval.

## Separation

- No MCP approve tool exists.
- The kernel denies approve-like capabilities.
- Agent-spawned children receive no approval IPC handles, nonces, or
  secrets and cannot forge approvals.
- Synthetic input, UI automation, coordinate proposals, and child
  processes cannot satisfy STRONG because only the broker-held verifier
  can produce a bound attestation.
- Replayed prior approvals fail as replay through one-shot consumption.
- Expired approvals fail closed.
- Digest mismatch fails closed.
- Workspace and policy revision drift fail closed.
- Double consumption fails closed.
- Wrong nonce fails closed.
- Forged tokens referencing unknown records fail closed.
- Forged history fails checksum verification.

## Retained behavior

All SG-000015 local mutation, SG-000016 fetch, SG-000017 push, and
SG-000018 nonce, expiry, one-shot, digest, workspace, policy, and
history behaviors are retained. P06 approval flows still require fresh
approval and pass regressions with digest construction unchanged apart
from the policy revision and class label. Denied and unavailable
approvals are recorded and never authorize execution.

## Explicitly denied

Denied in this grain: workspace trust creation, modification, or
revocation; approval history UX beyond recorded class and presence
fields; emergency revoke broadcast; changes to per-operation digests,
prompts, or summaries outside class labeling; new privileged
capabilities or capability families; new network, browser, UI
automation, clipboard, or installer authority; persistent approval reuse
across operations; approval delegation to remote or MCP callers;
synthetic-input generation or input injection; elevation.
