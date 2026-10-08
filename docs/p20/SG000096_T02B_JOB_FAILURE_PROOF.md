# SG-000096 T02-B: Verified Native Job-Assignment Failure Cleanup

Status: implementation candidate only; not T02 exit or executable Full User shell authority.

## Implementation boundary

- Reuses Deskal's existing `qdral-provider-process` Windows `CreateProcessW`, suspended `ChildProcess`, `OwnedHandle`, and Job Object machinery without importing donor source or adding dependencies.
- Adds `JobObject::assign_or_terminate`: if native Job assignment fails or fails membership verification, the suspended child must be terminated and its process handle signaled before the original failure is returned. Unverified termination returns its own failure.
- Corrects `ChildProcess::terminate_best_effort` so it never marks a child completed after an unverified `TerminateProcess`/`WaitForSingleObject` attempt.
- Reuses verified assignment cleanup in existing AppContainer single-child, piped execution and the SG-000096 ordinary-current-user disposable fixture.
- Adds a native Windows test that successfully creates a suspended ordinary-user child, supplies an invalid test-only Job handle to induce a **real native** `AssignProcessToJobObject` error, invokes the shared failure cleanup, and asserts the child process handle is signaled with no child left running.

## Native Windows evidence

- `cargo test -p qdral-provider-process --lib sg000096 -- --test-threads=1`: 3/3 PASS.
- `cargo test -p qdral-provider-process -- --test-threads=1`: 35/35 PASS (27 library, 2 protected-state, 4 PowerShell, 2 workspace).
- `cargo clippy -p qdral-provider-process --all-targets -- -D warnings`: PASS.
- `cargo fmt --all --check` and `git diff --check`: PASS.

## Exclusions and residual gates

- This is T02-B native cleanup only. No production Full User command/session dispatcher, MCP or CLI exposure, leased authority, approval consumption, remote/full-admin shell, release payload or installer change.
- Native parent/child token equivalence does **not** substitute for independent expected local lease/owner/epoch identity validation. Safe/AppContainer limits remain unchanged.
- Windows Job Object lifecycle protection is **not** network or filesystem sandboxing; a future permitted user-token process has the ambient OS user's file and network access.
- Expected executable handle digest/file ID, cwd junction/reparse race proof, bounded new Full User output/timeout, independent approval consumption, lease revoke/expiry and full process-tree lifecycle under future T02 implementation remain open.
- Exact-head CI, genuine TypeSafe Jev, Alibaba OCR Delegation plus manual exclusions, zero unresolved PR threads, normal signed+DCO merge, and post-merge CI remain required. Do not activate T03, T04 or T05 solely because of this proof.

Provenance: native Rust code already owned by Deskal. No new donor source imported.
