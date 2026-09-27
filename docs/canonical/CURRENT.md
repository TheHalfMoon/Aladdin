# Cotra Current Canonical Frontier

Status: ACTIVE_GRAIN
Date: 2026-09-27
Governance snapshot base: d9708b1f2537befaf09db57e94099949b2a4115c
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this activation snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000016 are `CLOSED` and canonical.

SG-000016 implementation PR `#38` merged as `c85b79f24ce2deef46319d9c5790623fd70c3b0d` after exact-head CI `36339733920`, Review Gates `36339732607`, genuine TypeSafe Jev `15/15` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of both excluded files, and zero unresolved review threads. Implementation post-merge CI `36339937619` completed SUCCESS.

SG-000016 governance closeout PR `#39` qualified on exact head `7ad72e3974ba754c4a4860e30f26d1af6aa5ef87` with CI `36340440771` SUCCESS, Review Gates `36340440946` SUCCESS, genuine TypeSafe Jev `6/6` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of excluded `docs/canonical/CURRENT.md`, and zero review threads. It merged normally as canonical main `d9708b1f2537befaf09db57e94099949b2a4115c`; post-closeout CI `36340662627` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

The canonical SG-000016 authority adds only local `git.fetch.preview` and approved destination-scoped anonymous `git.fetch` into `refs/remotes/cotra/<policy-id>/<branch>`; `git.push` and credential authority remain absent.

## Active grain

SG-000017 — Bounded Git push with protected credential references — is the sole active COTRA-P06 grain.

This activation authorizes implementation and qualification of the destination-scoped Git push capability while deliberately keeping ambient credential and generic network authority absent.

Authorized target design:
- workspace-bound Git push destination policy identified by stable policy id rather than an arbitrary caller URL;
- canonical HTTPS destination only, with DNS hostname, default/explicit port 443, no URL userinfo/query/fragment, and no alternate Git transport syntax;
- explicit local source ref and explicit remote destination ref, each restricted to validated `refs/heads/<branch>` short-branch form;
- local-only/read-only stale-protection preview that performs no DNS or network request and returns the exact current HEAD plus deterministic refs and current remote-ref object id/`ABSENT` value;
- public-address resolution/classification followed by pinning of one validated address to the actual Git HTTPS request so DNS rebinding cannot widen authority;
- fresh local approval bound to exact workspace/repository/policy/destination/pinned-address/source-ref/destination-ref/HEAD/prior-ref/credential-reference state;
- post-approval destination/state/credential-binding revalidation;
- non-fast-forward rejection with hard force-push denial and no force, delete, or wildcard refspec path;
- hardened Git transport with HTTPS-only protocol policy, redirects disabled, TLS verification enabled, no inherited proxy/cookies/authorization headers/askpass/interactive prompts, and no tags/prune/submodule side effects;
- protected credential-reference architecture with secret redaction and no ambient credential-manager inheritance;
- rejection of repository-local URL rewrites, credential-helper overrides, or equivalent configuration that could widen or re-authenticate the approved destination;
- resulting remote-ref evidence plus proof that checked-out HEAD, current branch, index, and worktree state remain unchanged except for intended remote-tracking evidence;
- deterministic resolver/security/credential tests and a no-cost real HTTPS push qualification or an explicit unproven record where no no-cost destination exists.

## Active acceptance frontier

SG-000017 must prove that Git push is narrower than generic Git command execution and never leaks raw secrets.

The preview path must make exact stale-protection material obtainable without guessing and without creating hidden network authority. It cannot resolve DNS, contact the configured destination, request approval, or mutate repository state.

The actual destination used by `git.push` must be both policy-authorized and public-address validated. Redirect following is disabled rather than treated as implicit destination expansion.

The caller cannot supply arbitrary remote URLs, arbitrary refspecs, force or delete semantics, proxy settings, raw credentials, custom HTTP headers, or transport protocols.

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
