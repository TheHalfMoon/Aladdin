# Aladdin Device Fabric (Proposed)

Status: PROPOSED PLANNING ONLY. Remote actuation remains gated (SG-000105 and successors). This design reuses the existing device identity, pairing, OAuth, relay, and remote-lease contracts (SG-000052 to SG-000057, SG-000084) and does not import MeshCentral, MeshAgent, or RustDesk code.

## 1. Goals and non-goals

Goals: one user (or an authorized operator) controls many enrolled computers from one controlling device; tasks can run on several devices; files can move between devices under grants; progress is visible everywhere; any device can stop everything locally.

Non-goals: unattended fleet management at enterprise scale, SYSTEM-level agents, remote access to protected OS screens, automatic discovery-and-join, any publicly reachable desktop server.

## 2. What exists today (VERIFIED)

| Element | Location | Notes |
|---|---|---|
| Device identity: Ed25519 key pair, `dev-<32 hex>` id, private key never leaves device | `apps/qdral-mcp/src/device_identity.ts` | Key store `device/device_key.json` under protected state |
| One-time pairing code (256-bit, 300 s, 5 attempts), challenge-response (120 s), key rotation, epochs, revocation | same | |
| Relay protocol: sequence numbers, single-use replay nonces, duplicate handling, 60 s reconnect grace, max 16 queued frames, no durable offline queue | `docs/security/RELAY_PROTOCOL_CONTRACT.md` Sections 6 and 7 | |
| Outbound-only device uplink and remote-session lease | `apps/qdral-mcp/src/device_uplink.ts`, `crates/qdrald/src/remote_lease.rs` | |
| OAuth 2.1 for remote principals | `apps/qdral-mcp/src/oauth_authorization.ts` | |
| Self-hostable relay with quotas | `apps/qdral-relay` | |

Gap: the relay connects remote MCP clients to one device. There is no controller role, no device directory, no multi-device task model, no device-to-device file transfer, and no visible remote-session indicator.

## 3. Device identity and ownership

- Enrollment is always initiated on the target device by its local user (STRONG presence), producing a pairing offer. Nothing joins automatically.
- Owner model: `Account (user or organization) -> Device groups -> Devices`. Each device record: id, public key, display name, group, OS, capability manifest (which hosts and profiles are installed), enrollment epoch, attended/unattended policy.
- Key rotation: device-initiated, signed by the old key, recorded in the device epoch; controllers reject frames signed with retired keys.
- Lost or stolen device: account-side revoke (immediate relay denial) plus local revoke on next contact. Proposed hardening (not verified as current behavior): wrap the device private key with DPAPI (user scope) so a copied key file is not usable on another machine or account.
- Controller devices are just enrolled devices with a `controller` capability; controlling another device never grants the controller's own local authority to anyone.

## 4. Connectivity

Preference order:

1. LAN direct: mDNS-advertised presence only after both devices are enrolled in the same account; connection is mutually authenticated with device keys (Noise IK or TLS 1.3 with raw public keys). No unauthenticated listener; the listener accepts only enrolled peers and is off by default.
2. Direct over the internet via UDP hole punching through a rendezvous (QUIC). RustDesk demonstrates the pattern; implement independently.
3. Relay fallback: the existing Aladdin relay, end-to-end encrypted between devices so the relay sees only routing metadata. Self-hosted relay supported (already containerized in CI).

Trust assumptions: the relay and rendezvous are untrusted for confidentiality and integrity (E2E encryption and device signatures), trusted only for availability. A malicious relay can drop or delay, never forge or read.

Offline and network change: frames carry operation ids and expiries; reconnect within 60 s resumes; after that, in-flight remote operations report `outcome_unknown` or `not_started` precisely as defined in the relay contract. No durable offline queue for mutations (keep the frozen SG-000052 rule).

## 5. Orchestration model

```text
Controller (app or Aladdin AI cloud orchestrator)
   |
   |  TaskGraph { nodes: DeviceTask, edges: depends_on }
   v
Per-device queue (on the target device, owned by qdrald)
   |
   +-- admission: device grant, attended/unattended policy, lease, budget
   +-- execution: local method hierarchy, local approvals
   +-- receipt: signed by the target device key
```

