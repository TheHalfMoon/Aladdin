# Cotra Current Canonical Frontier

Status: ACTIVE_GRAIN
Date: 2026-09-29
Governance snapshot base: f0383ef357072e1f5ab2e32f5b0bdf527a109f33
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this activation snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000027 are `CLOSED` and canonical.

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

## Closed SG-000023 DOM observation

SG-000023 closed canonically: activation PR `#59` (activation base `28e0d912b88961134a41bda87b3545211f06f6e9`, activation head `51b64d26272c42acabda943f6ff5bc011af32020`, CI `36435248806`, Review Gates `36435249467`, Jev `3/3`, OCR v1.12.9 `1 reviewable + 1 excluded/manually reviewed`, merge `0ab6ea79bc5e1b2aa417f6b5f94e8c61861ea7bc`, post-merge CI `36435581576`), implementation PR `#60` (qualified head `5ae3e03ee02b199ee35cd77331b4199f9c791a0a`, CI `36437199187`, Review Gates `36437195899`, Jev `16/16`, OCR v1.12.9 `7 reviewable + 1 excluded/manually reviewed`, merge `6b1259a29a4f3350597fbac5d9cfe5c64f86a8c4`, post-merge CI `36437754602`), and governance closeout recorded here, all with zero findings, zero blockers, and zero unresolved review threads. The canonical SG-000023 authority adds only read-only snapshot observation with typed node identity as a local read of an already-authorized active page and origin in the Cotra registry with no browser launch or attachment.

## Closed SG-000024 DOM actuation

SG-000024 closed canonically: activation PR `#62` (activation base `c40b72a55084e31cfd33e8c23aeaa06c3ec7853a`, activation head `d1224f5a58f48ffb647f9499a41aa6491855e21c`, CI `36438889230`, Review Gates `36438888956`, Jev `3/3`, OCR v1.12.9 `1 reviewable + 1 excluded/manually reviewed`, merge `03d95ec0e411e2cd3d5abb78caad8425c8d8b988`, post-merge CI `36439181347`), implementation PR `#63` (qualified head `421139092fa4aa8c9dc545240b48cd69b4398978`, CI `36441194364`, Review Gates `36441189927`, Jev `24/24`, OCR v1.12.9 `9 reviewable + 1 excluded/manually reviewed`, merge `0a42c9e440e451023518496cadb9afc1952b30c4`, post-merge CI `36441495286`), and governance closeout recorded here, all with zero findings, zero blockers, and zero unresolved review threads. The canonical SG-000024 authority adds only structured invoke and value-entry actuation on typed node identities with per-action SOFT approval in the Cotra registry with no browser launch or attachment.

## Closed SG-000025 scoped downloads

SG-000025 closed canonically: activation PR `#65` (activation base `da637f5e4c823cb486ab30a521f003d9baf8bd55`, activation head `19be90f34d35a494958d5e75c2bfd6dae941e1cd`, CI `36457618398`, Review Gates `36457618307`, Jev `5/5`, OCR v1.12.9 `1 reviewable + 1 excluded/manually reviewed`, merge `d0520b5bdda52da24827d6e3a4f26b9e4d6e8061`, post-merge CI `36463042632`), implementation PR `#66` (qualified head `52912c3cad6ac6d407cb9862f4fad6402fcc78fc`, CI `36466971400`, Review Gates `36466968515`, Jev `20/20` with zero findings and zero blocking findings, OCR v1.12.9 `10 reviewable + 1 excluded/manually reviewed`, merge `f2a19535295158a40c7cf05a002b1b6201c6aabf`, post-merge CI `36467256256`), and governance closeout recorded here, all with zero unresolved review threads. The canonical SG-000025 authority adds only scoped bounded `browser.download/preview` and `browser.download/download` into the approved workspace download root, with no browser launch, no attachment, and no Cotra network transfer.

