# Cotra Current Canonical Frontier

Status: CLOSEOUT_CANDIDATE
Date: 2026-09-27
Governance snapshot base: 08be5a4d572d4b5c6283f63ede26bdc7c0cece14
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000014 are already `CLOSED` and canonical.

This closeout candidate records SG-000015 as `CLOSED` using the qualified implementation and merged-tree evidence below. That closeout classification becomes canonical only after this governance change itself merges and the resulting canonical main passes post-merge CI.

SG-000015 implementation evidence:
- implementation PR: `#35`;
- implementation base: `235e3339071ab4e6ef233c10114a067a33be5888`;
- qualified head: `df6dbe31e06db6e8b718bd2ab8a58d936f25d35f`;
- exact-head CI: `36333639216` — SUCCESS;
- exact-head Review Gates: `36333636589` — SUCCESS;
- implementation merge: `08be5a4d572d4b5c6283f63ede26bdc7c0cece14`;
- implementation post-merge CI: `36333869446` — SUCCESS.

Genuine TypeSafe Jev reviewed the exact implementation range with `16/16` hunks, zero findings, and zero blocking findings. Alibaba Open Code Review v1.12.9 delegated the same exact range across 12 changed files: 10 reviewable files and two exclusions (`apps/cotra-mcp/src/git_mutation.test.ts` by default-path classification and `docs/security/SG-000015_LOCAL_GIT_MUTATION_NOTE.md` by unsupported-extension classification). Both excluded files were manually reviewed. There were zero unresolved blocking review threads.

Manual exact-diff security review confirmed that the public mutation surface remains restricted to `git.branch.create`, `git.stage`, `git.unstage`, and `git.commit`; no arbitrary Git argv or Git network transport is exposed; hooks, signing, editors, credential helpers/prompts, external filters, unsafe paths, and unsupported repository states fail closed; approval binds the material repository state and exact expected HEAD; state is reconstructed after approval; and resulting branch/HEAD/index/commit evidence is verified.

## Successor frontier

No successor grain is authorized by this closeout candidate.

COTRA-P06 is not complete at this frontier. The canonical architecture still requires bounded `git.fetch` and `git.push`, network policy, force-push denial by default, and verified resulting refs in addition to the local branch/stage/commit foundation now being closed.

After this SG-000015 closeout merges and its post-merge CI succeeds, re-read the canonical architecture, delivery plan, evidence ledger, open governance records, and this `CURRENT.md` to derive the next lawful COTRA-P06 grain. Do not infer or pre-authorize the next SpecGrain identifier or exact network design from numbering alone.

Any successor network-Git grain must explicitly define and prove permitted remotes/destinations, protocol and credential policy, approval class, expected ref/HEAD protections, redirect or destination-widening behavior where applicable, bounded fetch/push semantics, hard force-push denial, postcondition/ref verification, secret handling, and an explicit egress class. It must not expose generic Git shell or general network authority.

## Canonical public authority boundary retained

The SG-000015 implementation adds only bounded local Git mutation inside already trusted workspaces after fresh local approval:
- create and switch to one new validated branch rooted at the exact expected HEAD;
- stage an explicit literal repository-relative path set;
- unstage an explicit literal repository-relative path set;
- create one non-amending unsigned commit from the exact approved staged state.

Still denied or absent:
- `git.fetch` and `git.push`;
- Git network transport and credential-helper authority;
- force push;
- destructive history/ref mutation and arbitrary Git commands;
- submodule mutation;
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

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
