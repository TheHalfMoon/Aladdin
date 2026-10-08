# SG-000096 T02-A: Suspended-Child Ownership Qualification

Status: IMPLEMENTATION CANDIDATE — not canonical, not the SG-000096 T02 exit.

## Verified scope

- Reuses Deskal's existing Windows `CreateProcessW`, `OwnedHandle`, `ChildProcess`, and `JobObject` primitives in `qdral-provider-process`; no copied donor code or new dependency.
- Centralizes conversion of a newly created suspended process into owned handles. If the thread handle cannot be adopted after successful `CreateProcessW`, explicitly terminate the child and verify a signaled process wait before returning. A failed or unverified termination returns a distinct failure.
- Adds a test-only ordinary current-user suspended process using a fixed Windows system executable. The test verifies the child is not AppContainer and compares its actual TokenUser SID, AuthenticationId logon LUID, session ID and elevation state against the parent token. It assigns the disposable child to Deskal's kill-on-close Job Object before `ResumeThread`, then checks process completion and Job quiescence.
- Adds a test-only missing-thread-handle fault injection after successful suspended creation to exercise termination before any Job assignment. Test-only launch methods are excluded from production builds with `#[cfg(test)]`.
- Preserves the existing AppContainer/Safe entrypoints; `ShellProcess` remains disabled for every authority profile. No MCP/CLI/relay, external process, release, installer, filesystem, network or privileged authority is added.

## Evidence and limits

- Windows host: `cargo test -p qdral-provider-process --lib sg000096` — 2/2 PASS.
- With required test-only environment entries `TMP` and `ComSpec` restored for this process, `cargo test -p qdral-provider-process --lib -- --test-threads=1` — 26/26 PASS.
- `cargo clippy -p qdral-provider-process --all-targets -- -D warnings` and `cargo fmt --all --check` — PASS.
- Native GitHub exact-head qualification, genuine Jev and Alibaba OCR review, manual excluded-file review, zero unresolved threads, normal merge and post-merge CI remain required.
- This is a native security/qualification slice, **not** a callable ordinary-user executor. Before completing T02, separately prove those token bindings against an independently supplied expected authority (parent-token equivalence in this fixture alone is not authorization), executable handle digest/file identity, workspace/cwd race safety, time/output limits, full process-tree cleanup, live lease plus consumed one-shot approval, and failure injection for Job assignment, stale identity, revoke and timeout. No current-user shell is offered in production.
- A Job Object supervises process lifecycle only and is not a filesystem or network sandbox. An ordinary OS user process still has its ambient account access.

Provenance: existing Deskal implementation in `crates/qdral-provider-process/src/lib.rs`. No Desktop Commander source imported in this slice.
