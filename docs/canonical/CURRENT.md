# Cotra Current Canonical Frontier

Status: CLOSED_GRAIN
Date: 2026-09-28
Governance snapshot base: 698eff08e736c133b93100e1983fe4be2c5909d0
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000022 are `CLOSED` and canonical.

SG-000016 implementation PR `#38` merged as `c85b79f24ce2deef46319d9c5790623fd70c3b0d` after exact-head CI `36339733920`, Review Gates `36339732607`, genuine TypeSafe Jev `15/15` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of both excluded files, and zero unresolved review threads. Implementation post-merge CI `36339937619` completed SUCCESS.

SG-000016 governance closeout PR `#39` qualified on exact head `7ad72e3974ba754c4a4860e30f26d1af6aa5ef87` with CI `36340440771` SUCCESS, Review Gates `36340440946` SUCCESS, genuine TypeSafe Jev `6/6` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of excluded `docs/canonical/CURRENT.md`, and zero review threads. It merged normally as canonical main `d9708b1f2537befaf09db57e94099949b2a4115c`; post-closeout CI `36340662627` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

The canonical SG-000016 authority adds only local `git.fetch.preview` and approved destination-scoped anonymous `git.fetch` into `refs/remotes/cotra/<policy-id>/<branch>`; `git.push` and credential authority remain absent.

SG-000017 implementation PR `#41` merged as `c61d7699d5621bf476da1bac484fc9041ea2648d` after exact-head CI `36345155505`, Review Gates `36345155633`, genuine TypeSafe Jev `14/14` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of both excluded files, and zero unresolved review threads. Implementation post-merge CI `36345286560` completed SUCCESS.

SG-000017 governance closeout PR `#42` qualified on exact head `94b14d404aca0d083c8ada28660eee6dc850606b` with CI `36360818831` SUCCESS, Review Gates `36360818833` SUCCESS, genuine TypeSafe Jev `7/7` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of excluded `docs/canonical/CURRENT.md`, and zero review threads. It merged normally as canonical main `517c03b58fa898bc5aec2de7debd86ff9969796b`; post-closeout CI `36361047524` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

The canonical SG-000017 authority adds only local `git.push.preview` and approved destination-scoped `git.push` of one validated `refs/heads` source to one validated `refs/heads` destination on one workspace-bound canonical HTTPS destination with protected credential references; force push, branch deletion, raw credential arguments, and ambient credential-manager authority remain absent.

COTRA-P06 is exited at this frontier: branch, stage, unstage, and commit are closed under SG-000015, fetch is closed under SG-000016, and push is closed under SG-000017, with force push denied by default, destination-scoped network policy applied, and resulting refs verified.

SG-000018 implementation PR `#44` merged as `263abbdfa68beaefd5d48e1354f32b46c11bfb38` after exact-head CI `36363644307`, Review Gates `36363642475`, genuine TypeSafe Jev `36/36` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of the excluded file, and zero unresolved review threads. Implementation post-merge CI `36363931388` completed SUCCESS.

SG-000018 governance closeout PR `#45` qualified on exact head `6d70c65242324a9763c08a4d2dfc3f145d51e29d` with CI `36364171183` SUCCESS, Review Gates `36364171105` SUCCESS, genuine TypeSafe Jev `7/7` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of excluded `docs/canonical/CURRENT.md`, and zero review threads. It merged normally as canonical main `e9728df8535ee64381f1c29dbdced6850d856226`; post-closeout CI `36364352725` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

The canonical SG-000018 authority adds only broker-level approval nonces with short expiry, one-shot exact-digest binding, and an append-only redacted approval history; strong user-presence approval, trust changes, persistent reuse, remote delegation, and approval bypass remain absent.

SG-000019 implementation PR `#47` merged as `ac1e7c98040e3812a5d2e235f89d05bba773ecdd` after exact-head CI `36380608151`, Review Gates `36380606179`, genuine TypeSafe Jev `45/45` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of the excluded file, and zero unresolved review threads. Implementation post-merge CI `36380978676` completed SUCCESS.

SG-000019 governance closeout PR `#48` qualified on exact head `2fa26801a1842023a0807953e161657bf23432ba` with CI `36381657856` SUCCESS, Review Gates `36381658308` SUCCESS, genuine TypeSafe Jev `7/7` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of excluded `docs/canonical/CURRENT.md`, and zero review threads. It merged normally as canonical main `35f1f84d45118abd50e5a4b4606294e45311fd78`; post-closeout CI `36381829440` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

