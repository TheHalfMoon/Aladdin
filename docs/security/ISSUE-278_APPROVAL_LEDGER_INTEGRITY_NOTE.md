# Issue #278 — Approval ledger integrity, concurrency and recovery

Status: IMPLEMENTATION CANDIDATE
Issue: #278
Amends: SG-000018 approval history

## Defect

The v0.1.0 broker reset the in-memory chain tip to an approval's issuance
checksum after appending that approval's consumption marker. The next record
therefore chained to the issuance instead of to the marker, and the loader
silently kept only the valid prefix. After a restart, consumption markers
beyond the break were lost and a consumed approval could verify again.

## Authority delta

None expanded. File, process, network, browser, UI/input, secret and
privilege authority are unchanged. Approval authority is narrowed: a ledger
that does not verify end to end grants nothing.

## Ledger semantics

- Issuance, revoke and consumption are separate append-only events. A
  consumption marker must equal its issuance record except for `consumed`
  and its own chain fields.
- Every persisted event must verify against the running tip. A torn final
  line, an unparsable or unverifiable record, a duplicate id or nonce, an
  out-of-sequence revoke epoch, or a second consumption poisons the broker.
  An invalid suffix never yields a usable valid-prefix history.
- An append is acknowledged only after `write_all` and `sync_all` succeed (and
  on Unix after the new file's directory entry is synced). Any append error
  poisons the broker without retrying, because durability is then unknown.
- A poisoned broker refuses every grant and consumption, and is checked
  before any person is prompted, so nobody approves an action that cannot be
  recorded.

## Concurrency

The MCP edge, the CLI and `qdral full-control` each create brokers in
separate processes over one ledger. Each transaction (issue, revoke,
consume) takes an exclusive OS lock on the sidecar
`approval-history.jsonl.lock`, verifies and applies every byte other
processes appended since its last view, decides, appends durably, and
releases the lock. The lock is never held while a person decides, and the OS
releases it when a holder dies.

- A token consumed in one process is denied in every other process
  (four-process race test: exactly one consumption succeeds).
- Before trusting the file again, a broker re-reads its last verified line.
  A ledger that shrinks, disappears or is replaced (even by one of the same
  length) under a live broker poisons it.
- Lock contention past 5 seconds, and I/O errors such as a sharing
  violation from backup or antivirus software, report a transient error.
  They are never treated as corruption.
- The broker records the revoke epoch when a prompt opens. If an emergency
  revoke (from any process) happens while the person decides, an approval
  is recorded as unavailable and refused.

## Recovery and upgrade from v0.1.0

v0.1.0 chained each consumption marker to its own issuance record and then
reset the tip to that record, so any v0.1.0 history that consumed an
approval and contains another record after that issuance does not verify.
In practice nearly every v0.1.0 user who approved anything is affected
once. After upgrade such a broker fails closed and its error names the
v0.1.0 upgrade as the expected cause. `qdral doctor` reports the failure
and `qdral approvals recover`:

1. takes the writer lock and refuses a ledger that is missing, verifies, or
   is busy;
2. renames the ledger byte for byte to
   `approval-history.jsonl.quarantine-<ms>-<sha256 prefix>` (never deleted
   or rewritten);
3. starts a new chain with a non-authorizing `Unavailable` record naming the
   quarantined file and its SHA-256.

No earlier approval carries over, so every operation needs a fresh approval.
Running brokers adopt the new ledger on their next transaction without a
restart, but only when the new chain differs from the one they knew,
starts with the recovery record, and verifies completely.

A torn final line (a crash mid-append) also fails closed and requires
recovery, although such a line was never acknowledged. This is deliberate:
a torn tail cannot be distinguished from tampering without a trust anchor.
Because approvals expire after five minutes, recovery loses no standing
authority; the quarantined file keeps the audit history. v0.1.0 histories
whose chain is valid (no record after a consumption) keep loading unchanged.

## Residual risk: tail rollback

The SHA-256 chain is self-contained. A fresh broker cannot tell a complete
ledger from one whose valid tail (for example a consumption marker) was
removed at a record boundary, or that was deleted outright. A live broker
detects both. Exploiting a rollback needs write access to the protected Qdral
state directory plus a still-unexpired approval (five-minute window) held by
a caller; that same-user, write-capable attacker is outside the current
threat model. Full anti-rollback protection requires an external trust anchor
(for example a monotonic counter or a sealed tip held outside the ledger
directory) and is **not** claimed here.

Nonces come from a non-cryptographic hash of process id, clock, counter and
digest. Approval tokens stay inside the broker process, so this is not an
exploitable gap today, but nonces must be made unpredictable before tokens
ever cross a process or network boundary.

## Evidence

Unit and process tests in `crates/qdral-approval/src/lib.rs`:
`issue_a_and_b_consume_a_then_restart_replay_denied`,
`two_brokers_on_one_ledger_see_each_others_transitions`,
`concurrent_processes_consume_one_token_exactly_once`,
`crash_mid_append_releases_the_lock_and_fails_closed_until_recovery`,
`writer_lock_contention_is_transient_and_never_corruption`,
`truncated_or_deleted_ledger_under_a_live_broker_fails_closed`,
`legacy_v010_history_without_a_successor_still_verifies`,
`legacy_v010_broken_chain_fails_closed_until_recovered_with_preserved_evidence`,
`recovery_refuses_missing_verified_or_busy_ledgers`,
`same_length_replacement_under_a_live_broker_fails_closed`,
`emergency_revoke_during_an_open_prompt_refuses_the_approval`,
`live_broker_adopts_a_recovered_ledger_without_restart`,
`poisoned_broker_never_adopts_its_own_unchanged_chain`,
`legacy_v010_interleaved_consumption_fails_closed_with_upgrade_guidance`,
`sharing_violation_is_transient_and_never_corruption` (Windows). Lifecycle:
`approval_history_check_verifies_the_chain_not_just_json`,
`approvals_recover_quarantines_only_an_unverifiable_ledger`,
`purge_removes_data_only_when_requested`, and packaged Windows release
qualification (uninstall purges the sidecar lock).
