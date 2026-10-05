# Deskal P18 Exit Matrix

Product: Deskal. QDRAL-P18 is a retained compatibility and program
identifier within the Deskal product; it is not the current product
brand. This is the exit record for SG-000086: every capability is
exactly one of PROVEN, NOT_EXPOSED, or UNVERIFIED. No row is both
exposed and unverified, so P18 closes.

Specifier: SG-000086, the sole active QDRAL-P18 grain. Base:
`7d17fc5e69f0072e0ccb125390194d965cb811f6`.

Companion qualification detail:
`docs/canonical/P18_QUALIFICATION.md` (SG-000085 traceability).

## Exit rows

| Row | Status | Evidence |
|---|---|---|
| Browser host supervision | PROVEN | SG-000074 tests, `tests/sg000074_host.rs`, ceiling locks in `tests/sg000085_qualification.rs` |
| Browser networking | PROVEN | SG-000075 tests plus hop, byte, widening, and bypass-denial tests |
| Browser identities | PROVEN | Page, document, and node generation binding in SG-000076, SG-000077, SG-000083 tests plus freshness and digest tests |
| DOM and accessibility observation | PROVEN | SG-000076 tests plus redaction and bound tests |
| Structured browser actuation | PROVEN | SG-000077 tests plus dispatch classification and no-retry tests |
| Bounded transfers | PROVEN | SG-000078 tests plus size, binding, and type-denial tests |
| Exact-window capture | PROVEN | SG-000080 tests plus geometry, frame, rate, byte, and restart-invalidation tests |
| Structured UIA | PROVEN | SG-000081 tests plus approval, bound, reuse, and drift tests |
| Coordinate fallback | PROVEN | SG-000082 tests plus scroll, text, TTL, and stale-capture tests |
| Model adapters | PROVEN | SG-000083 tests plus byte-bound, non-echo, and backend-denial tests |
| Remote leases | PROVEN | SG-000084 tests plus lifetime and cross-scope tests |
| Failure semantics | PROVEN | 42-entry hostile corpus, outcome classification, no-retry proof |
| Privacy | PROVEN | Redaction, non-echo marker corpus, bounded diagnostics |
| Resource ceilings | PROVEN | Value locks for every published ceiling plus boundary enforcement |
| Denied authority | NOT_EXPOSED | Closed registries, hostile corpus, static tripwires; arbitrary JavaScript, shells, personal profiles, streams, queues, remote approval, and elevation remain unreachable |
| Packaging | PROVEN | Version parity test (npm and Cargo agree on 0.2.0), pinned manifests, lockfiles |
| Dependency audit | PROVEN | Lockfile pins, no wildcard or branch deps, `Supply chain audit` green |
| License audit | PROVEN | Apache-2.0 workspace license lock plus third-party notices generation in release |
| Third-party notices | PROVEN | `scripts/third-party-notices.mjs` wired into release, attached per release notes |
| SBOM | PROVEN | CycloneDX generation in CI and release (`scripts/generate-sbom.mjs`) |
| Provenance | PROVEN | Provenance records plus build-provenance attestation in release |
| Reproducibility | PROVEN | Reproducibility check action in CI |
| Windows qualification | PROVEN | `Rust / windows-latest` and `Release qualification / windows-latest` green on every merged P18 PR; policy suites execute on real Windows runners |
| Tool and profile parity | PROVEN | Exact registry locks and unknown-profile denial in `tests/sg000086_exit.rs`; no implicit widening |
| Exact-head CI | PROVEN | Per-PR CI on the exact qualified head, recorded in canonical evidence |
| TypeSafe Jev | PROVEN | Genuine exact-diff review on every qualified head with zero blocking findings |
| Alibaba OCR | PROVEN | Exact-range delegation on every qualified range with excluded files manually reviewed |
| Manual review | PROVEN | Full-file review of every OCR-excluded file, recorded per PR |
| Post-merge verification | PROVEN | Green post-merge CI on every canonical merge SHA, recorded in canonical evidence |

## Donor position at exit

Pinned input under study: `bytedance/UI-TARS-desktop@2ff41a9e515828c5bd5b276e493d73aa0bdf4a3a`
per `docs/canonical/QDRAL_COMPUTER_USE_DONOR_MATRIX.md`. At P18
exit no donor code is imported anywhere in the shipped tree (proven
by the canonical SG-000066 tripwire in
`apps/qdral-mcp/src/p16-exit.test.ts`, which forbids donor runtime
markers in every crates `.rs` file and both app manifests on every
CI run): the action parser shape is Deskal-native syntax-only
normalization, the operator abstraction is concept-only, raw
model-to-input is reference-only, and remote-operator authority,
donor agent loops, arbitrary browser scripting, unrestricted
commands, and personal profiles are rejected and remain unreachable.

## Remote posture at exit

Remote execution remains disabled. Leases bind and deny per
SG-000084; no background streams, offline queues, delayed execution,
or cross-scope reuse exist. Nothing in this exit grain enables
remote authority.

## Language rule

This document is English only.
