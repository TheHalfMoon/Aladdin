# Cotra Current Canonical Frontier

Status: ACTIVE_GRAIN
Date: 2026-09-27
Governance snapshot base: 3623363603ab2cd2edfb2f456beb84c420983fc8
Evidence ledger: `.specgrain/canonical-evidence.json`

`Governance snapshot base` records the exact canonical parent from which this activation snapshot was authored. It deliberately does not claim the eventual merge SHA of the commit containing this file.

## SpecGrain state semantics

- `GRAIN`: authorized/active packet whose acceptance is not yet canonically proven.
- `PROVEN`: acceptance and required qualification are proven, but canonical closeout/governance is incomplete.
- `CLOSED`: machine state corresponding to Diffcipline `COMPLETE_CANONICAL`.

The machine-readable evidence for every `CLOSED` grain is recorded in `.specgrain/canonical-evidence.json` and validated by CI.

## Closed canonical grains

SG-000001 through SG-000015 are `CLOSED` and canonical.

SG-000015 implementation PR `#35` merged as `08be5a4d572d4b5c6283f63ede26bdc7c0cece14` after exact-head CI `36333639216`, Review Gates `36333636589`, genuine TypeSafe Jev `16/16` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of both excluded files, and zero unresolved review threads. Implementation post-merge CI `36333869446` completed SUCCESS.

SG-000015 governance closeout PR `#36` qualified on exact head `16e11df3638dc6fe03e134466781fd40f25ebc9d` with CI `36334105032` SUCCESS, Review Gates `36334104987` SUCCESS, genuine TypeSafe Jev `7/7` hunk coverage with zero findings/blockers, Alibaba Open Code Review v1.12.9 exact-range delegation plus manual review of excluded `docs/canonical/CURRENT.md`, and zero review threads. It merged normally as canonical main `3623363603ab2cd2edfb2f456beb84c420983fc8`; post-closeout CI `36334237958` completed SUCCESS across Governance, Node/Ubuntu, Node/Windows, Rust/Ubuntu, and Rust/Windows.

The canonical SG-000015 authority is limited to approved local `git.branch.create`, `git.stage`, `git.unstage`, and `git.commit`; Git network transport and push remain absent.

## Active grain

SG-000016 — Bounded Git HTTPS fetch and egress-policy foundation — is the sole active COTRA-P06 grain.

This activation authorizes implementation and qualification of the first destination-scoped Git network capability while deliberately keeping credential and push authority absent.

Authorized target design:
- workspace-bound Git destination policy identified by stable policy id rather than an arbitrary caller URL;
- canonical HTTPS destination only, with DNS hostname, default/explicit port 443, no URL userinfo/query/fragment, and no alternate Git transport syntax;
- public-address resolution/classification followed by pinning of one validated address to the actual Git HTTPS request so DNS rebinding cannot widen authority;
- `git.fetch` of one explicit `refs/heads/<branch>` source into deterministic Cotra-owned `refs/remotes/cotra/<policy-id>/<branch>` state;
- fresh local approval bound to exact workspace/repository/policy/destination/pinned-address/ref/HEAD/prior-ref state;
- post-approval destination/state revalidation;
- hardened Git transport with HTTPS-only protocol policy, redirects disabled, TLS verification enabled, no inherited proxy/cookies/authorization headers/credential helpers/askpass/interactive prompts, and no tags/prune/submodule/FETCH_HEAD side effects;
- rejection of repository-local URL rewrites or equivalent configuration that could widen the approved destination;
- resulting-ref evidence plus proof that checked-out HEAD, current branch, index, and worktree state remain unchanged;
- deterministic resolver/security tests and a no-cost real HTTPS fetch qualification.

## Active acceptance frontier

SG-000016 must prove that Git network access is narrower than generic HTTP, shell, or arbitrary Git authority.

The actual destination used by Git must be both policy-authorized and public-address validated. Merely checking an HTTPS hostname and then allowing Git to resolve it independently is not sufficient. Redirect following is disabled rather than treated as implicit destination expansion.

The caller cannot supply arbitrary remote URLs, arbitrary refspecs, proxy settings, credentials, custom HTTP headers, or transport protocols.

## Explicitly deferred COTRA-P06 authority

Not authorized by SG-000016:
- `git.push`;
- force push;
- any raw credential/token argument;
- ambient Git credential-manager authority;
- credential helper/askpass/interactivity;
- SSH, git://, file://, ext, remote helpers, or HTTP without TLS;
- generic `network.fetch` or arbitrary HTTP/socket authority;
- arbitrary Git argv/refspec/ref mutation;
- branch deletion or destructive history operations;
- tag or submodule mutation.

A later lawful P06 grain must add push only with explicit source/destination refs, expected remote-ref protection, force/non-fast-forward denial, destination revalidation, fresh approval, protected credential-reference handling or explicitly credentialless destination policy, secret redaction, and post-push remote-ref verification. Raw credentials must never be accepted merely because Git can consume them.

## Canonical public authority boundary retained

Existing SG-000015 local Git mutation remains subject to its exact approved-state controls.

Still denied or absent outside the active SG-000016 HTTPS-fetch scope:
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
