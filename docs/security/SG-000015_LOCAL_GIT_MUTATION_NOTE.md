# SG-000015 — Approved local Git mutation security note

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P06
Grain: SG-000015

## Purpose

SG-000015 introduces the first bounded local Git mutation surface for repositories already inside a trusted Quntal workspace. It is deliberately narrower than generic Git command execution.

The only newly reachable mutation capabilities are:

- `git.branch.create` — create and switch to one new validated branch at an exact expected HEAD;
- `git.stage` — stage an explicit literal repository-relative file set;
- `git.unstage` — unstage an explicit literal repository-relative file set;
- `git.commit` — create exactly one non-amending unsigned commit from an exact approved staged state.

## Approval and stale-state binding

Every mutation requires fresh local Quntal approval.

The approval digest binds:

- workspace id;
- policy revision;
- capability and operation;
- repository target and canonical repository root;
- expected HEAD and current branch;
- material worktree/index/staged-state digest;
- exact literal path set where applicable;
- branch name for branch creation;
- SHA-256 digest of the commit message for commit.

After approval, Quntal reconstructs the relevant repository state and fails with `TARGET_STALE` if it differs from the approved state. The Git provider performs a final state check immediately before applying the mutation.

## Repository and path boundary

The Git provider first resolves the repository through the trusted workspace provider boundary. Mutation inputs do not accept arbitrary Git argv.

Path-bearing mutation rejects:

- absolute paths;
- parent traversal;
- NUL/newline/control input;
- Git pathspec magic;
- `.git` control paths;
- `.gitmodules` mutation;
- duplicates and oversized path sets;
- directories and symlinks;
- paths with configured Git filter drivers for staging.

Literal pathspecs are used for mutation commands.

## Git subprocess hardening

Mutation subprocesses use fixed Git subcommands only. The provider disables or suppresses hidden execution and interactivity, including:

- repository hooks through an empty temporary hooks directory and `--no-verify` where applicable;
- commit signing;
- editors and sequence editors;
- credential helpers and credential prompting;
- interactive credential UI;
- fsmonitor;
- pagers;
- external diff/textconv paths where relevant;
- Git network protocol transport.

Commit requires repository-local `user.name` and `user.email`; global identity inheritance is not accepted as the authority source.

Unsupported in-progress repository states such as merge, rebase, cherry-pick, revert, bisect, or sequencer state fail closed before mutation.

## Postconditions

Each operation returns typed evidence and verifies its material result:

- branch creation verifies the resulting current branch and unchanged expected HEAD;
- stage/unstage return index evidence for only the approved path set;
- commit verifies HEAD advanced exactly once, the new commit parent equals the approved expected HEAD, and the current branch did not change unexpectedly.

## Explicitly excluded authority

SG-000015 does not authorize:

- `git.fetch` or `git.push`;
- any Git network transport;
- force push;
- branch deletion;
- switching to arbitrary existing branches;
- reset, rebase, merge, cherry-pick, revert, stash, clean, tag, or arbitrary reflog mutation;
- arbitrary Git subcommands or caller-provided Git argv;
- commit amend or commit signing;
- submodule mutation;
- credential-helper authority;
- raw shell, `cmd.exe`, or public PowerShell;
- additional public `process.spawn` executable targets;
- browser/UI automation, elevation, approval bypass, or persistent approval reuse.

Windows public `process.spawn` remains restricted to the SG-000010-qualified `%SystemRoot%\\System32\\whoami.exe` target. Public `powershell.run` remains absent.

## Qualification requirements

The final implementation head must pass:

- Rust / Windows;
- Rust / Ubuntu;
- Node / Windows;
- Node / Ubuntu;
- Governance;
- genuine TypeSafe Jev exact-diff review;
- Alibaba Open Code Review exact-range qualification;
- manual exact-diff security review;
- zero unresolved blocking review threads.

Cubic, Qodo, CodeRabbit, and similar systems are not qualification evidence.

This document records intended and implemented controls only. It does not by itself claim SG-000015 `PROVEN` or `CLOSED`; those states require exact-head, merge, post-merge, and canonical closeout evidence.
