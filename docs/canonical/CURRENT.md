# Cotra Current Canonical Frontier

Status: CLOSEOUT_CANDIDATE
Date: 2026-09-27
Governance snapshot base: c85b79f24ce2deef46319d9c5790623fd70c3b0d
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000015 are already `CLOSED` and canonical.

This closeout candidate records SG-000016 as `CLOSED` using the qualified implementation and merged-tree evidence below. That closeout classification becomes canonical only after this governance change itself merges and the resulting canonical main passes post-merge CI.

SG-000016 implementation evidence:
- implementation PR: `#38`;
- implementation base: `5f4c15073f11be20066d1dde319b742fb49f5a3d`;
- qualified head: `fbe699f52ada7792a7009ab3100e5d370ec0fa33`;
- exact-head CI: `36339733920` — SUCCESS;
- exact-head Review Gates: `36339732607` — SUCCESS;
- implementation merge: `c85b79f24ce2deef46319d9c5790623fd70c3b0d`;
- implementation post-merge CI: `36339937619` — SUCCESS.

Genuine TypeSafe Jev reviewed the exact implementation range with `15/15` hunks, zero findings, and zero blocking findings. Alibaba Open Code Review v1.12.9 delegated the same exact range across 12 changed files: 10 reviewable files and two exclusions (`apps/cotra-mcp/src/git_fetch.test.ts` by default-path classification and `docs/security/SG-000016_BOUNDED_GIT_FETCH_NOTE.md` by unsupported-extension classification). Both excluded files were manually reviewed. There were zero unresolved blocking review threads.

Manual exact-diff security review confirmed destination-scoped canonical HTTPS policy with stable policy ids, short-branch validation rejecting full refs, local-only preview with no DNS or network, public-address classification with pinned request address and post-approval drift denial, HTTPS-only transport with redirects disabled and TLS verification enabled, proxy cookie header credential-helper askpass prompt suppression with repository rewrite rejection, deterministic Cotra-owned ref mutation with exact HEAD and prior binding, fresh approval with revalidation, resulting-ref verification with unchanged checkout proof, and hard-denied push force-push arbitrary network shell PowerShell browser elevation and approval reuse.

The canonical SG-000016 authority adds only local `git.fetch.preview` and approved destination-scoped anonymous `git.fetch` into `refs/remotes/cotra/<policy-id>/<branch>`; `git.push` and credential authority remain absent.

## Successor frontier

No successor grain is authorized by this closeout candidate.

COTRA-P06 is not complete at this frontier. The canonical architecture still requires the lawful push path with protected credential handling, explicit source and destination refs, expected remote-ref protection, force and non-fast-forward denial, destination revalidation, fresh approval, secret redaction, and post-push verification, in addition to the local mutation and fetch foundation now being closed.

After this SG-000016 closeout merges and its post-merge CI succeeds, re-read the canonical architecture, delivery plan, evidence ledger, open governance records, and this `CURRENT.md` to derive the next lawful COTRA-P06 grain. Do not infer or pre-authorize the next SpecGrain identifier or exact push design from numbering alone.

A successor push grain must explicitly define and prove permitted destinations, protocol and credential policy, approval class, expected ref protections, redirect or destination-widening behavior where applicable, bounded push semantics, hard force-push denial, postcondition and ref verification, secret handling, and an explicit egress class. It must not expose generic Git shell or general network authority.

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Still denied or absent outside the closed SG-000016 HTTPS-fetch scope:
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
- generic process network authority;
- PowerShell remoting;
- detached/background public execution;
- public process kill capability;
- `git.push` and force push;
- generic network fetch/socket authority;
- browser automation;
- Windows UI Automation/input injection;
- elevation;
- approval bypass or persistent approval reuse.

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
