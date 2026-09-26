# Cotra Current Canonical Frontier

Status: COMPLETE_CANONICAL
Date: 2026-09-27
Governance snapshot base: d6bb05a1b8dbb26b0927b1acce67854c8bdad80d
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the canonical parent from which this snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000013 are `CLOSED` and canonical, subject to exact-head qualification and successful merge/post-merge verification of this recovery closeout.

SG-000013 implementation evidence remains:
- implementation PR `#27`;
- qualified head `74b4a91d7b8e7f582a0df0fc814526658795514f`;
- exact-head CI `36258473411` — 5/5 SUCCESS;
- Review Gates `36258472190` — SUCCESS;
- genuine TypeSafe Jev 2/2 hunks, zero findings/blockers;
- Alibaba Open Code Review v1.12.9 exact-range delegation SUCCESS;
- native Windows SG-000013 PowerShell integration tests 4/4 PASS;
- implementation merge `ddfcb4c9d5f06af446d44e031eb96bd70e4424d9`;
- implementation post-merge CI `36258601073` — 5/5 SUCCESS.

The failed first closeout PR `#28` remains recorded as failed evidence and is not erased. Its post-merge CI `36262599177` exposed a pre-existing Windows parallel temp-workspace collision in `cotra-policy` rather than a PowerShell-containment defect.

Forward-only recovery PR `#29` repaired that test-isolation defect without authority expansion. Recovery head `d9285db573c11a192fdf51cd8829f010191f5902` merged as `d6bb05a1b8dbb26b0927b1acce67854c8bdad80d`; recovery post-merge CI `36266186799` completed SUCCESS.

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

## Next lawful frontier

After this closeout is exact-head qualified, merged, and post-merge verified, derive the successor from repository truth before activation.

The next COTRA-P05 evidence unit must close the workspace-access gap required by the original Windows containment decision before any generic/public PowerShell authority is exposed. SG-000013 proved PowerShell process containment but did not prove intended read/write access to a trusted workspace. The successor must therefore qualify workspace-scoped AppContainer filesystem authority while retaining denial of Cotra protected state, network, authority-channel inheritance, and unrelated user resources.

The Microsoft experimental `CreateProcessInSandbox` / Bound File System path remains research-only because it is experimental and does not support inherited handles required by Cotra's current explicit stdout/stderr/NUL-handle model. The baseline must use stable Windows security primitives unless that compatibility gap is separately resolved and qualified.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED without required exact-head, platform, merge, post-merge, and governance evidence.

Language rule: all repository technical content is English only.
