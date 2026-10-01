# Release Security Review (COTRA-P13)

Status: program-level security review for the first Cotra release, recorded under SG-000046. It summarizes how every change was independently reviewed, what the reviews found, and what risk remains. It makes no claim that was not observed.

## Method

Every grain from SG-000008A onward went through the same exact-head qualification before merge, recorded per grain in `.specgrain/canonical-evidence.json` and `docs/canonical/CURRENT.md`. SG-000001 through SG-000007 predate the automated review gate; the ledger records their historical security and semantic review and makes no retroactive automated-review claim. The gate:

1. **Exact-head CI** on Windows and Ubuntu (Rust format, Clippy with warnings denied, all tests; Node typecheck and tests; governance validation; from SG-000044 the packaged-release qualification on a fresh `windows-latest` runner; from SG-000045 the supply-chain audit).
2. **Genuine TypeSafe Jev** semantic review of every hunk of the exact diff (pinned review revision `31f89602797fb7bea007f8a480bf368bf564954e`), requiring complete hunk coverage and zero blocking findings.
3. **Alibaba Open Code Review** v1.12.9 exact-range delegation (checksum-pinned binary). Its delegated language rules were applied in manual review, and every file it excludes (Markdown, lockfiles) was reviewed by hand.
4. **Exact-diff manual security review** against the threat model and the grain's authority delta.
5. **Zero unresolved review threads** at merge, and successful post-merge CI before a grain could close.

Third-party review bots that commented on pull requests are not qualification evidence. Their comments were still evaluated on their merits: on the P12 and P13 pull requests #118 through #129 they raised 78 threads, the large majority of which led to fixes (redaction coverage, crash recovery of interrupted updates, cross-process lifecycle locking, handle-inheritance and child-environment hygiene, version strings validated before use as paths, lockfile-pinned packaging, workflow hardening); the rest were answered with reasons.

## Findings during release hardening

Security-relevant defects found and fixed before release (each with regression tests):

- **Protected state reachable from broad workspaces** (SG-000041): a trusted folder containing `%LOCALAPPDATA%\Cotra` exposed trust, approval, and audit state to workspace tools. Such workspaces are now refused.
- **ACL reset could follow a junction** (SG-000042): the install tree is now proven link-free before the owner-only DACL is applied.
- **Detached supervisor inherited callers' handles** (SG-000043): the supervisor is launched with an explicit handle list.
- **Secret redaction gaps** (SG-000043): spaced, quoted, option-style, multi-token, and later-in-token credential assignments are now redacted, and diagnostics are redacted before any truncation.
- **Unvalidated record versions used as path components** (SG-000043): records are validated before use.
- **Update activation window and crash consistency** (SG-000044): `start` refuses while an update is pending, a lifecycle lock serializes commands, and interrupted updates recover through the validated marker.
- **Test-only features reaching shipped binaries** (SG-000045): building the test fixture in the same cargo invocation unified a dev-dependency feature (`cotra-approval/test-support`) into the release build. No production code references that module, so no approval path could reach it, but the shipped binaries must not depend on how the fixture is built. The independent-rebuild check exposed it, and no release had been published; shipped binaries are now built with `--bins` only and the reproducibility check runs on every change.
- **Supply-chain generators reading outside their inputs** (SG-000045): the SBOM generator now refuses manifest paths outside the release and linked or non-regular payload entries, and notice collection never follows links or declared license files outside their package.

## Independent qualification of the release artifact

- The release-qualification job installs the **packaged and archived** release on a fresh Windows runner, drives MCP through the installed app (exact canonical tool set), runs `doctor`, updates while running, refuses a downgrade, rolls back, recovers automatically from a release whose CLI cannot verify itself, stops with verified termination, and uninstalls with retention and purge (rollback and recovery drill).
- The same job rebuilds and repackages from a separate checkout and fails unless the shipped binaries and the final archive are byte-identical.
- Dependencies are audited on every change (`cargo audit`, `npm audit`); see `docs/research/DEPENDENCY_LICENSE_REVIEW.md`.

## Final repository review

Recorded under SG-000046 against the repository at its activation base:

- **Markers**: no `TODO`, `FIXME`, `XXX`, `HACK`, `todo!()`, or `unimplemented!()` in code. The single `unreachable!()` (`crates/cotra-provider-process/src/lib.rs`, private-execution qualification) sits in a `match` guarded by the `matches!` check immediately above it.
- **Dead code**: the 24 snapshot files `crates/cotrad/src/sg000015_main.rs` through `sg000038_main.rs` were never compiled (`cotrad` builds only `src/sg000039_main.rs`, which includes `src/main.rs`) and are removed; the compiled `cotrad` test set is unchanged.
- **Test reliability**: two timing- and naming-dependent test races found in CI were repaired without weakening assertions (SG-000013 output limits in PR #125; `cotra-tunnel` shared temporary directories in PR #132).
- **Test-only code**: `cotra-approval`'s `test-support` module is referenced only from `#[cfg(test)]` modules.
- **Workflows and scripts**: `ci.yml`, `review-gates.yml`, and the draft-only `release.yml`, two composite actions, and five release scripts, all in use; no temporary qualification workflow is committed.
- **Versions**: the Cargo workspace, root `package.json`, and `apps/cotra-mcp/package.json` are all `0.1.0`; Rust 1.97.1 is pinned by `rust-toolchain.toml` and Node.js 24.19.0 by the workflows.
- **Pull requests**: the owner-authored PR #115 (donor-pattern plan) and PR #130 (agent context policy) are open and not part of the canonical state; they were not acted on. Merged feature branches remain on the remote; deleting them is left to the owner.

## Residual risks

- Same-user malicious code can read the user's own files and stop Cotra; Windows ACLs, AppContainer children, and STRONG presence reduce but cannot remove this.
- Real ChatGPT connectivity through the OpenAI tunnel, interactive SOFT/STRONG approval prompts, and SmartScreen behavior require a human and the owner's OpenAI credentials and are not exercised in CI.
- Release binaries are unsigned; integrity relies on manifest hashes, `SHA256SUMS.txt`, and the GitHub build-provenance attestation.
- `git_fetch`/`git_push` destination policies are not configurable through the lifecycle CLI, so those tools fail closed in an installed Cotra; `process_spawn` is restricted to `whoami.exe`.

## Conclusion

No known blocking security finding remains open. No capability beyond the closed, documented set was added during P11 through P13; the MCP surface is pinned by tests to exactly 20 tools.