SG-000025 proved that the destination root is always the approved workspace root derived from policy configuration and is never caller-selected; that canonical relative destinations deny absolute, drive, UNC, device, NT namespace, alternate data stream, parent and current traversal, empty and duplicate separators, trailing dot and space, reserved device name, illegal character, control character, and length escapes; that containment uses canonical filesystem identity with a separator-boundary comparison so a Windows directory junction and a Unix symlink cannot extend the approved root; that writes are create-only with no directory creation, no overwrite, no rename, no delete, and no truncate, plus post-write real-path, byte length, and SHA-256 re-verification with revert on mismatch; that an extension allowlist plus independent content sniffing denies PE, ELF, Mach-O, OLE compound, shell-script, ZIP, and PDF classes and requires filename, declared type, and content agreement; that declared and actual size must agree within 8 MiB behind strict bounded base64 decoding applied before any filesystem access; that download source identities are server-allocated, one-shot, and expiring after 120 seconds; that every download carries fresh SOFT approval with digest binding over the complete binding set including the actual content digest; that unauthorized origins and redirect widening fail closed; that downloaded content is never executed, opened, extracted, or launched; that evidence and the download registry are bounded and secret-free, recording only the source origin and a source URL digest rather than the raw URL; and that the agent cannot reach download capability through its own tool or input surface.

## Closed SG-000026 scoped uploads

SG-000026 closed canonically: activation PR `#68` (activation base `c37579332f2755f8480c337e88cacef765677b6c`, activation head `f6d7635b9410f8c5f55605eca600d8652d0ba21a`, CI `36469132408`, merge `86e4f2a9f58ed37b7753badd7d5dc42faaa1b5c6`), implementation PR `#69` (qualified head `5e9e09efafbddbbcc6e2050a3d432f77164a747f`, CI `36478991825`, Review Gates `36478988310`, Jev `26/26` with zero findings and zero blocking findings, OCR v1.12.9 `8 reviewable + 1 excluded/manually reviewed`, merge `d35049bb39292b78e515a3b32c0ec88c30e2c877`, post-merge CI `36479358381`), and governance closeout PR `#70` (qualified head `1082a40820262c78f77ed9a9771128f35ac255a6`, CI `36480615368`, Review Gates `36480610977`, Jev `10/10` with zero findings and zero blocking findings, OCR v1.12.9 `3 reviewable + 1 excluded/manually reviewed`, merge `75181787aec4282c3783e0d1e95b05c18e192f26`, post-merge CI `36568753846`), all with zero unresolved review threads. The canonical SG-000026 authority adds only scoped bounded `browser.upload/preview` and `browser.upload/submit` of one already recorded approved download artifact to one enabled typed file-input node, with no browser launch, no attachment, no page byte transfer, and no form submission.

SG-000026 proved that no upload shape accepts a path, file, content, directory, recursive, glob, or page-transfer field of any kind, so a caller can never name the file to read and the capability is strictly weaker than generic filesystem read; that the only admissible source is a file already recorded by the SG-000025 download registry inside the approved workspace download root, carrying a recorded content digest, so credential files, browser profile files, OS secret stores, user home files, workspace-authored files, and unrelated project files are unreachable by construction rather than by policy; that source identity shape is strictly validated as a `dl-` prefix with 64 lowercase hex characters and an upload source identity as a `ul-` prefix with 32, in both the policy and the dispatch layer, so a caller-typed fragment can never reach a registry lookup; that containment uses canonical filesystem identity of both the source parent and the source file with a separator-boundary comparison, so a Windows directory junction and a Unix symlink cannot redirect the read; that the source must be a regular file with a nonzero size within the hard maximum, and that byte length plus SHA-256 are re-verified against the recorded download, so a removed, replaced, or mutated artifact fails closed; that the target must be one known enabled `textbox`-role node with a `file` input type whose typed node identity is recomputed against the current profile, page, generation, origin, index, and policy revision, with wrong role, wrong input type, disabled state, replaced input, stale node, wrong page, wrong origin, wrong generation, and wrong workspace all failing closed; that filling a file input is denied, so the newly observable file input cannot be reached by typing a path and SG-000024 value entry does not widen; that workspace trust revision is bound into both the upload source identity and the approval digest, so an emergency revoke, a trust change, and a trust-revision change between preview and submit all fail closed; that upload source identities are server-allocated, one-shot, and deterministically expiring with bounded pending holds; that every upload carries fresh SOFT approval with digest binding over the complete binding set including the actual source content digest, spent before the upload is recorded; and that evidence is fixed-shape and secret-free, reporting `page_transfer_performed`, `executed`, `opened`, `extracted`, `cookies`, and `credentials` as `false` and carrying no file content, bytes, path, cookie, token, or session material.

