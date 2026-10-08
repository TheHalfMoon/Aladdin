# SG-000096 T02-C: Native Expected-Owner Qualification Candidate

Status: **partial, test-only candidate**. Tracking: #271; parent: #264. This document is not evidence of T02-C closeout, T02 completion, or production Full User shell activation.

## Prerequisite

T02-B PR #270 normally merged as `525bb081106cb46ba6852d14a6f977e27ab823e3`. Post-main CI run `37835734532` completed **9/9 SUCCESS** before this successor branch was implemented. The branch was fast-forwarded to this exact canonical commit.

## Implemented boundaries

- Windows-only test helpers read actual `TokenUser`, `TokenStatistics.AuthenticationId`, session ID, elevation, and `GetProcessTimes` creation generation from native handles.
- Expected SID is compared via the Windows `ConvertStringSidToSidW` and `EqualSid` primitives; native LUID, session, elevation, and creation generation are separately checked.
- A disposable current-user process is created suspended and assigned to the owned kill-on-close Job before expected-identity verification. A mismatch cannot reach `ResumeThread`; verified process termination and Job quiescence are required.
- Test-only negative cases: mismatched SID, logon AuthenticationId LUID, session, elevation, process creation generation, and malformed SID. One native positive test confirms an exact expected binding proceeds and drains.
- Existing production Safe AppContainer execution remains unchanged. All newly added helpers are `#[cfg(test)]`. There is **no production Full User process executor**, caller-facing MCP/CLI, or release/installer mutation.

## Evidence from authorized Windows host

- `cargo test -p qdral-provider-process --lib sg000096 -- --test-threads=1`: **4/4 PASS** (including the new test).
- `cargo test -p qdral-provider-process -- --test-threads=1`: **36/36 PASS** (28 unit, 2 protected-state, 4 PowerShell, 2 workspace).
- `cargo clippy -p qdral-provider-process --all-targets -- -D warnings`: PASS.
- `cargo fmt --all --check` and `git diff --check`: PASS.
- Five initial full-suite tests reported missing qualification environment variables on the authorized interactive PC, not test assertion failures. A rerun supplied only process-scoped `TMP=$TEMP` and `ComSpec=$SystemRoot\\System32\\cmd.exe`, without changing OS settings or repository code; the full rerun passed.

## Residual requirements / refusal to overclaim

- The test derives expected user SID, logon, session and elevation from a separately opened *parent token*; the expected child creation generation is recorded from the child handle. This **does not prove** that a production server independently obtains a protected STRONG Full User lease, validates policy revisions/owner generation, or consumes SOFT action approval.
- Native token query fault injection, reparse-safe executable/cwd handle identity and hashes, replay-resistant approval consumption, output quotas, timeout/revoke/descendant cleanup, and a production private Full User launcher remain for separately reviewed successors.
- Windows Job lifecycle supervision is not filesystem or network isolation. Any future Full User shell has ambient OS user capabilities.
- Do not close issue #271 or activate T03/T04 until all relevant successor gates, local qualified review, signed+DCO merge, exact-head CI 9/9, zero unresolved threads, and post-merge main CI have passed.

Provenance: native Deskal-owned Rust code only. No new donor source or dependency imported.
