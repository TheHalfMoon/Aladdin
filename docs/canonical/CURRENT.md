# Cotra Current Canonical Frontier

Status: CLOSEOUT_CANDIDATE
Date: 2026-09-28
Governance snapshot base: ac1e7c98040e3812a5d2e235f89d05bba773ecdd
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this closeout snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000018 are already `CLOSED` and canonical.

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

This closeout candidate records SG-000019 as `CLOSED` using the qualified implementation and merged-tree evidence below. That closeout classification becomes canonical only after this governance change itself merges and the resulting canonical main passes post-merge CI.

SG-000019 implementation evidence:
- implementation PR: `#47`;
- implementation base: `5e1720863dd9127cc2c214aba58e8b55494f8254`;
- qualified head: `b2b5b84fc5a14cf3d056c71eccb44f6f7b2cf674`;
- exact-head CI: `36380608151` — SUCCESS;
- exact-head Review Gates: `36380606179` — SUCCESS;
- implementation merge: `ac1e7c98040e3812a5d2e235f89d05bba773ecdd`;
- implementation post-merge CI: `36380978676` — SUCCESS.

Genuine TypeSafe Jev reviewed the exact implementation range with `45/45` hunks, zero findings, and zero blocking findings. Alibaba Open Code Review v1.12.9 delegated the same exact range across 7 changed files: 6 reviewable files and one exclusion (`docs/security/SG-000019_STRONG_PRESENCE_NOTE.md` by unsupported-extension classification). The excluded file was manually reviewed. There were zero unresolved blocking review threads.

Manual exact-diff security review confirmed the STRONG class is explicit with no SOFT downgrade, platform-mediated presence through a broker-held verifier with suspended input lease, fail-closed unavailable cancelled timeout malformed replay and drift handling, one-shot expiry digest workspace and policy binding retained for both classes, class-labeled checksum-chained redacted history without secrets, approve-like denial retained, no trust-change reuse delegation browser UI network elevation or synthetic-input authority, and retained P06 regressions.

The canonical SG-000019 authority adds only the STRONG approval class with platform-mediated presence verification for PRIVILEGED and selected DESTRUCTIVE operations with fail-closed unavailable handling; existing file, process, and Git operations retain the SOFT class, and trust changes, persistent reuse, remote delegation, and approval bypass remain absent.

## Successor frontier

No successor grain is authorized by this closeout candidate.

After this SG-000019 closeout merges and its post-merge CI succeeds, re-read the canonical architecture, delivery plan, threat model, evidence ledger, open governance records, and this `CURRENT.md` to derive the next lawful COTRA-P07 grain for workspace trust management, approval history UX, and emergency revoke. Do not infer or pre-authorize the next SpecGrain identifier or exact trust design from numbering alone.

A successor trust-UX grain must build on the SG-000018 replay-resistant foundation and SG-000019 class enforcement without weakening STRONG enforcement, one-shot, expiry, or digest binding. It must not expose browser automation, UI automation, elevation, or approval bypass.

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Existing SG-000017 destination-scoped push remains subject to its exact workspace policy, short-branch, preview, approval, pinning, credential-reference, transport-hardening, and resulting-ref controls.

Existing SG-000018 replay-resistant approval remains subject to its exact nonce, expiry, one-shot, digest, workspace, and policy bindings with redacted history.

Still denied or absent outside the active SG-000019 strong-presence scope:

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Existing SG-000017 destination-scoped push remains subject to its exact workspace policy, short-branch, preview, approval, pinning, credential-reference, transport-hardening, and resulting-ref controls.

Still denied or absent outside the active SG-000018 approval-hardening scope:
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

A later lawful grain must add workspace trust management and history UX only on top of the SG-000018 replay-resistant foundation and SG-000019 class enforcement. Raw credentials must never be accepted merely because Git can consume them.

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