## COTRA-P08 exit

COTRA-P08 is exited at this frontier. Structured browser operations are closed under SG-000021, SG-000022, SG-000023, SG-000024, SG-000025, and SG-000026, with isolated profile and origin binding plus successors proving SSRF denial, redirect-widening denial, and personal profile disabled by default, without exposing personal-profile access, debugging authority, broader egress, or approval bypass.

This program-exit snapshot records SG-000026 as merged and canonical. The `CLOSED` classification above becomes part of the canonical frontier only after this program-exit change itself merges and the resulting canonical main passes post-merge CI.

Exit matrix (each criterion is COMPLETE with canonical evidence in `.specgrain/canonical-evidence.json` and the grain blocks above):

Isolated profile - COMPLETE:
- isolated Cotra automation profile with fresh storage and no personal data via SG-000021 (activation PR `#53`, implementation PR `#54`, qualified head `f9912d7a4be7a822954e5dc260be5e698adf9ff8`, CI `36411407342`, Review Gates `36411407224`, Jev `10/10`, OCR v1.12.9 `8 reviewable + 2 excluded/manually reviewed`, merge `a4aef17a37999d7470912ad8c1a6ea72ae9561a3`, post-merge CI `36411958976`; governance closeout PR `#55`, qualified head `875b814b9f5307ff39254f65aaec8c4039752d43`, CI `36412559028`, Jev `8/8`, OCR `2 reviewable + 1 excluded/manually reviewed`, merge `e69708aca6b26f16b0351d8ad2313c2bc1d3c9bb`, post-merge CI `36413057165`).

Origin binding and egress controls - COMPLETE:
- exact scheme, host, and port binding with DNS re-resolution and post-resolution policy via SG-000021 with the same implementation evidence above;
- SSRF denial for loopback, link-local, private, unspecified, multicast, and mapped IPv6 via SG-000021 with the same implementation evidence above;
- redirect-widening denial with expected-origin exact binding and full revalidation via SG-000021 and SG-000022 (implementation PR `#57`, qualified head `2d8ae35db855dd50092efc46a5bd663f20d5d476`, CI `36432067459`, Review Gates `36432064800`, Jev `17/17`, OCR v1.12.9 `7 reviewable + 1 excluded/manually reviewed`, merge `698eff08e736c133b93100e1983fe4be2c5909d0`, post-merge CI `36432538358`; governance closeout PR `#58`);
- typed page lifecycle with SOFT-approved origin-bound navigation and hop-by-hop redirect validation via SG-000022 with the same implementation evidence above.

DOM and accessibility - COMPLETE:
- read-only snapshot observation with typed node identity via SG-000023 (activation PR `#59`, implementation PR `#60`, qualified head `5ae3e03ee02b199ee35cd77331b4199f9c791a0a`, CI `36437199187`, Review Gates `36437195899`, Jev `16/16`, OCR v1.12.9 `7 reviewable + 1 excluded/manually reviewed`, merge `6b1259a29a4f3350597fbac5d9cfe5c64f86a8c4`, post-merge CI `36437754602`; governance closeout PR `#61`);
- structured invoke and value-entry actuation on typed node identities with per-action SOFT approval via SG-000024 (activation PR `#62`, implementation PR `#63`, qualified head `421139092fa4aa8c9dc545240b48cd69b4398978`, CI `36441194364`, Review Gates `36441189927`, Jev `24/24`, OCR v1.12.9 `9 reviewable + 1 excluded/manually reviewed`, merge `0a42c9e440e451023518496cadb9afc1952b30c4`, post-merge CI `36441495286`; governance closeout PR `#64`).

