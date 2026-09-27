# Cotra Current Canonical Frontier

Status: CLOSEOUT_CANDIDATE
Date: 2026-09-27
Governance snapshot base: 8bb6b50a543d20d27f8cc86fa43a1ab747571ea4
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000013 are already `CLOSED` and canonical.

This closeout candidate records SG-000014 as `CLOSED` using the qualified implementation and merged-tree evidence below. That closeout classification becomes canonical only after this governance change itself merges and the resulting canonical main passes post-merge CI.

SG-000014 implementation evidence:
- implementation PR: `#32`;
- implementation base: `8ae4a41c242878b4e5ed9a8431dd4f23fc4b69fa`;
- qualified head: `5825070722220046717015c77efa528e132344e0`;
- exact-head CI: `36294704134` — SUCCESS;
- exact-head Review Gates: `36294703030` — SUCCESS;
- implementation merge: `8bb6b50a543d20d27f8cc86fa43a1ab747571ea4`;
- implementation post-merge CI: `36294846126` — SUCCESS.

Native Windows qualification proved that the zero-capability AppContainer child could read the deterministic input and create the deterministic output only inside the explicitly granted test-owned workspace, remained denied the deterministic sibling resource, observed NUL/EOF stdin, received no tested Cotra/API secret-like environment values, and reached verified Job quiescence. The original workspace DACL was explicitly restored before fixture cleanup. SG-000012 protected-state isolation and SG-000013 bounded PowerShell regressions remained green.

Genuine TypeSafe Jev reviewed the exact implementation range with `2/2` hunks, zero findings, and zero blocking findings. Alibaba Open Code Review v1.12.9 delegated the exact range with one reviewable code file and one unsupported-extension security note; the excluded note was manually security-reviewed. There were zero unresolved blocking review threads.

## Successor frontier

No successor grain is authorized by this closeout candidate.

After this SG-000014 closeout merges and its post-merge CI succeeds, re-read the canonical architecture, delivery plan, evidence ledger, open governance records, and `CURRENT.md` to derive the next lawful COTRA-P05 frontier. Do not infer or pre-authorize SG-000015 from numbering alone.

## Canonical public authority boundary retained

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Still denied or absent:
- public `powershell.run`;
- caller-provided PowerShell script text;
- direct PowerShell executable authority through `process.spawn`;
- generic executable-registry widening;
- additional public `process.spawn` executable targets;
- broad user-profile or arbitrary filesystem grants;
- caller-selected ACL targets/security descriptors;
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

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
