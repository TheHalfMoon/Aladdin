# Multi-device: one controller, two Windows targets first

Status: ADOPTED planning direction (founder decision, 2026-10-09; Sol revision governs where it differs from the Opus baseline). Grants no authority: every capability still needs its own grain, review and qualification. Current execution state: `EXECUTION_FRONTIER.md`. Evidence baseline of this revision: PR #283 at `87bc9d6a69b6c5b5dc415ee6e30ee0fcb60d8985`, PR #282 at `10540885f3c52bd0c0d2f00cc23e3cafe067fe36`.

## Decision

Extend existing device identity, pairing, OAuth, outbound relay and target leases. Build no new transport for the first controller managing targets B and C. Test three distinct machines/VMs; a single remote target is insufficient. Device identity primitives and relay contracts exist, but controller directory, transfer workflow and visible attended remote operation are not a completed journey.

MeshCentral/MeshAgent contribute design references for outbound channels, pinning, device inventory and indicators. Do not import their privileged service/remote KVM stack. Desktop Commander remote behavior is not a substitute for Aladdin's per-target authority. Founder security vocabularies may clarify consequence and egress classes; they must not introduce a peer policy daemon.

## Smallest authorized flow

1. B and C each initiate pairing locally with required presence. Controller A stores public identity and display metadata. Default grant is observation-only; B's grant does not extend to C.
2. User explicitly selects B and grants bounded report-folder read plus approved transfer egress to A. C remains independently selectable with no inherited authority.
3. A requests an immutable file identity and permitted metadata from B. User authorizes exact source, target and size/recipient. A's local kernel separately authorizes create-only destination inside its approved folder.
4. A bounded transfer streams encrypted chunks to staging with size/quota limits, content hash, transfer id and expiry. Destination validates full hash and commits atomically without overwrite. An existing conflicting path requires a fresh decision.
5. A receives B's provenance receipt and independently verifies destination bytes/hash. Later task handoff transfers description/artifact references, never approvals, leases or credentials.

Remote folder read and create-only transfer need an explicitly adopted capability contract; do not route them around today's restrictions or assume full remote desktop control is necessary. Arbitrary remote shell/desktop input remains gated by SG105 or authorized successors and local time-boxed leases.

## Protocol and protection

Existing qdral-relay/1 admits only mcp_request, mcp_response, mcp_error, cancel and heartbeat. Its bounded reconnect/expiry behavior must remain intact; no new frame kinds by editorial plan change. A reviewed versioned application payload/transfer service can fit existing envelopes if size/binding/semantics permit; otherwise request an explicitly versioned protocol amendment. Do not encode bulk data into unbounded MCP messages.

TLS to the relay is not device-to-device end-to-end encryption. If the adopted threat model requires relay-blind content, use an audited maintained protocol/library with endpoint-key binding, rotation, replay and nonce management; no homegrown crypto. That requirement may precede confidential transfer, but does not require LAN, QUIC or hole punching. Do not claim today's relay already has that confidentiality.

Bind tenant, principal, controller, target device, enrollment epoch, session, task/transfer, expiry and authorized scopes. Validate on the target. DPAPI wrapping can reduce copied-key use across accounts/machines; it does not defeat malware already running as the same user. Credential/cookie/approval export is prohibited. Show a target-side remote indicator before work, with local stop independent of controller/cloud availability.

## Reliability and proof

| Test | Required result |
|---|---|
| A paired with B but not C | C rejects access; B grants never authorize C |
| Controller tampering/malicious relay | Forged target/epoch/principal denied; confidentiality tested if E2E is adopted; drops produce bounded failures |
| B and C online, then one offline | Bounded directory shows actual status; no indefinitely queued mutation; unrelated target remains usable |
| Transfer break before commit | No final destination artifact; staging expires, quota reclaimed |
| Break after dispatch/commit before receipt | outcome_unknown; reconcile destination hash/existence, do not blindly create twice |
| Repeated chunks/requests | Deduplicate transfer reads/chunks by validated identity; mutations remain one-shot |
| Source changes mid-transfer | Identity/hash mismatch fails; fresh observation and consent |
| Revocation/restart | Admission closes, queued work expires, leases end; no old grant resurrection |
| Target stop/controller crash | Input released and owned work terminated where provable; unknown side effects visible |
| Success | Independent controller-side byte/hash check and target-scoped receipts |

Acceptance: one controller can select each of two authorized targets, read only its granted folder, transfer a verified permitted copy, deny a third/unpaired target, and cancel under network drop. Use native non-admin Windows on all targets. No multi-device E2E has been run in this review.

## Path to dozens/hundreds

Keep the same app: paginated/searchable device directory, bounded subscriptions, independent target queues with per-controller/per-tenant quotas, one mutating task per target by default, bounded read concurrency, directory epochs and revocation propagation. Start with an explicitly placed task list; add DAG placement only when real workflows need dependencies. Measure 2/10/100-target simulated control-plane load plus actual representative Windows agents, reconnect storms, fan-out cancellation and slow/offline targets. Simulations are not 100 real-device proof.

LAN direct, mDNS discovery, QUIC, hole punching, weighted schedulers, unattended policy and enterprise groups are deferred behind demonstrated need, explicit authority review and measured bottlenecks. The product can scale logically without bundling a fleet platform into every client; availability, operating costs and team administration still require qualification.