Download and upload scope - COMPLETE:
- scoped bounded downloads into the approved workspace download root with canonical relative destination, create-only write, type and size allowlist, same-origin and redirect-widening denial, one-shot expiring source identity, fresh SOFT digest-bound approval, and non-execution via SG-000025 (activation PR `#65`, implementation PR `#66`, qualified head `52912c3cad6ac6d407cb9862f4fad6402fcc78fc`, CI `36466971400`, Review Gates `36466968515`, Jev `20/20`, OCR v1.12.9 `10 reviewable + 1 excluded/manually reviewed`, merge `f2a19535295158a40c7cf05a002b1b6201c6aabf`, post-merge CI `36467256256`; governance closeout PR `#67`);
- scoped bounded uploads of one recorded approved download artifact to one enabled typed file-input node with canonical identity containment, digest and size re-verification, one-shot expiring source identity, workspace trust binding, fresh SOFT digest-bound approval, and no page transfer via SG-000026 (activation PR `#68`, implementation PR `#69`, qualified head `5e9e09efafbddbbcc6e2050a3d432f77164a747f`, CI `36478991825`, Review Gates `36478988310`, Jev `26/26`, OCR v1.12.9 `8 reviewable + 1 excluded/manually reviewed`, merge `d35049bb39292b78e515a3b32c0ec88c30e2c877`, post-merge CI `36479358381`; governance closeout PR `#70`, qualified head `1082a40820262c78f77ed9a9771128f35ac255a6`, CI `36480615368`, Review Gates `36480610977`, Jev `10/10`, OCR v1.12.9 `3 reviewable + 1 excluded/manually reviewed`, merge `75181787aec4282c3783e0d1e95b05c18e192f26`, post-merge CI `36568753846`).

Personal profile disabled - COMPLETE:
- personal-profile mode denied by default with no personal cookies, credentials, passwords, or sessions via SG-000021 with the same implementation evidence above, retained through SG-000022, SG-000023, SG-000024, SG-000025, and SG-000026 with no personal-profile widening.

Authority separation - COMPLETE:
- structured browser authority remains separate from approval authority with fresh SOFT approval and digest binding for navigation, actuation, download, and upload, STRONG enforcement retained, and no new network, UI automation, clipboard, installer, elevation, debugging, scripting, credential, coordinate, screenshot, or delegation authority beyond the dedicated browser registry APIs recorded in the six grain authority boundaries above.

All P08 closeouts (SG-000021 PR `#55`, SG-000022 PR `#58`, SG-000023 PR `#61`, SG-000024 PR `#64`, SG-000025 PR `#67`, SG-000026 PR `#70`) merged normally with zero unresolved blocking review threads and successful post-merge CI.

## Closed SG-000027 UIA observation

SG-000027 closed canonically: activation PR `#72` (activation base `f9c708075df1c6b33841ba3a6fa452147f60e24c`, activation head `b0130b5598930e23a52d876c7b0b9d7e645425f5`, CI `36572228900`, Review Gates `36572228837`, Jev `3/3`, OCR v1.12.9 `1 reviewable + 1 excluded/manually reviewed`, merge `9621ebfb1764324ecfe794b4b2462d1a0469c365`, post-merge CI `36572575368`), implementation PR `#73` (qualified head `8f07fc36e33fe5bbcbab141e6a79e61081cf00c7`, CI `36576000440`, Review Gates `36576000906`, Jev `12/12` with zero findings and zero blocking findings, OCR v1.12.9 `9 reviewable + 2 excluded/manually reviewed`, merge `e74b2d2a2b5517108775ab86d0b1efe618a74c20`, post-merge CI `36576544932`), and governance closeout recorded here, all with zero unresolved review threads. The canonical SG-000027 authority adds only read-only `uia.process/observe`, `uia.window/list`, `uia.window/observe`, `uia.tree/observe`, and `uia.element/observe` against a process-lifetime typed identity registry, with no process launch, no input injection, and no network egress.

