# SG-000096 T02-C — Synthetic per-action SOFT approval gate

Status: test-only, unarmed. Parent issues #271 and #264; program #260.

## Canonical predecessor
Branch based directly on `main` `1b9919ff95c26e47712b21c1a411c5bfbc88dc73`, after post-main CI `37860297680` succeeded 9/9. No completed work repeated.

## Test-only scope
The only new executable logic is under `#[cfg(test)]` in the Windows process provider. It uses the existing `qdral-approval` `test-support` feature as a development dependency, providing a fixed synthetic SOFT broker. It never displays a human approval UI, mints real STRONG user-presence authority, executes a Full User process, or opens the claimed executable or cwd. No child is spawned in this fixture.

The test verifies a disposable MAC-authenticated Full User lease against actual Windows SID/logon LUID, derived device identity, current owner creation generation, session, policy revision, epoch and expiry. It creates a typed proposed `ShellSessionIntent`, computes its domain-separated digest, and validates the entire independently retained `ExpectedShellBinding` before requesting any synthetic action token. A synthetic SOFT token is consumed exactly once and replay is rejected. Even after that consumption, `authorize_shell_session_t01` MUST continue to deny ShellProcess.

## Adversarial matrix
- Case 0: valid synthetic SOFT consume, replay rejection, T01 disabled.
- Cases 1, 2, 7 and 10: argv change, operation nonce drift, operation-kind change and child-generation drift reject before token consumption.
- Cases 3, 4, 5, 8 and 9: wrong workspace, expired token, attempted STRONG class upgrade, wrong digest and forged token nonce deny.
- Case 6: the synthetic SOFT broker must not issue STRONG approval.

## Limitations
`FixedApprovalBroker` is only artificial test support, NOT a real `LocalApprovalBroker` or genuine presence proof. The temporary HMAC key and session record are synthetic, not the actual protected installed runtime authority. Executable path and SHA-256 values inside the test intent are proposal bytes only, NOT native file-ID/hash proof. This test does not qualify local mediation, concurrent durable approval consumption across restart, actual release admission, executable/cwd handle race safety, output limits, cancellation, T03 sessions, T04 local MCP, or T05 adversarial closeout. Full User ShellProcess is UNARMED and may have ambient filesystem/network privileges; a Job Object is process lifecycle supervision, not a sandbox.

No donor source, privileged/remote shell route, public MCP command, hosted service, installer, release or new external dependency is added.

## Qualification
Windows SG96 native tests, full provider regression, lifecycle release-blocker tests, cargo fmt/Clippy, whitespace and Graft wiring graph must be observed. Deep semantic Graft is not automatically qualified. GitHub exact-head CI 9/9, genuine Jev and Alibaba OCR plus manual excluded-file inspection, signed+DCO, zero unresolved threads, normal merge, and post-main 9/9 remain mandatory; do not claim them before verification.
