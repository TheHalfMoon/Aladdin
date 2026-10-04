# Qdral Diffcipline

Status: ACTIVE PLANNING RULE
Date: 2026-09-23

Diffcipline is Qdral's exact-diff execution discipline. Its purpose is to prevent useful work from becoming unauditable work.

## 1. Core rule

Every implementation unit must be reviewable as one bounded claim.

A PR must answer:
- what exact behavior is being added or changed;
- what authority changes;
- what files changed;
- what tests prove the claim;
- what remains explicitly unproven.

## 2. Exact-live start

Before authoring:
1. fetch canonical main;
2. record exact main SHA;
3. record clean/dirty worktree state when working locally;
4. inspect open PRs that may overlap;
5. confirm the active SpecGrain packet;
6. create a forward-only branch from the exact base.

Do not rely on a handoff SHA without re-verification.

## 3. Scope budget

Default target per implementation packet:
- one capability or one coherent cross-cutting contract;
- <= 12 changed files unless justified;
- <= 600 net added lines unless generated code/tests make a documented exception;
- no unrelated formatting;
- no drive-by dependency upgrades;
- no donor import outside the packet's provenance scope.

The bounds are reviewability defaults, not incentives to hide necessary tests.

## 4. File discipline

Every changed file must map to:
- implementation;
- test;
- contract/schema;
- evidence/governance;
- required dependency metadata.

If a file has no mapping, remove it from the PR or explain it.

## 5. Authority delta

Every PR must explicitly state whether it changes:
- file authority;
- process authority;
- network authority;
- browser authority;
- UI/input authority;
- secret authority;
- approval authority;
- privilege/elevation authority.

Security review is mandatory when any authority expands.

## 6. Dependency discipline

A new dependency requires:
- purpose;
- version/pin;
- license;
- maintenance signal;
- network behavior;
- privilege implications;
- safer alternative considered.

Lockfile changes must correspond exactly to declared dependency changes.

## 7. Donor provenance

For copied/adapted source:
- donor repository;
- exact commit;
- donor path;
- destination path;
- license;
- attribution requirement;
- copied/adapted/concept-only classification;
- local modifications.

No undocumented copy-paste.

## 8. Test discipline

Tests must include:
- happy path;
- denial path;
- malformed input;
- stale/race path when relevant;
- security regression for expanded authority.

Windows-specific claims require Windows-native evidence.

## 9. Exact-head qualification

Before merge:
- record PR head SHA;
- run required CI against that exact head;
- record run IDs;
- verify no new commit appeared after qualification;
- inspect changed-file list and diff stat;
- inspect unresolved review threads;
- run semantic review.

If head moves, qualification resets.

## 10. Review stack

Required as applicable:
1. automated format/lint/type/test;
2. SpecGrain acceptance check;
3. Diffcipline exact-diff check;
4. Jev-style semantic architecture/security review;
5. Alibaba Open Code Review;
6. targeted human/founder approval for governance-sensitive merge;
7. post-merge verification.

Automated reviewers do not replace security reasoning.

## 11. Merge discipline

Never merge:
- a disqualified head;
- a PR with unresolved blocking findings;
- a PR whose authority delta is undocumented;
- a PR whose Windows-only claim lacks Windows evidence;
- a PR that silently widens workspace/network/approval authority.

Use expected-head SHA when the merge mechanism supports it.

## 12. Post-merge evidence

After merge:
- record merge SHA;
- verify canonical main contains the intended tree;
- run/observe post-merge CI when configured;
- close or advance the SpecGrain packet only after effectiveness evidence exists.

Merged is not automatically equivalent to proven.

## 13. Failure behavior

If a gate cannot be run:
- say UNPROVEN;
- preserve evidence;
- do not invent success;
- do not compensate by widening scope.

If a security test exposes an architectural flaw:
- stop that authority expansion;
- repair the design before proceeding.

## 14. Planning PR rule

Planning-only PRs:
- do not claim implementation;
- do not advance runtime completion state;
- may define future authority but do not grant it;
- must identify any unresolved architectural question.

## 15. Canonical completion

A task is COMPLETE_CANONICAL only when:
- implementation merged;
- acceptance criteria proven;
- required platform evidence green;
- post-merge state verified;
- no blocking review finding remains;
- governance state reflects the result.
