# SG-000096 T02-C: Native pre-resume inspection failure qualification

Status: test-only successor candidate; does not authorize or expose Full User ShellProcess dispatch. Tracking #271 / #264.

## Prerequisites
- Base canonical `main`: `684e2b7017fd92f1d73dfe57db8f06e959d0aa80` (PR #273 normal merge).
- Post-main CI: `37848036529`, 9/9 SUCCESS before this successor branch was created.
- Earlier native SID, LUID, session, elevation and creation-time checks remain intact.

## Test-only cases
- Case 0: native suspended child is assigned to its owned Job; exact Job membership and real token snapshot both verify before the thread resumes. Require finite completion and zero active Job processes.
- Case 1: a distinct empty Job must not attest membership of the already assigned child. Denial requires observed child termination and original Job quiescence.
- Case 2: a failed native token query using an invalid process handle cannot authorize resume. Verify child termination and original Job quiescence.
- Case 3: an invalid non-null Job handle must return a native error; any unexpected successful membership query fails the negative fixture. Verify child termination and owned Job quiescence.

No unverified child kill is classified as a successful denial. The code is inside test-only Windows fixture functions; no production process shell, admin, remote, or release routes change.

## Explicit remaining gates
This is not a substitute for separately protected installed-runtime owner identity, direct verification of its native process token, one-shot SOFT approval consumption, race-safe executable/cwd identities, bounded output/time quotas, or production cancellation. Job supervision does not sandbox filesystem/network access. T02-C, T02, T03, T04 and T05 remain open pending their full acceptance matrices.

No donor code, new external dependencies, or production capability is introduced.

## Native Windows qualification

- `cargo test -p qdral-provider-process --lib sg000096 -- --test-threads=1`: 6/6 PASS (five preceding native tests plus new failure matrix).
- `cargo test -p qdral-provider-process -- --test-threads=1`: 38/38 PASS (30 unit, 2 protected-state, 4 PowerShell, 2 workspace).
- `cargo test -p qdral-lifecycle --test release_blocker_sweep -- --test-threads=1`: 5/5 PASS.
- `cargo clippy -p qdral-provider-process --all-targets -- -D warnings`, `cargo fmt --all --check`, and whitespace: PASS.

This local qualification does not replace genuine Jev and Alibaba OCR (with manual excluded-file inspection), 9/9 exact-head CI, signed+DCO, zero unresolved threads, normal merge, or 9/9 post-main. Those are pending independent GitHub evidence.