The canonical SG-000019 authority adds only the STRONG approval class with platform-mediated presence verification for PRIVILEGED and selected DESTRUCTIVE operations with fail-closed unavailable handling; existing file, process, and Git operations retain the SOFT class, and trust changes, persistent reuse, remote delegation, and approval bypass remain absent.

## Closed SG-000020 trust management

This program-exit snapshot records SG-000020 as merged and canonical. The `CLOSED` classification below becomes part of the canonical frontier only after this program-exit change itself merges and the resulting canonical main passes post-merge CI.

SG-000020 activation evidence:
- activation PR: `#49`;
- activation base: `35f1f84d45118abd50e5a4b4606294e45311fd78`;
- activation head: `3c69962132238142d200b3b27a9bc9e59ee54f58`;
- exact-head CI: `36382114469` — SUCCESS;
- exact-head Review Gates: `36382114395` — SUCCESS;
- activation merge: `d3a71554a02e009e903a47c75f1b4e4f0086b8d5`;
- activation post-merge CI: `36382435022` — SUCCESS.

Genuine TypeSafe Jev reviewed the exact activation range with `7/7` hunks, zero findings, and zero blocking findings. Alibaba Open Code Review v1.12.9 delegated the same exact range across 2 changed files: 1 reviewable file (`.specgrain/specs/SG-000020.json`) and one exclusion (`docs/canonical/CURRENT.md` by unsupported-extension classification). The excluded file was manually reviewed. There were zero unresolved blocking review threads.

SG-000020 implementation evidence:
- implementation PR: `#50`;
- implementation base: `d3a71554a02e009e903a47c75f1b4e4f0086b8d5`;
- qualified head: `ea609b013d9233f696fb9b5877f59c5a7600d0ec`;
- exact-head CI: `36383880270` — SUCCESS;
- exact-head Review Gates: `36383878962` — SUCCESS;
- implementation merge: `e40d7ad255566025617557bbd8f9fb91f1595550`;
- implementation post-merge CI: `36384092522` — SUCCESS.

Genuine TypeSafe Jev reviewed the exact implementation range with `27/27` hunks, zero findings, and zero blocking findings, pinned at `31f89602797fb7bea007f8a480bf368bf564954e`. Alibaba Open Code Review v1.12.9 delegated the same exact range across 7 changed files: 6 reviewable files and one exclusion (`docs/security/SG-000020_TRUST_REVOKE_NOTE.md` by unsupported-extension classification). The excluded security note was manually reviewed. There were zero unresolved blocking review threads.

Manual exact-diff security review confirmed trust grant, trust revoke, and emergency revoke require the STRONG class with platform-mediated presence and a suspended input lease, no caller-supplied trust boolean is accepted, the trust digest binds workspace, policy revision, capability, requested transition, current trust state, and current revision with stale-state fail-closed, emergency revoke advances a global epoch that fails closed for pre-revoke tokens while post-revoke tokens remain usable, trust and approval history are checksum-chained, redacted, bounded between 1 and 200, and tamper-evident, unknown or corrupt trust fails closed as untrusted, denied and unavailable trust and revoke decisions are recorded without authorizing execution, no MCP trust-approve tool exists, no trust-change bypass, persistent reuse, remote delegation, browser, UI, network, clipboard, installer, elevation, or synthetic-input authority is introduced, and SG-000018 replay resistance plus SG-000019 STRONG enforcement with P06 regressions remain intact.

The canonical SG-000020 authority adds only workspace trust management with STRONG-gated changes, class-labeled history UX, and emergency revoke with fail-closed epoch invalidation through dedicated PRIVILEGED APIs; existing file, process, and Git operations retain their class, and trust-change bypass, persistent reuse, remote delegation, and approval bypass remain absent.

SG-000020 governance closeout PR `#51` qualified on exact head `10aa7593671ad0e76a3973ad2be21c2e555169ed` with CI `36403028457` SUCCESS, Review Gates `36403028473` SUCCESS, genuine TypeSafe Jev `9/9` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation over 3 files with 2 reviewable and 1 excluded file (`docs/canonical/CURRENT.md`) manually reviewed, and zero unresolved blocking review threads. It merged normally as canonical main `da98225fe3cec510dcdc14459c11293cea0cce59`; post-closeout CI `36403358203` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