SG-000027 proved that no UIA shape accepts caller-supplied PID, window handle, UIA runtime id, selector, coordinate, approval material, or secret fields, so callers can only present server-allocated typed identities; that process identities bind PID, executable digest, process generation, session identity, workspace, and policy revision with PID reuse and restart fail-closed; that window identities bind owning process, handle, window generation, session, desktop, workspace, and policy revision with destroyed and reused handles fail-closed; that element identities bind owning window, runtime id, tree generation, control type, workspace, and policy revision with disappeared, replaced, and role-changed elements fail-closed; that tree observation is bounded by window count, depth, node count, response size, and string length with explicit truncation reporting; that password and secret bearing values are redacted and never enter records, logs, or evidence; that protected Cotra approval surfaces are omitted from listings and denied on direct observation; that every invoke, click, value, select, toggle, scroll, focus, keyboard, mouse, SendInput, coordinate, screenshot, clipboard, network, process-spawn, injection, termination, and elevation shape fails closed; that the native adapter proves real process identity against Windows APIs on Windows while reporting the live desktop as unavailable instead of fabricating windows; and that the agent cannot reach UIA capability through its own tool or input surface.

## Closed SG-000028 UIA invoke

SG-000028 closed canonically: activation PR `#75` (activation base `592c96fdeb21669e038cc4d65572294103b4992f`, activation head `ecfab6986d4561af4aeeb83ae9a8916c8837d146`, CI `36580610712`, Review Gates `36580610725`, Jev `3/3`, OCR v1.12.9 `1 reviewable + 1 excluded/manually reviewed`, merge `b33da3eb953ab3350e83da28f7459ff6a5fa7fa2`, post-merge CI `36581545389`), implementation PR `#76` (qualified head `50bd49a48610fbbd51ee3d47df6c262ae852ee19`, CI `36582708688`, Review Gates `36582708941`, Jev `18/18` with zero findings and zero blocking findings, OCR v1.12.9 `7 reviewable + 1 excluded/manually reviewed`, merge `8a8688cc5416b064569233e49d6df7bd8763a883`, post-merge CI `36583168629`), and governance closeout recorded here, all with zero unresolved review threads. The canonical SG-000028 authority adds only structured `uia.element/invoke` on exact typed Button, Hyperlink, MenuItem, and SplitButton elements with Invoke pattern support and enabled state against the process-lifetime typed identity registry, with fresh SOFT digest-bound approval and immediate stale-target revalidation, and with no coordinate, synthetic-input, screenshot, clipboard, network, or elevation authority.

SG-000028 proved that no invoke shape accepts caller-supplied PID, window handle, UIA runtime id, selector, coordinate, approval material, secret, value, or fallback fields, so callers can only present one server-allocated typed element plus expected tree generation and expected control type; that invoke binds server-derived process identity, typed window identity, and typed element identity with PID reuse, restart, destroyed and reused handles, disappeared, replaced, and role-changed elements fail-closed; that only Button, Hyperlink, MenuItem, and SplitButton elements with Invoke support and enabled state actuate while other control types, unsupported patterns, disabled elements, password and secret elements, and protected Cotra surfaces are denied; that every invoke carries fresh SOFT approval with exact digest binding over workspace, policy revision, process, window, element, generations, control type, and action material with one-shot consumption and stale-digest fail-closed; that immediate pre-actuation revalidation denies process, window, element, state, protected-surface, workspace, and policy drift without silent retargeting and without fallback to mouse, keyboard, SendInput, coordinates, screenshots, clipboard, network, or elevation; that successful invoke advances the window tree generation so stale identities cannot be replayed; that the native adapter proves real process identity against Windows APIs on Windows while reporting live invoke as unavailable instead of fabricating actuation; and that the agent cannot reach invoke through its own tool or input surface.

## Successor frontier

COTRA-P09 continues at this frontier. No successor grain beyond SG-000029 is authorized by this activation.

## Active grain

SG-000029 - Structured Windows UI Automation ValuePattern actuation with approval binding and stale-target revalidation - is the sole active COTRA-P09 grain.

