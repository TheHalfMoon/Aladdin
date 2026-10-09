# Security Threat Model (Proposed)

Status: PROPOSED PLANNING ONLY. Extends, and does not replace, `docs/security/THREAT_MODEL.md`, `docs/security/UNIVERSAL_CONNECTIVITY_THREAT_MODEL.md`, and `docs/security/SG-000093_FULL_CONTROL_AUTHORITY_MODEL.md`. Security issue #278 stays open; this document proposes a repair and a test matrix only.

## 1. Assets

| Asset | Where it lives |
|---|---|
| Ability to act as the user on a device (input, files, processes, browser sessions) | `qdrald` and its hosts on each device |
| Approval and lease state (SOFT/STRONG approvals, FullControlLease, remote leases) | Per-user protected state on each device |
| Device private keys | Device protected store (`device/device_key.json` today) |
| Screen pixels, accessibility text, page content, clipboard, terminal output | Transient on device; optionally sent to a model under egress policy |
| Task receipts and audit | Device; optional encrypted cloud copy |
| Cloud account, billing, conversation memory | Aladdin AI cloud (server-side only) |

## 2. Trust boundaries

```text
[Untrusted content: web pages, documents, screens, emails]
        | (prompt injection source)
[Models: Hala One, Reliance, Claude, GPT, others]  -- propose only
        |
[Clients: Aladdin app UI, MCP clients, Aladdin AI cloud orchestrator]
        |  authenticated channel, typed proposals
[qdrald on device]  <-- the only authority; local human for SOFT/STRONG
        |
[Hosts: Computer Host, Browser Engine, Shell Host]  -- execute only
        |
[Windows: user session, UIA, Win32, ConPTY, Job Objects]
```

## 3. Threats and required controls

