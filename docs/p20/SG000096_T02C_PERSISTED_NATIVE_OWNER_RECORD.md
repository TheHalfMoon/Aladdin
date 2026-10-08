# SG-000096 T02-C: Persisted runtime owner and MAC-protected lease fixture

Status: Windows **test-only** qualification candidate (parent #271 / #264). This does not close T02-C, T02, or SG-000096, and does not introduce any production ShellProcess capability.

## Provenance and prerequisite

Built from canonical main `29f1cb88e3fd7631b616379d99a0c07e3b394f82` after its post-merge CI run `37854168442` reported **9/9 SUCCESS**. All additions are Deskal-owned; no donor code was copied or imported. Existing Cargo workspace packages `qdral-lifecycle` and `serde_json` are added solely as **development dependencies** of the process provider; no new third-party package or release runtime dependency is introduced.

## What the native fixture verifies

Before spawning any ordinary-user child, the fixture writes a disposable typed `RuntimeSessionRecord` into a dedicated test temporary directory and reads it back from disk. Its deterministic synthetic session nonce encodes the observed live owner PID and real process creation generation. The separately persisted record must match the test's independently retained expected session identifier, schema, PID and generation; `qdral-lifecycle::runtime::open_verified` must reopen the real owner process with matching PID, creation time and image. Windows runtime-session validation confirms the native process is still live with the recorded creation generation.

The same test root holds an independently MAC-authenticated synthetic Full User lease. The verified lease's SID, logon AuthenticationId LUID, device identity, policy revision, authority epoch, expiry, and runtime-session ID must match the reopened owner record and actual native owner token before any child is created. Only the positive path creates a suspended disposable ordinary-user child, assigns and verifies its Job Object membership, compares the actual child SID/LUID/session/elevation/creation generation, and resumes the fixture with bounded completion and Job quiescence. Any failure after child creation must terminate the child and verify cleanup.

## Adversarial qualification cases

Existing cases 0-9 remain unchanged in intent: valid binding, HMAC tamper, revocation, expiry, stale epoch, wrong SID/logon LUID/device, owner creation drift, and child creation drift. New **persisted runtime record** cases deny **before child creation**:

- 10: altered persisted owner process creation generation;
- 11: altered persisted executable image path;
- 12: syntactically valid but changed persisted session nonce;
- 13: altered owner PID (PID-reuse / wrong-owner defense);
- 14: missing persisted runtime record;
- 15: unexpected runtime-record schema.

## Explicit security limits

The fixture uses a **synthetic test key**, a synthetic session nonce and a disposable record; this is not the installed protected Deskal runtime's actual user-approved owner record, nor evidence of genuine STRONG presence or SOFT one-shot approval. It validates an *isolated model of the data flow* and native checks. The process handle is opened by the existing Windows lifecycle verifier in the test's own user context; production broker / IPC, user-signaled approvals, exact executable/cwd handle binding, process quotas, interactive T03, local-only T04 and full T05 adversarial exit remain UNPROVEN.

A Windows Job Object bounds lifecycle and child cleanup; it does not sandbox filesystem/network power of a Full User process. No real full-user command, privileged operation, remote surface, new public MCP tool, or new release/install payload is introduced.

## Qualification evidence

Local native Windows qualification observed on the authorized Abdulaziz device:

- SG96 native tests: 6/6 PASS, including persisted-record adversarial cases 10-15 within the new Full User fixture.
- Full provider suite: 38/38 PASS (30 unit, 2 protected-state, 4 PowerShell, 2 workspace).
- Release blocker sweep: 5/5 PASS.
- Graft wiring: 234 source files, 5704 nodes, 9502 edges; graph check PASS; the semantic deep-meaning tier was not run.
- Formatting, Clippy all-targets -D warnings, signature+DCO, manual excluded-file inspection, real Jev/Alibaba delegated reviews, 9/9 exact-head CI, normal merge and 9/9 post-main are independently required; do not mark them successful until checked.
