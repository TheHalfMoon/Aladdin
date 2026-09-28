# Cotra Current Canonical Frontier

Status: CLOSEOUT_CANDIDATE
Date: 2026-09-27
Governance snapshot base: c61d7699d5621bf476da1bac484fc9041ea2648d
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000016 are already `CLOSED` and canonical.

SG-000016 implementation PR `#38` merged as `c85b79f24ce2deef46319d9c5790623fd70c3b0d` after exact-head CI `36339733920`, Review Gates `36339732607`, genuine TypeSafe Jev `15/15` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of both excluded files, and zero unresolved review threads. Implementation post-merge CI `36339937619` completed SUCCESS.

SG-000016 governance closeout PR `#39` qualified on exact head `7ad72e3974ba754c4a4860e30f26d1af6aa5ef87` with CI `36340440771` SUCCESS, Review Gates `36340440946` SUCCESS, genuine TypeSafe Jev `6/6` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of excluded `docs/canonical/CURRENT.md`, and zero review threads. It merged normally as canonical main `d9708b1f2537befaf09db57e94099949b2a4115c`; post-closeout CI `36340662627` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

The canonical SG-000016 authority adds only local `git.fetch.preview` and approved destination-scoped anonymous `git.fetch` into `refs/remotes/cotra/<policy-id>/<branch>`; `git.push` and credential authority remain absent.

## Active grain

This closeout candidate records SG-000017 as `CLOSED` using the qualified implementation and merged-tree evidence below. That closeout classification becomes canonical only after this governance change itself merges and the resulting canonical main passes post-merge CI.

SG-000017 implementation evidence:
- implementation PR: `#41`;
- implementation base: `6cbf1055f7432dba1a6dd682e4f1e364ec1aff2a`;
- qualified head: `4b769372b2b27fb1d3cc3ee3f3e0ee2c6cb5edee`;
- exact-head CI: `36345155505` — SUCCESS;
- exact-head Review Gates: `36345155633` — SUCCESS;
- implementation merge: `c61d7699d5621bf476da1bac484fc9041ea2648d`;
- implementation post-merge CI: `36345286560` — SUCCESS.

Genuine TypeSafe Jev reviewed the exact implementation range with `14/14` hunks, zero findings, and zero blocking findings. Alibaba Open Code Review v1.12.9 delegated the same exact range across 12 changed files: 10 reviewable files and two exclusions (`apps/cotra-mcp/src/git_push.test.ts` by default-path classification and `docs/security/SG-000017_BOUNDED_GIT_PUSH_NOTE.md` by unsupported-extension classification). Both excluded files were manually reviewed. There were zero unresolved blocking review threads.

Manual exact-diff security review confirmed workspace-bound push destination policy with stable policy ids and destination-bound credential references, canonical HTTPS with port 443 and no userinfo or alternate transports, short source and destination branch validation rejecting full refs refspec separators force markers and wildcards, local-only preview with no DNS network approval secret access or mutation, public-address classification with pinned request address and post-approval drift denial, fresh approval bound to workspace repository policy destination pinned address refs HEAD prior and credential reference without secret material, post-approval destination state and credential-binding revalidation, remote-state comparison before push bytes, HTTPS-only transport with redirects disabled TLS verification enabled proxy cookie header credential-helper askpass suppression repository URL-rewrite rejection and hook neutralization, non-fast-forward rejection with hard force-push deletion tag mirror and arbitrary-refspec denial, secret redaction in evidence errors argv config and child environments with isolated askpass cleanup, resulting-ref verification with remote-tracking update and unchanged checkout proof, and retained SG-000015 and SG-000016 regressions.

The canonical SG-000017 authority adds only local `git.push.preview` and approved destination-scoped `git.push` of one validated `refs/heads` source to one validated `refs/heads` destination on one workspace-bound canonical HTTPS destination with protected credential references; force push, branch deletion, raw credential arguments, and ambient credential-manager authority remain absent.

## COTRA-P06 exit matrix

COTRA-P06 exits when branch, stage, unstage, commit, fetch, and push are all canonically closed with force push denied by default, network policy applied, and resulting refs verified:

- branch, stage, unstage, commit — `CLOSED` under SG-000015 with exact approved-state controls;
- fetch preview and approved fetch — `CLOSED` under SG-000016 with workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls;
- push preview and approved push — `CLOSED` under this SG-000017 candidate with destination-bound credential references, expected remote-ref protection, non-fast-forward rejection, hard force-push denial, destination revalidation, fresh approval, secret redaction, and post-push verification;
- force push denied by default — proven by SG-000017 command-plan and validation tests with no force, delete, mirror, or wildcard path;
- network policy applied — destination-scoped canonical HTTPS with public-address pinning on both fetch and push transports;
- resulting refs verified — fetch destination-ref and push remote-ref verification with unchanged checkout proof.

No P06 capability remains open after this closeout merges and its post-merge CI succeeds.

## Successor frontier

No successor grain is authorized by this closeout candidate.

After this SG-000017 closeout merges and its post-merge CI succeeds, re-read the canonical architecture, delivery plan, evidence ledger, open governance records, and this `CURRENT.md` to derive the next lawful COTRA-P07 grain for strong approval and trust UX. Do not infer or pre-authorize the next SpecGrain identifier or exact approval design from numbering alone.

A successor strong-approval grain must explicitly define and prove local user-presence approval, trust UX, workspace trust, approval history, replay resistance, stale approval resistance, approval drift protection, and separation of agent authority from approval authority. It must not expose browser automation, UI automation, elevation, or approval bypass.

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Still denied or absent outside the active SG-000017 push scope:
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
- force push and branch deletion;
- raw credential arguments and ambient credential-manager authority;
- generic network fetch/socket authority;
- browser automation;
- Windows UI Automation/input injection;
- elevation;
- approval bypass or persistent approval reuse.

A later lawful grain must add P07 strong approval and trust UX only after COTRA-P06 exits with branch, stage, unstage, commit, fetch, and push all canonically closed. Raw credentials must never be accepted merely because Git can consume them.

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
