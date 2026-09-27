# Cotra Current Canonical Frontier

Status: ACTIVE_GRAIN
Date: 2026-09-27
Governance snapshot base: 5ded68d3f42100ef6f077c6b61fcd79c2a0784d5
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this activation snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000014 are `CLOSED` and canonical.

SG-000014 closeout merged through PR `#33` as canonical main `5ded68d3f42100ef6f077c6b61fcd79c2a0784d5`; post-closeout CI `36302401799` completed SUCCESS.

The completed COTRA-P05 program now has canonical evidence for bounded argv execution, destructive timeout/output-limit termination, descendant lifecycle hardening, protected-state isolation, provider-private bounded PowerShell containment, and workspace-scoped AppContainer filesystem qualification. Public `powershell.run` was not exposed by P05.

## Active grain

SG-000015 — Approved local Git mutation foundation — is the sole active COTRA-P06 grain.

This activation authorizes implementation and qualification of local-only typed Git mutation for trusted workspace repositories:
- create and switch to one new validated branch rooted at an exact expected HEAD;
- stage an explicit literal repository-relative path set;
- unstage an explicit literal repository-relative path set;
- create one non-amending unsigned commit from an exact approved staged state.

Every mutation must require fresh local approval and must re-check expected HEAD plus the material approved state after approval before changing repository state.

## Active acceptance frontier

SG-000015 must prove that local Git mutation is narrower than generic Git command execution.

Required controls include:
- repository root remains canonically inside the selected trusted workspace;
- expected HEAD is mandatory and stale HEAD fails closed before mutation;
- approval binds workspace, repository, operation, expected HEAD, exact literal path set or staged-state digest, branch/message material, and policy revision;
- material state is revalidated after approval before mutation;
- hooks, editors, commit signing, credential prompting/helpers, fsmonitor, pagers, and Git network transport remain disabled or absent;
- staging rejects paths with configured Git filter drivers so clean/smudge/process filters cannot become hidden code-execution authority;
- mutation path inputs reject absolute paths, parent escapes, NUL, pathspec magic, and `.git` control paths;
- postconditions verify resulting branch/HEAD/index/commit evidence;
- read-only `git.status`, `git.diff`, and `git.log` regressions remain green.

## Deferred COTRA-P06 authority

Not authorized by SG-000015:
- `git.fetch`;
- `git.push`;
- any Git network transport;
- credential helpers or interactive credential prompts;
- force push;
- branch deletion;
- switching to an existing branch;
- reset, rebase, merge, cherry-pick, revert, stash, clean, tag mutation, or arbitrary Git subcommands;
- submodule mutation;
- commit amend or commit signing.

A later lawful COTRA-P06 grain may qualify bounded network Git operations only after SG-000015 reaches canonical closeout and destination/credential policy is explicitly defined and proven.

## Canonical public authority boundary retained

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Still denied or absent outside the active SG-000015 Git-local scope:
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
- browser automation;
- Windows UI Automation/input injection;
- elevation;
- approval bypass or persistent approval reuse.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
