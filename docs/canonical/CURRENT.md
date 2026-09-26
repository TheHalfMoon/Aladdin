# Cotra Current Canonical Frontier

Status: CLOSEOUT_RECOVERY
Date: 2026-09-26
Governance snapshot base: 23796b29eaed06781fe964d16a19768c7800886b
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the canonical parent from which this snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000012 are `CLOSED` and canonical.

SG-000013 remains `PROVEN`, not `CLOSED`.

Its implementation remains valid and proven:
- implementation PR `#27`;
- qualified head `74b4a91d7b8e7f582a0df0fc814526658795514f`;
- exact-head CI `36258473411` — 5/5 SUCCESS;
- Review Gates `36258472190` — SUCCESS;
- genuine TypeSafe Jev 2/2 hunks, zero findings/blockers;
- Alibaba Open Code Review v1.12.9 exact-range delegation SUCCESS;
- native Windows SG-000013 PowerShell integration tests 4/4 PASS;
- implementation merge `ddfcb4c9d5f06af446d44e031eb96bd70e4424d9`;
- implementation post-merge CI `36258601073` — 5/5 SUCCESS.

## Failed first closeout attempt

Closeout PR `#28` was exact-head qualified and merged as `23796b29eaed06781fe964d16a19768c7800886b`, but canonical post-merge CI `36262599177` failed in Windows Rust.

The failure was not in SG-000013 PowerShell containment. It was a pre-existing test-isolation defect in `cotra-policy`: parallel tests generated temporary workspace names from clock nanoseconds only. On Windows, two tests could receive the same effective timestamp; one test could remove the shared directory before another authorization call, producing `WorkspaceDenied` with OS error 2.

This recovery branch repairs the helper by adding process ID plus an atomic per-process sequence to the temp directory name. It also corrects SG-000013 machine state from prematurely `CLOSED` back to `PROVEN` until recovery is merged, post-merge verified, and a fresh governance closeout succeeds.

## Canonical public authority boundary retained

No authority changed because of the closeout or recovery.

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Still denied or absent:
- public `powershell.run`;
- caller-provided PowerShell script text;
- direct PowerShell executable authority through `process.spawn`;
- generic executable-registry widening;
- additional public `process.spawn` executable targets;
- `cmd.exe` / raw shell authority;
- caller-provided process environment overrides;
- stdin payload injection;
- process network authority;
- PowerShell remoting;
- detached/background public execution;
- public process kill capability;
- Git mutation;
- browser automation;
- Windows UI Automation/input injection;
- elevation;
- approval bypass or persistent approval reuse.

## Next frontier after recovery

Do not activate a successor until SG-000013 is canonically `CLOSED`.

The next COTRA-P05 evidence unit must close the workspace-access gap required by the original Windows containment decision before any generic/public PowerShell authority is exposed. SG-000013 proved PowerShell process containment but did not prove intended read/write access to a trusted workspace. A successor must therefore qualify workspace-scoped AppContainer filesystem authority while retaining denial of Cotra protected state, network, authority-channel inheritance, and unrelated user resources.

The Microsoft experimental `CreateProcessInSandbox` / Bound File System path is research-only for now because it remains experimental and does not support inherited handles required by Cotra's current explicit stdout/stderr/NUL-handle model. The baseline must use stable Windows security primitives unless that compatibility gap is separately resolved and qualified.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED without required exact-head, platform, merge, post-merge, and governance evidence.

Language rule: all repository technical content is English only.
