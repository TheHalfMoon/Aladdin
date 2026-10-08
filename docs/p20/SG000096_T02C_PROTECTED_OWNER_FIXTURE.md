# SG-000096 T02-C: Protected Expected-Owner Native Test Candidate

Status: candidate-only, test-gated. This does not complete T02-C, T02, or production Full User shell support. Tracking #271 and #264.

## Prerequisite and source

Canonical `main` was `3feddd6ccfbad9959bd24b2f527edf680ea10ba0` when this branch was created; post-merge CI `37840376795` completed 9/9 SUCCESS before any successor implementation. All added code is native Deskal-owned test logic and a `qdral-policy` development dependency; no donor source is imported.

## Qualification-only behavior

The Windows test opens a live native owner token and records its SID, logon AuthenticationId LUID, session, elevation, and process-creation generation. It creates an isolated disposable HMAC-authenticated Full User lease store **outside the user's real Deskal authority paths**. The expected SID/LUID used for native child verification comes from the MAC-verified lease, not by copying the child token. The Full User lease is checked against the separately observed local owner and exact policy, epoch, device and session, before any process is created.

Only after acceptance does the test create a disposable ordinary-user child suspended, attach it to a kill-on-close Job, and compare its native SID, LUID, session, elevation and creation generation before resume. Denials after spawn must terminate the suspended child and observe Job quiescence. The native test handles are never exposed through a production API; this is *not* shell authorization.

## Adversarial cases

- 0: exact lease, live owner and child, bounded completion
- 1: valid JSON with deliberately tampered HMAC in the protected store
- 2: revoked lease
- 3: expired lease
- 4: stale authority epoch
- 5: wrong native user SID
- 6: wrong native logon LUID
- 7: wrong device binding
- 8: changed owner creation generation before child creation
- 9: changed child creation generation after suspended launch, requiring verified termination

## Honest limits

The disposable test key is synthetic and in-memory; it does not represent a real user-presence grant. The owner process creation generation is captured within this test, **not yet loaded and authenticated from the live installed Deskal runtime session record**. The expected Windows session/elevation values are read from the verified parent owner token, and child creation time comes from its native handle. This is not proof of an independently sourced protected *runtime owner record*, nor approval consumption. Live production dispatch remains disabled.

Remaining gates: validate persisted runtime session identity and nonce against protected state, real local STRONG Full User lease, independently consumed one-shot SOFT approval with replay denial, native token-query failure injection, canonical executable/cwd handle and digest checks, bounded output/time/descendant cleanup, T03 interactive sessions, T04 local-only MCP exposure and T05 adversarial closeout. Job Objects are lifecycle supervision, not filesystem or network isolation.

## Native local qualification

- `cargo test -p qdral-provider-process --lib sg000096 -- --test-threads=1`: 5/5 PASS.
- `cargo test -p qdral-provider-process -- --test-threads=1`: 37/37 PASS (29 unit, 2 protected state, 4 PowerShell, 2 workspace).
- `cargo clippy -p qdral-provider-process --all-targets -- -D warnings`: PASS.
- All tests ran using disposable native child processes and isolated synthetic local authority state; missing local `TMP` / `ComSpec` were supplied only in test process environment, not installed as system settings.

All further changes require native Windows regression, cargo fmt, Clippy, exact-head 9/9 CI, genuine Jev and Alibaba OCR (manual excluded-file review), signed+DCO forward history, normal merge and post-main CI.