## COTRA-P07 exit

COTRA-P07 is exited at this frontier. Strong approval and trust UX are closed under SG-000018, SG-000019, and SG-000020, with the agent unable to produce a valid strong approval or trust change through its own tool or input surface, replay and drift resistance proven, and resulting trust and revoke records verified.

Exit matrix (each criterion is COMPLETE with canonical evidence in `.specgrain/canonical-evidence.json` and the grain blocks above):

Approval and replay — COMPLETE:
- approval classes with SOFT/STRONG separation via SG-000019 (implementation PR `#47`, qualified head `b2b5b84fc5a14cf3d056c71eccb44f6f7b2cf674`, CI `36380608151`, Jev `45/45`, OCR v1.12.9 `6 reviewable + 1 excluded/manually reviewed`, merge `ac1e7c98040e3812a5d2e235f89d05bba773ecdd`, post-merge CI `36380978676`);
- nonce protection, expiry, one-shot consumption, replay resistance, digest binding, and drift protection via SG-000018 (implementation PR `#44`, qualified head `9ecd534c008ca1fd1c28aacfd1e7e96323d7cea1`, CI `36363644307`, Jev `36/36`, OCR v1.12.9 `10 reviewable + 1 excluded/manually reviewed`, merge `263abbdfa68beaefd5d48e1354f32b46c11bfb38`, post-merge CI `36363931388`).

Approval broker — COMPLETE:
- broker separation with workspace and policy binding, unavailable-path fail-closed, no caller-created approval, and no bypass or reuse via SG-000018 with the same implementation evidence above.

Strong user presence — COMPLETE:
- STRONG presence boundary with platform-mediated verification, agent forgery resistance, no caller-provided strong claim, no silent downgrade, and fail-closed unavailable, cancelled, and timeout paths via SG-000019 with the same implementation evidence above; no Windows Hello or presence material enters records, history, or logs.

Workspace trust — COMPLETE:
- stable workspace identity, explicit trust state, STRONG-gated mutation, no caller-forged trust, no agent self-grant, policy revision binding, and trust persistence via SG-000020 (activation PR `#49`, implementation PR `#50`, qualified head `ea609b013d9233f696fb9b5877f59c5a7600d0ec`, CI `36383880270`, Jev `27/27`, OCR v1.12.9 `6 reviewable + 1 excluded/manually reviewed`, merge `e40d7ad255566025617557bbd8f9fb91f1595550`, post-merge CI `36384092522`).

Revocation — COMPLETE:
- normal revoke, emergency revoke, epoch-based invalidation, stale-token rejection, and no silent agent undo of revoke via SG-000020 with the same implementation evidence above.

History — COMPLETE:
- redacted approval history and redacted trust history with class labeling, checksum-chain integrity, bounded queries between 1 and 200, no secret leakage, fail-closed corrupt state, and class-labeled history UX via SG-000018, SG-000019, and SG-000020 with the implementation evidence above.

Authority separation — COMPLETE:
- agent authority remains separate from approval authority, STRONG presence authority remains separate from the model, and P07 introduced no new network, browser, UI automation, clipboard, installer, elevation, or delegation authority beyond the dedicated PRIVILEGED trust APIs recorded in the three grain authority boundaries above.

All P07 closeouts (SG-000018 PR `#45`, SG-000019 PR `#48`, SG-000020 PR `#51`) merged normally with zero unresolved blocking review threads and successful post-merge CI.

## Closed SG-000021 browser foundation

SG-000021 closed canonically: activation PR `#53`, implementation PR `#54` (qualified head `f9912d7a4be7a822954e5dc260be5e698adf9ff8`, CI `36411407342`, Review Gates `36411407224`, Jev `10/10`, OCR v1.12.9 `8 reviewable + 2 excluded/manually reviewed`, merge `a4aef17a37999d7470912ad8c1a6ea72ae9561a3`, post-merge CI `36411958976`), and governance closeout PR `#55` (qualified head `875b814b9f5307ff39254f65aaec8c4039752d43`, CI `36412559028`, Jev `8/8`, OCR `2 reviewable + 1 excluded/manually reviewed`, merge `e69708aca6b26f16b0351d8ad2313c2bc1d3c9bb`, post-merge CI `36413057165`), all with zero findings, zero blockers, and zero unresolved review threads. The canonical SG-000021 authority adds only isolated browser profile status and origin-bound destination validation as local policy with no browser launch or attachment.