This activation authorizes implementation and qualification of a single narrow structured value actuation shape on top of the SG-000027 observation registry, the SG-000028 invoke registry, the SG-000018 replay-resistant foundation, SG-000019 class enforcement, SG-000020 trust and revoke records, and the closed COTRA-P08 registry: uia.element/set_value bound to one server-allocated typed element with arguments element_id, expected_tree_generation, expected_control_type, and value only; server-derived process identity with PID reuse and restart denial; typed window identity with destroyed, replaced, and reused-handle denial; typed element identity with disappeared, replaced, role-changed, and generation-changed denial; Value eligibility limited to Edit, Document, and ComboBox with required Value pattern support and required enabled state; bounded values of at most 1024 characters with password, secret, protected-surface, and oversized denial; fresh per-action SOFT approval with one-shot exact-digest binding over workspace, policy revision, process, window, element, generations, control type, value digest, and action material; immediate pre-actuation revalidation with no silent retargeting; protected Cotra approval-surface exclusion; and explicit denial of keyboard fallback, mouse fallback, SendInput fallback, invoke, select, toggle, scroll, focus, coordinates, screenshots, clipboard, network, and elevation, while deliberately keeping SelectionPattern, TogglePattern, and ScrollPattern as successor work.

Authorized target design:
- uia.element/set_value bound to one typed element with exact process pairing, window pairing, tree generation, runtime identity, control type, Value pattern, enabled state, bounded value, workspace, and policy revision;
- fresh SOFT approval with digest binding including the value digest and one-shot consumption;
- immediate pre-actuation stale-target revalidation with TargetStale denial on any drift;
- protected Cotra approval-surface exclusion with no value path reaching presence or approval material;
- password, secret, disabled, oversized, and ineligible targets denied with no keyboard fallback, and deterministic unit and security tests for happy-path value, stale process, restarted process, PID reuse, stale and reused HWND, wrong process and window pairing, stale element, wrong runtime identity, wrong tree generation, changed control type, unsupported pattern, disabled element, protected surface, password element, oversized value, missing and stale and reused and digest-mismatched approval, policy drift, state drift, caller-supplied identities, selector and coordinate requests, fallback requests, and elevation requests.

## Active acceptance frontier

SG-000029 must prove that set_value binds server-derived process, typed window, and typed element identity and rejects caller-provided claims, that only Edit, Document, and ComboBox elements with Value support and enabled state actuate with bounded non-secret values, that every set_value carries fresh SOFT digest-bound approval including the value digest with one-shot consumption, that immediate pre-actuation revalidation fails closed on any drift without silent retargeting and without fallback to keyboard, mouse, SendInput, coordinates, screenshots, clipboard, network, or elevation, that protected Cotra surfaces and password targets are excluded, that the agent cannot reach value through its own tool or input surface, and that Windows-specific qualification proves structured binding without fabricating interactive desktop evidence.

Select, toggle, scroll, focus, generic input injection, coordinate fallback, screenshots, visual proposals, clipboard access, generic network authority, persistent approval reuse, remote delegation, and elevation must remain absent, and STRONG enforcement, one-shot, expiry, and digest binding must not weaken.

A P09 exit requires joint proof from SG-000027 observation plus SG-000028 invoke plus SG-000029 value plus select, toggle, and scroll successors that Cotra windows are protected, structured action is preferred, and expected process and window identity is enforced. It must not expose generic input injection, coordinate fallback, or elevation authority.

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Existing SG-000016 destination-scoped fetch remains subject to its exact workspace policy, short-branch, preview, approval, pinning, transport-hardening, and resulting-ref controls.

Existing SG-000017 destination-scoped push remains subject to its exact workspace policy, short-branch, preview, approval, pinning, credential-reference, transport-hardening, and resulting-ref controls.

Existing SG-000018 replay-resistant approval remains subject to its exact nonce, expiry, one-shot, digest, workspace, and policy bindings with redacted history.

Existing SG-000019 STRONG presence enforcement remains subject to its exact class, presence-verifier, input-lease, nonce, expiry, one-shot, digest, workspace, and policy bindings with class-labeled history.

Existing SG-000020 workspace trust management remains subject to its exact STRONG-gated trust change, epoch-bound revoke, workspace and policy binding, and redacted history controls.

Existing SG-000021 browser profile and destination policy remains subject to its exact isolated-profile, origin-binding, SSRF, and redirect-widening controls with actuation denial.

Existing SG-000022 bounded navigation remains subject to its exact page lifecycle, SOFT-approved origin-bound transitions, hop-by-hop redirect validation, download-trigger denial, and stale foreign drift controls with actuation denial.