| # | Threat | Control (existing = E, proposed = P) | Test |
|---|---|---|---|
| T1 | Prompt injection from screen or page content steers actions | E: typed targets, per-action SOFT for mutations, protected surfaces. P: egress-labeled observations; content from observations is never placed in system instructions; Reliance may flag but never authorizes | Hostile-page and hostile-window corpus; zero unapproved side effects |
| T2 | Model approves its own action | E: approvals only from local broker dialogs; Aladdin windows excluded from automation | Attempt UIA/input on approval dialog: denied |
| T3 | Hosted Aladdin AI gains OS authority | P: cloud sends `ComputerActionProposal` only; device re-validates against local policy; cloud cannot mint leases, approvals, or targets | Forged proposals with stale/unknown targets fail closed |
| T4 | Lateral movement across enrolled devices | P: per-device grants; no privilege inheritance from the controlling device; no credential replication; each device's human consents locally | Controller compromise test: other devices still require their own grants |
| T5 | Replay of remote commands | E: relay frames with sequence and expiry (SG-000052). P: operation ids deduplicated per device with durable seen-set; idempotency keys on mutations | Duplicate delivery and reorder fuzzing |
| T6 | Approval replay or loss across restart | #278: reproduced; see Section 5 | Section 5 matrix |
| T7 | Silent remote desktop control | P: always-visible indicator while any remote session or lease is active (tray badge plus screen-edge border, as MeshAgent does); local kill switch hotkey | Native UI test that indicator renders before first remote action |
| T8 | Secrets in screenshots, logs, receipts | E: password field redaction, clipboard secret denial. P: Sentrdel-derived detectors on OCR/UIA text before egress; secret handles for typed secrets | Seeded-secret corpus: zero leaks to logs or egress |
| T9 | Browser session theft (cookies, tokens) | E: isolated profile, no cookie export. P: existing-browser mode via a paired extension bound to device key; no cookie export ever | Attempt cookie read via extension RPC: denied |
| T10 | Donor or dependency supply chain | E: pinned donors, SBOM, reproducible build, provenance attestation. P: no runtime downloads; no remote feature flags | CI: lockfile diff equals declared dependency diff |
| T11 | Local malware as same user | Out of scope to fully prevent (same-user code can drive UI). P: STRONG for trust changes; HMAC-protected authority store (E, SG-000095); document residual risk | Documented residual risk |
| T12 | Ledger tampering by a same-user process | SHA-256 chain is not a MAC (noted in #278). P: per-install HMAC key in DPAPI-protected store; ACL owner-only; documented limits | Tamper test: modified record fails closed |
| T13 | Job Object assumed to be a sandbox | P: never claim filesystem or network isolation from Job Objects; Full User shell is the user's authority by definition | Review checklist item |
| T14 | Voice spoofing (replayed or synthetic speech triggers actions) | P: voice never authorizes; voice produces intents that still require on-device approval for consequential actions | Replay a recorded command: no side effect without approval |
| T15 | Cost abuse of hosted AI | P: per-tenant budgets, hard ceilings, rate limits, abuse detection | Budget exhaustion test returns typed error, no overrun |
| T16 | Cross-tenant data leakage in cloud | P: tenant-scoped encryption keys, request-scoped KV caches, no cross-tenant prompt caching | Isolation tests with canaries |
| T17 | Protected OS surfaces automated remotely | E: protected-surface exclusion. P: secure desktop, UAC, credential UI, Windows Hello prompts are never automatable regardless of origin | Native negative tests |

## 4. Classification: security findings versus feature gaps

| Item | Class |
|---|---|
| #278 approval ledger chain inconsistency | SECURITY (integrity defect; replay reproduced at ledger level) |
| Ledger durability uses `flush()` without `sync_all()`, no file lock | SECURITY (durability and concurrency) |
| Browser engine absent (template snapshot) | FEATURE GAP |
| Remote full control absent | FEATURE GAP (intentionally gated) |
| No visible remote-session indicator yet | SECURITY REQUIREMENT for any remote actuation |
| README tool count drift | DOCUMENTATION |

## 5. Issue #278 — reproduction, impact, and repair proposal

### 5.1 Reproduction (VERIFIED)

Executed on `main@61e664b3c39380a76aede29aa9c2d7fcbc449b08` with toolchain 1.97.1 by adding two temporary tests to `crates/qdral-approval/src/lib.rs` in a scratch clone (not committed):

```rust
#[test]
fn repro_278_consumed_marker_lost_after_restart_allows_replay() {
    let path = temp_path("repro278-a");
    let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
    let prompt_a = strong_prompt_with("digest-a278", 5_000);
    let token_a = broker.request_token(&prompt_a).expect("token a");
    let prompt_b = strong_prompt_with("digest-b278", 5_000);
    let _token_b = broker.request_token(&prompt_b).expect("token b");
    let expected_a = ConsumeExpectation::strong(prompt_a.digest.clone(), "default", "sg-000019-v1");
    broker.consume(&token_a, &expected_a, 5_100).expect("first consume of A");
    assert!(broker.consume(&token_a, &expected_a, 5_101).is_err(), "in-process replay denied");
    drop(broker);
    let mut reloaded = ApprovalLedger::load_or_create(path.clone());
    let replay = reloaded.consume(&token_a, &expected_a, 5_200);
    let _ = std::fs::remove_file(&path);
    assert!(replay.is_err(), "REPLAY: approval A was consumed again after restart");
}

#[test]
fn repro_278_record_after_consumed_marker_unreadable_after_restart() {
    let path = temp_path("repro278-c");
    let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
    let prompt_a = strong_prompt_with("digest-a278c", 5_000);
    let token_a = broker.request_token(&prompt_a).expect("token a");
    let expected_a = ConsumeExpectation::strong(prompt_a.digest.clone(), "default", "sg-000019-v1");
    broker.consume(&token_a, &expected_a, 5_100).expect("consume A");
    let prompt_c = strong_prompt_with("digest-c278", 5_000);
    let token_c = broker.request_token(&prompt_c).expect("token c");
    drop(broker);
    let reloaded = ApprovalLedger::load_or_create(path.clone());
    let present = reloaded.records.contains_key(&token_c.record_id);
    let _ = std::fs::remove_file(&path);
    assert!(present, "record C written after a consumed marker is unreadable after restart");
}
```

Command: `cargo test -p qdral-approval repro_278 -- --nocapture --test-threads=1`

Observed output (abridged):

```text
REPRO278 replay-after-restart result: Ok("CONSUMED_AGAIN")
... panicked ... REPLAY: approval A was consumed again after restart
REPRO278 ledger lines on disk: 3; record C readable after restart: false
... panicked ... record C written after a consumed marker is unreadable after restart
test result: FAILED. 0 passed; 2 failed
```

Both #278 hypotheses are confirmed at the ledger level.

### 5.2 Root cause (source)

- `consume` sets `self.tip = updated.checksum` (the issuance record's checksum) before `append_consumed_marker` (`lib.rs` near line 1192).
- `append_consumed_marker` writes `prev_checksum = self.tip` but never advances `self.tip` to the marker checksum (`lib.rs` near line 1223).
- `load_or_create` stops at the first chain mismatch (`break`), silently discarding everything after it, including consumed markers and later approvals.
- Persistence uses `flush()` only, not `sync_all()`; there is no single-writer lock.

### 5.3 Impact assessment

- Exploitability today: LOW. In shipped flows the token is created and consumed inside one in-process dispatch (for example `crates/qdrald/src/fs_mutation.rs:96-106`) and never crosses a process boundary, so a restarted daemon has no surviving token to replay. The replay window is also bounded by `APPROVAL_TTL_MS` = 5 minutes.
- Integrity impact today: CERTAIN. After a restart, approval history after the first broken link is silently dropped, and consumed approvals appear unconsumed.
- Future impact: HIGH. Any design that holds an approval across IPC, sessions, remote delivery, or restart (SG-000096 interactive sessions, remote approvals, approve-ahead UI) would turn the defect into real replay.

### 5.4 Repair design

1. Separate record kinds: immutable `Issued` records and independent `Consumed { record_id, nonce }` transition records. Never rewrite issuance content.
2. Every append computes `prev_checksum` from the actual last persisted record and, only after a durable write, sets `self.tip` to the new record's checksum.
3. Durability: write, `flush`, then `sync_all` (Windows `FlushFileBuffers`) before returning success from `consume`. If durability fails, deny dispatch and mark the operation `not_started`.
4. Single writer: exclusive lock file (`LockFileEx`) held for the broker lifetime; a second broker on the same ledger fails closed.
5. Load: on any chain break, enter fail-closed mode (deny all consumes, report `approval_unavailable`, preserve the file for forensics), never "accept the prefix".
6. Authentication: replace plain SHA-256 chaining with HMAC-SHA-256 keyed by a per-install secret in the existing protected HMAC authority store (SG-000095), so a same-user tamper cannot rebuild a valid chain without the key. Document that malware running as the user with access to the key remains out of scope.
7. Migration: read legacy ledgers; if broken, rename to `.legacy-broken` and start a new genesis with a recorded migration event; no legacy approval is honored after migration (approvals live 5 minutes, so no user impact).

### 5.5 Regression matrix (must pass before #278 can close)

| Case | Expected |
|---|---|
| Issue A, issue B, consume A, restart, consume A | Denied (already consumed) |
| Issue A, consume A, issue C, restart | Full chain readable; C present |
| Consume A and B out of order, restart twice | Zero replays; full history |
| Truncated last line or corrupted marker | Fail closed; nothing resurrected |
| Two brokers on one ledger | Second broker refuses to start |
| Write or sync failure during consume | Consume returns error; operation not dispatched |
| Tampered record without key | Load fails closed |
| Emergency revoke, restart, old token | Denied |
| Expired token after restart | Denied |
| 10,000 interleaved issue/consume cycles with random restarts (property test) | Invariants hold |

Gate: the repair needs the full governance stack (signed+DCO, exact-head 9/9 CI, genuine TypeSafe Jev, Alibaba OCR with manual exclusions, zero unresolved threads, normal merge, post-merge CI) and an independent reviewer. This document does not close #278 and does not change SG-000096 authorization order.

## 6. Privacy defaults

- No telemetry by default; opt-in diagnostics only, never containing pixels, page bodies, clipboard, file contents, or terminal output.
- Screenshots leave the device only under an explicit egress grant naming the destination class (`EXTERNAL_MODEL` = Hala One or a BYO model) and are minimized (window crop, redaction).
- Cloud retention default: task transcripts 30 days, screenshots 0 days (processed in memory) unless the user opts in; training on user data only by explicit opt-in.
