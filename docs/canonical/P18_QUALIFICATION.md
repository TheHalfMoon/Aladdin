# Deskal P18 Adversarial Qualification Matrix

Product: Deskal. QDRAL-P18 is a retained compatibility and program
identifier within the Deskal product; it is not the current product
brand. This matrix traces every P18 capability from requirement to
implementation to tests to review to evidence. Qualification adds no
authority: every row is PROVEN by tests and review, or NOT_EXPOSED by
runtime and tool discovery. No row is both exposed and unverified.

Specifier: SG-000085, the sole active QDRAL-P18 grain. Base:
`b6fd6a0125d92d3d46eeea692756b2f9e2679757`.

## Capability rows

| Capability | Requirement | Implementation | Tests | Status |
|---|---|---|---|---|
| Browser host supervision | SG-000074 | `crates/qdral-browser-host/src/supervise.rs`, `host.rs` | `tests/sg000074_host.rs`, ceiling locks (`JOB_ACTIVE_PROCESS_LIMIT`, `LAUNCH_GRACE`, `SHUTDOWN_TIMEOUT`, `HANDSHAKE_TIMEOUT`) in `tests/sg000085_qualification.rs` | PROVEN |
| Browser networking | SG-000075 | `src/navigation.rs` | 9 inline mediation tests; hop, byte, and widening bounds plus bypass-transport denial in `sg000085_qualification.rs` | PROVEN |
| DOM and accessibility observation | SG-000076 | `src/observation.rs` | 7 inline tests (512 nodes, depth 16, 64 KiB, concurrency 4); redaction, freshness, and error-leak tests in `sg000085_qualification.rs` | PROVEN |
| Structured browser actuation | SG-000077 | `src/actuation.rs` | 5 inline tests; dispatch classification, digest binding, and no-retry proof in `sg000085_qualification.rs` | PROVEN |
| Bounded transfers | SG-000078 | `src/transfers.rs` | 5 inline tests (8 MiB, traversal, sniffing); size, binding, and type-denial tests in `sg000085_qualification.rs` | PROVEN |
| Browser tool exposure | SG-000079 | `src/exposure.rs` | 4 inline tests (12 qualified, 0 live); denial-registry lock plus personal-profile and mapping denial in `sg000085_qualification.rs` | PROVEN |
| Exact-window capture | SG-000080 | `src/capture.rs` | 4 inline tests; geometry, frame, rate, byte, and restart-invalidation tests in `sg000085_qualification.rs` | PROVEN |
| Structured UIA | SG-000081 | `src/uia.rs` | 5 inline tests; one-shot approval, value and scroll bounds, PID reuse, and drift tests in `sg000085_qualification.rs` | PROVEN |
| Coordinate fallback | SG-000082 | `src/coordinates.rs` | 4 inline tests; scroll, text, TTL, digest-binding, and stale-capture tests in `sg000085_qualification.rs` | PROVEN |
| Proposal IR and model adapters | SG-000083 | `src/proposal.rs` | 6 inline tests; byte bounds, secret-free errors, and backend-denial tests in `sg000085_qualification.rs` | PROVEN |
| Remote lease isolation | SG-000084 | `src/remote_leases.rs` | 4 inline tests; lifetime ceiling and cross-scope denial in `sg000085_qualification.rs` | PROVEN |
| Failure semantics | SG-000085 A2 | `src/error.rs`, per-module denial paths | 42-entry hostile corpus, outcome classification, and no-retry tests in `sg000085_qualification.rs` | PROVEN |
| Privacy | SG-000085 A2 | Redaction in `observation.rs`, static diagnostic discipline | Redaction, non-echo, and bounded-diagnostic tests in `sg000085_qualification.rs` | PROVEN |
| Resource ceilings | SG-000085 A2 | `MAX_*` constants per module | Value locks for every published ceiling plus boundary enforcement in `sg000085_qualification.rs` | PROVEN |
| Denied authority | SG-000085 A2 | Denial registries and absent surfaces | Closed-shape locks plus static source tripwires in `sg000085_qualification.rs` | PROVEN |

## Explicitly denied and unreachable (NOT_EXPOSED)

Arbitrary JavaScript, caller-facing browser evaluate, generic
CDP and DevTools, unrestricted shell, PowerShell, and cmd, generic
script execution, generic sockets and tunnels, personal browser
profiles, raw model-to-input authority, whole-screen and background
streams, credential and security-dialog automation, remote approval,
model self-approval, silent elevation, silent coordinate fallback,
arbitrary transfer paths, mutation auto-retry after dispatch, and
runtime dynamic donor-code fetching. Proven by the hostile corpus
(runtime denial), the closed-shape locks, and the static source
tripwires in `tests/sg000085_qualification.rs`.

## Known observations (not blockers)

- The absolute capture frame-count bound (30) is shadowed by the
  per-minute rate bound (20) and the 60-second lease lifetime, so it
  stands as unreachable defense in depth. Every live frame path still
  enforces a bound, proven by test.
- Dispatch outcomes use the supported vocabulary only: pre-dispatch
  validation failure (approval unconsumed, nothing started),
  dispatched receipt, and postcondition verification. There is no
  `outcome_unknown` variant to test; completion is never claimed
  without the exact-plus-one generation advance.
- Error diagnostics echo bounded, already-validated material only
  (flag names, validated origins, parser reasons); secret and payload
  material never enters errors, proven by the marker corpus.

## Evidence

Exact-head CI, genuine TypeSafe Jev, Alibaba Open Code Review,
manual review of excluded files, and zero unresolved review threads
are recorded on the implementation PR. Language rule: this document
is English only.
