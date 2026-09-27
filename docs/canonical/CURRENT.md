# Cotra Current Canonical Frontier

Status: ACTIVE_GRAIN
Date: 2026-09-27
Governance snapshot base: 9b70d2629d66b6e79180edc2c813c460ae5b49fb
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this activation snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000013 are `CLOSED` and canonical.

SG-000013 recovery closeout merged through PR `#30` as canonical main `9b70d2629d66b6e79180edc2c813c460ae5b49fb`; post-closeout CI `36276793414` completed SUCCESS.

The failed first closeout PR `#28` remains historical failed evidence and is not erased. Forward-only recovery PR `#29` repaired the Windows parallel temp-workspace collision without authority expansion before the successful recovery closeout.

## Active grain

SG-000014 — Workspace-scoped AppContainer filesystem authority qualification — is the sole active COTRA-P05 grain.

This activation authorizes only provider-private qualification of the minimum stable-Windows filesystem authority needed for a zero-capability AppContainer child to read and write inside one explicit test-owned trusted workspace while remaining denied access to Cotra protected state, unrelated user resources, network capability, and inherited authority channels.

No public authority is added by activation.

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

## Active acceptance frontier

SG-000014 must prove on native Windows that the contained AppContainer identity can read and write only within an explicit test-owned trusted workspace after a minimum stable-Windows grant is established, while a sibling resource outside that workspace and Cotra protected state remain denied.

The implementation must retain SG-000011 descendant lifecycle, SG-000012 protected-state/authority-channel isolation, and SG-000013 bounded PowerShell containment regressions. Temporary filesystem-security mutation must be deterministic, test-owned, bounded, and cleaned up.

The Microsoft experimental `CreateProcessInSandbox` / Bound File System path remains research-only and is excluded as the production baseline for this grain.

Do not expose public PowerShell or broader process/filesystem authority in SG-000014.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