- DeviceTask: `{task_id, device_id, capability_scope, method_ceiling, inputs (by content hash), expected outputs, deadline, idempotency_key}`.
- Placement: explicit by default (the user names the device); optional rule-based placement by capability manifest, online state, and current load; never by the model alone.
- Dependencies: DAG edges carry artifact hashes; a downstream node starts only when the upstream receipt verifies (signature plus postcondition).
- Fairness: per-device FIFO with per-controller weights; one mutating task per device by default (configurable), unlimited read-only tasks within bounds.
- Retries: device-aware and only for `not_started` or idempotent reads. `outcome_unknown` requires reconciliation by observation before any new side effect.
- Failure isolation: a device failure fails only its subgraph; siblings continue unless marked dependent.
- Progress: devices publish signed status events (`queued`, `running`, `awaiting_approval`, `completed`, `cancelled`, `outcome_unknown`) to the controller and to the dashboard.
- Handoff ("start here, continue there"): handoff transfers a task description and referenced artifacts, never authority. The receiving device re-observes and re-authorizes locally.
- File transfer: content-addressed chunks, E2E encrypted, written only into a destination path permitted by the receiving device's grant, create-only by default, hash-verified.

## 6. Security rules

| Rule | Mechanism |
|---|---|
| Explicit enrollment and local consent | Pairing initiated on target with STRONG presence |
| Device-specific capability grants | Each device stores its own grants for each controller principal; default grant for a new controller is observation-only |
| No privilege inheritance or lateral movement | A controller's grants on device A have no effect on device B; device B never accepts commands relayed by device A unless A is an enrolled controller with B's own grant |
| No credential replication | Secrets are device-local handles; no export path |
| No unrestricted remote shell by default | Remote shell requires a RemoteFullControlLease issued locally (P20 Section 4.5) |
| User-visible indicator | Tray badge and screen-edge border while any remote session or remote lease is active (pattern from MeshAgent's notify bar and monitor border) |
| Immediate local kill switch | Global hotkey and tray action revoke all remote leases and cancel queued remote work |
| Attended versus unattended | Separate policies; unattended requires an explicit, time-boxed local grant and is never the default; protected OS screens are never automatable |
| Replay-safe commands | Operation id plus device-scoped durable seen-set (after the #278-style ledger repair pattern), signature, expiry |
| Exact binding | Every frame binds account, controller principal, device id, device epoch, session id, task id |
| Audit | Target-device-signed receipts; controller stores copies |

## 7. Reliability requirements

| Scenario | Required behavior |
|---|---|
| Network drop mid-action | `outcome_unknown` if dispatched; no automatic re-dispatch |
| Duplicate request | Recorded result returned once for idempotent operations; otherwise denied |
| Target device restart | Queued items expire; leases end; receipts survive; user must re-grant |
| Cancel | Propagates to hosts; reports verified termination only when the host proves it |
| Partial completion | Receipt lists completed steps and the first unknown step |
| Stranded input | Computer Host releases held keys and buttons on cancel, crash, or lease end (unwind already required by SG-000095) |
| Orphan processes | Job Object kill-on-close for owned process trees |

## 8. Comparison

| System | Reuse | Do not inherit |
|---|---|---|
| Aladdin relay/device identity | Extend directly | — |
| MeshCentral/MeshAgent (Apache-2.0) | Patterns: outbound control channel, server pinning, notify bar, device groups | SYSTEM service, Duktape runtime, browser-based remote KVM |
| RustDesk (AGPL-3.0) | Patterns only: rendezvous plus relay, hole punching | Any code |
| UFO3 Galaxy (MIT) | DAG task model, heartbeat and reconnection ideas | Unauthenticated AIP transport |
| Kernux remote-device concepts | Contract vocabulary | Separate daemon |
| Desktop Commander remote | — | Hosted Supabase relay, persisted session without device ACL |

## 9. Acceptance tests (minimum)

1. Enrollment requires local STRONG presence; a pairing code alone grants observation only.
2. Compromised controller cannot act on a device without that device's grant.
3. Relay operator cannot read or forge frames (E2E test with a malicious relay).
4. Duplicate, reordered, and replayed frames are rejected or deduplicated per contract.
5. Indicator visible before the first remote action; kill switch revokes within 1 second.
6. Network drop during a click yields `outcome_unknown` and no re-dispatch.
7. Two-device DAG with a file artifact completes only with verified receipts.
8. Lost-device revoke blocks the device at the relay within 5 seconds.
