# Cotra Current Canonical Frontier

Status: CLOSEOUT_CANDIDATE
Date: 2026-09-28
Governance snapshot base: e40d7ad255566025617557bbd8f9fb91f1595550
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000019 are already `CLOSED` and canonical.

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

## Closeout candidate

This closeout candidate records SG-000020 as `CLOSED` using the qualified activation, implementation, and merged-tree evidence below. That closeout classification becomes canonical only after this governance change itself merges and the resulting canonical main passes post-merge CI.

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

## Successor frontier

No successor grain is authorized by this closeout candidate.

After this SG-000020 closeout merges and its post-merge CI succeeds, re-read the canonical architecture, delivery plan, threat model, evidence ledger, open governance records, and this `CURRENT.md` to rebuild the full COTRA-P07 gap matrix and derive the next lawful unit. Do not infer or pre-authorize the next SpecGrain identifier or exact design from numbering alone.

A P07 exit requires joint proof from SG-000018 replay resistance, SG-000019 STRONG enforcement, and SG-000020 trust and revoke records that the agent cannot produce a valid strong approval or trust change through its own tool or input surface with replay and drift resistance intact. It must not expose browser automation, UI automation, elevation, or approval bypass.

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Existing SG-000017 destination-scoped push remains subject to its exact workspace policy, short-branch, preview, approval, pinning, credential-reference, transport-hardening, and resulting-ref controls.

Existing SG-000018 replay-resistant approval remains subject to its exact nonce, expiry, one-shot, digest, workspace, and policy bindings with redacted history.

Existing SG-000019 STRONG presence enforcement remains subject to its exact class, presence-verifier, input-lease, nonce, expiry, one-shot, digest, workspace, and policy bindings with class-labeled history.

Existing SG-000020 workspace trust management remains subject to its exact STRONG-gated trust change, epoch-bound revoke, workspace and policy binding, and redacted history controls.

Still denied or absent outside the closed SG-000020 trust scope:

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Existing SG-000017 destination-scoped push remains subject to its exact workspace policy, short-branch, preview, approval, pinning, credential-reference, transport-hardening, and resulting-ref controls.

Existing SG-000018 replay-resistant approval remains subject to its exact nonce, expiry, one-shot, digest, workspace, and policy bindings with redacted history.

Existing SG-000019 STRONG presence enforcement remains subject to its exact class, presence-verifier, input-lease, nonce, expiry, one-shot, digest, workspace, and policy bindings with class-labeled history.

Existing SG-000020 workspace trust management remains subject to its exact STRONG-gated trust change, epoch-bound revoke, workspace and policy binding, and redacted history controls.

Still denied or absent outside the closed SG-000020 trust scope:
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

A later lawful grain must complete P07 exit only on top of the SG-000018 replay-resistant foundation, SG-000019 class enforcement, and SG-000020 trust and revoke records. Raw credentials must never be accepted merely because Git can consume them.

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