## Closed SG-000022 bounded navigation

SG-000022 closed canonically: activation PR `#56`, implementation PR `#57` (qualified head `2d8ae35db855dd50092efc46a5bd663f20d5d476`, CI `36432067459`, Review Gates `36432064800`, Jev `17/17`, OCR v1.12.9 `7 reviewable + 1 excluded/manually reviewed`, merge `698eff08e736c133b93100e1983fe4be2c5909d0`, post-merge CI `36432538358`), and governance closeout recorded here, all with zero findings, zero blockers, and zero unresolved review threads. The canonical SG-000022 authority adds only typed page lifecycle and SOFT-approved origin-bound navigation with hop-by-hop redirect validation and download-trigger denial in the Cotra registry with no browser launch or attachment.

## Successor frontier

COTRA-P08 continues at this frontier. No successor grain beyond SG-000022 is authorized by this closeout. DOM observation, DOM actuation, scoped downloads, and scoped uploads remain successor work.

## Active grain

None. SG-000022 is `CLOSED` and canonical. The next lawful P08 grain must be derived from canonical P08 requirements with a narrow SpecGrain.

SG-000022 proved that pages live isolated on the Cotra automation profile, that navigation binds exact origin with re-resolution, that redirect chains validate hop by hop, that stale and foreign page identities fail closed, that every navigation carries fresh SOFT approval, and that the agent cannot reach navigation capability through its own tool or input surface.

A P08 exit requires joint proof from SG-000021 isolation and origin binding plus its successors that structured browser operations are proven with SSRF tests, redirect-widening tests, and the personal profile disabled by default. It must not expose personal-profile access, debugging authority, broader egress, or approval bypass.

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Existing SG-000017 destination-scoped push remains subject to its exact workspace policy, short-branch, preview, approval, pinning, credential-reference, transport-hardening, and resulting-ref controls.

Existing SG-000018 replay-resistant approval remains subject to its exact nonce, expiry, one-shot, digest, workspace, and policy bindings with redacted history.

Existing SG-000019 STRONG presence enforcement remains subject to its exact class, presence-verifier, input-lease, nonce, expiry, one-shot, digest, workspace, and policy bindings with class-labeled history.

Existing SG-000020 workspace trust management remains subject to its exact STRONG-gated trust change, epoch-bound revoke, workspace and policy binding, and redacted history controls.

Existing SG-000021 browser profile and destination policy remains subject to its exact isolated-profile, origin-binding, SSRF, and redirect-widening controls with actuation denial.

Existing SG-000022 bounded navigation remains subject to its exact page lifecycle, SOFT-approved origin-bound transitions, hop-by-hop redirect validation, download-trigger denial, and stale foreign drift controls with actuation denial.

Still denied or absent outside the closed SG-000022 bounded-navigation scope:

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Existing SG-000017 destination-scoped push remains subject to its exact workspace policy, short-branch, preview, approval, pinning, credential-reference, transport-hardening, and resulting-ref controls.

Existing SG-000018 replay-resistant approval remains subject to its exact nonce, expiry, one-shot, digest, workspace, and policy bindings with redacted history.

Existing SG-000019 STRONG presence enforcement remains subject to its exact class, presence-verifier, input-lease, nonce, expiry, one-shot, digest, workspace, and policy bindings with class-labeled history.

Existing SG-000020 workspace trust management remains subject to its exact STRONG-gated trust change, epoch-bound revoke, workspace and policy binding, and redacted history controls.

Still denied or absent outside the closed COTRA-P07 scope:
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
- approval bypass, persistent approval reuse, or remote approval delegation.
- soft-button satisfaction of STRONG-class operations;
- STRONG-to-SOFT downgrade when platform verification is unavailable.
- trust-change bypass, revoke bypass, or history tampering.

COTRA-P07 is exited at this frontier, so a later lawful P08 grain must build only on top of the SG-000018 replay-resistant foundation, SG-000019 class enforcement, and SG-000020 trust and revoke records. Raw credentials must never be accepted merely because Git can consume them.

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