Existing SG-000023 DOM observation remains subject to its exact active-page binding, typed node identity, bounded data-minimized snapshot, password and secret redaction, and stale foreign drift controls with actuation denial.

Existing SG-000024 DOM actuation remains subject to its exact node binding, role and state matching, per-action SOFT approval, generation-bump invalidation, bounded non-password values, password-fill denial, and stale foreign drift controls with extended-verb denial and no coordinate fallback.

Existing SG-000025 scoped bounded downloads remain subject to their exact active-page binding, approved workspace download root, canonical relative destination, create-only write, type and size allowlist, same-origin and redirect-widening denial, one-shot expiring source identity, fresh SOFT digest-bound approval, bounded secret-free evidence, and downloaded-file non-execution controls.

Existing SG-000026 scoped bounded uploads remain subject to their exact active-page binding, exact origin and generations, workspace and policy revision, one recorded approved download artifact as the only admissible source, canonical filesystem identity containment with separator-boundary comparison, byte length and SHA-256 re-verification, one known enabled typed file-input target, one-shot expiring upload source identity, workspace trust binding, fresh SOFT digest-bound approval, bounded secret-free evidence reporting no page transfer, and file-input fill, directory upload, multiple-file upload, archive extraction, execution, and page byte transfer denial.

Retained SG-000027 read-only UIA observation remains subject to its exact server-derived process identity, typed window and element identity, expected-generation binding, workspace and policy revision, bounded data-minimized snapshots, password and secret redaction, protected Cotra surface exclusion, stale-identity fail-closed behavior, and value, select, toggle, scroll, focus, coordinate, synthetic-input, screenshot, clipboard, network-egress, and elevation denial beyond the SG-000028 invoke scope.

Retained SG-000028 structured UIA invoke remains subject to its exact server-derived process binding, typed window and element binding, expected tree generation, expected Button, Hyperlink, MenuItem, and SplitButton control type, required Invoke pattern support, required enabled state, workspace and policy revision, fresh per-action SOFT digest-bound approval with one-shot consumption, immediate pre-actuation stale-target revalidation with generation-bump invalidation, protected Cotra surface exclusion, password and secret target denial, and coordinate, mouse, keyboard, SendInput, click fallback, value, select, toggle, scroll, focus, screenshot, clipboard, network-egress, and elevation denial.

Still denied or absent outside the closed COTRA-P07 scope, the closed COTRA-P08 scope, and the SG-000027 observation plus SG-000028 invoke scope:
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
- generic browser automation beyond the closed structured registry (launch, attachment, debugging, scripting);
- Windows UI Automation value, select, toggle, scroll, focus, and input injection beyond the closed observation plus structured invoke registry;
- elevation;
- approval bypass, persistent approval reuse, or remote approval delegation.
- soft-button satisfaction of STRONG-class operations;
- STRONG-to-SOFT downgrade when platform verification is unavailable.
- trust-change bypass, revoke bypass, or history tampering.
- caller-selected upload sources, any upload path field, and uploads of files not recorded by the SG-000025 download registry;
- directory upload, multiple-file upload, and filling a file input;
- page byte transfer and form submission;
- archive extraction and decompression;
- execution, opening, or shell launch of downloaded content;
- automatic or silent download behavior on navigation or actuation;
- caller-selected download destinations, absolute destinations, and destinations outside the approved workspace root;
- download persistence of cookies, credentials, tokens, session material, or personal browser state.

COTRA-P07 and COTRA-P08 are exited at this frontier, so a later lawful P09 grain must build only on top of the SG-000018 replay-resistant foundation, SG-000019 class enforcement, SG-000020 trust and revoke records, and the closed P08 structured-browser registry. Raw credentials must never be accepted merely because Git can consume them.

Windows public `process.spawn` remains positively restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Architecture: Cotra is standalone; Kernux is not a dependency.

Evidence rule: never claim PROVEN or CLOSED canonical without required exact-head, platform, merge, post-merge, and governance evidence.

Qualification rule: genuine TypeSafe Jev and Alibaba Open Code Review are required qualification evidence where integrated. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.

Language rule: all repository technical content is English only.
