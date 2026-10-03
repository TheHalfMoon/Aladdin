# SG-000058 QUNTAL-P15 Adversarial and Chaos Exit Note

Status: IMPLEMENTATION FOR QUNTAL-P15
SpecGrain: SG-000058
Base: `bd3db96427ab2f8eb24dd97b3931440f49fcf876`
Date: 2026-10-02

This note maps every P15 adversarial condition to executable evidence, or
records it as UNVERIFIED with the exact owner procedure. Happy paths alone
do not close P15.

## Evidence sources

- `apps/quntal-relay/src/adversarial.test.ts` (new): two tenants on one real
  loopback relay with durable state, two real device uplinks, the
  authoritative MCP builder, and recording kernels.
- `crates/quntald/src/sg000039_main.rs` `sg000058_real_lease_boundary_on_this_workstation`
  (new): the real quntald envelope path, workstation probe, lease store, trust
  store, policy, and filesystem dispatch.
- Existing SG-000054 through SG-000057 suites (`oauth-authorization.test.ts`,
  `device-uplink.test.ts`, `relay.test.ts`, `device-channel.test.ts`,
  `self-host.test.ts`, quntal-policy `remote_session`, quntald `remote_lease`
  and `sg000055_envelope_tests`).

## Matrix

| Condition | Result | Evidence |
| --- | --- | --- |
| Wrong tenant / wrong principal | Fails closed | adversarial: foreign session 404 both ways; kernels see only their own principal |
| Wrong device | Fails closed | uplink: route device mismatch dropped; envelope device binding; adversarial: per-tenant device ids |
| Wrong connection | Fails closed | relay: session bound to route (404); uplink: connection never switches route; quntald: pinned connection |
| Wrong provider / client | Fails closed | adversarial: cross-client refresh `client_mismatch`; edge requires route client to equal token client |
| Wrong audience / resource / issuer | Fails closed | relay and OAuth tests: 401 `invalid_token`; authorization request `wrong_resource` redirect |
| Wrong or unknown scope | Fails closed | edge 403 `insufficient_scope` / `TOOL_SURFACE_DENIED`; uplink scope ceiling; quntald leased ceiling |
| Expired / revoked token | Fails closed | relay tests; self-host revoke at edge; adversarial device revoke |
| Stolen refresh token | Fails closed | self-host: offline device `device_proof_missing` immediately; rotation replay revokes family; code replay revokes issued tokens |
| Device offline | Fails closed | relay: 503 `DEVICE_OFFLINE`, nothing queued; refresh refused |
| Replay / duplicate | Never re-executes | uplink: nonce, duplicate correlation, repeated sequence; hub: single-use response nonces, one-shot correlation |
| Reorder | Fails closed | uplink: gaps `RELAY_SEQUENCE_INVALID`; relay keeps sequences contiguous |
| Delayed request | Fails closed | uplink: expiry and 60 s queue life `REMOTE_QUEUE_EXPIRED` |
| Late response | Dropped, honest outcome | hub: undelivered timeout becomes a cancel never executed; delivered timeout `TRANSPORT_UNAVAILABLE`; late push ignored |
| Reconnect | Never extends | quntald real boundary: different connection `REMOTE_SESSION_INACTIVE`; uplink bounded backoff |
| Relay restart | No carryover | adversarial: sessions 404 after restart, no replayed calls, device reconnects, lease still decides |
| Queue-after-revoke | Never executes | uplink: revocation rechecked before dispatch; quntald real boundary: revoked lease denied |
| Queue-after-lease-expiry | Never executes | quntald real boundary: expired lease denied; uplink queue expiry |
| Cross-route read exfiltration | Bounded | relay route binding plus quntald lease workspace set; real boundary: workspace outside the lease denied |
| Cross-route mutation | Denied | same bindings; write/execute keep their own approvals; management tools do not exist (adversarial) |
| Workspace revoke | Invalidates lease | quntald real boundary: trust revoke denies an active lease |
| Profile narrowing | Invalidates lease | quntal-policy: client profile id or revision change inactive |
| Policy revision change | Invalidates lease | quntald real boundary: workspace-set drift denies; quntal-policy: policy revision drift |
| Device revoke | Invalidates | relay: revoked device loses channel and tokens; quntal-policy: device epoch change; uplink: route revocation |
| Lock | Fails closed (model) | quntal-policy and quntald: `Locked` and `Unknown` inactive; real probe returns `Unlocked` only for an active unlocked session. Real lock transition: UNVERIFIED (procedure below) |
| Logoff | Fails closed (model) | lease binds session id and logon time; a different logon is inactive. Real logoff transition: UNVERIFIED (procedure below) |
| Suspend | Fails closed (model) | wall-clock expiry plus revalidation on every dispatch. Real suspend/resume: UNVERIFIED (procedure below) |
| Quota exhaustion | Fails closed | adversarial: per-route 429 before the device; self-host: registration and daily budget 429; no paid overflow |
| Oversized payload | Fails closed | relay 413; uplink `REMOTE_RATE_LIMITED`; result cap |
| Malformed frame / protocol | Fails closed | uplink: exact fields, kinds, version, JSON-RPC shape; relay: batches, client responses, content type, protocol version |
| Malicious metadata | No effect | adversarial: `_meta` and extra arguments cannot change principal, device, or scopes; unknown arguments stripped before the kernel |
| Privacy / log leakage | Clean | adversarial: relay and uplink log events are class strings only; no token, code, identifier, path, or payload in logs; relay state holds no tokens, payloads, or device private keys; CI container logs checked |

## P15 exit criteria

- Remote authenticated MCP reaches one paired device without inbound PC
  ports: self-host and adversarial tests; the device only makes outbound
  POSTs (source scans prove no listener).
- Every dispatch requires remote auth plus an active local lease: edge token
  verification plus the real quntald boundary test (no lease, expiry, revoke,
  reconnect all `REMOTE_SESSION_INACTIVE`).
- The relay cannot mint local authority, and neither can a provider: lease
  creation needs local STRONG presence; remote contexts can never reach
  `remote.*`, trust, approval, or lifecycle capabilities; the kernel remote
  context comes only from the local pairing record.
- Compromised-relay read exposure is bounded by lease scope and time: the
  lease workspace set, scope ceiling, profile, and 15-minute hard maximum,
  plus the documented residual risk.
- Self-host mode works independently: the self-host end-to-end test and the
  CI container job.
- Quota behavior fails closed: quota tests and the hard-ceiling validator.

## Real Windows evidence

`sg000058_real_lease_boundary_on_this_workstation` was run on a real
Windows 11 Home (build 26200) interactive session. The probe reported
`Unlocked`, the exact active lease allowed a real `fs.read`, and reconnect,
an out-of-lease workspace, expiry, revoke, trust revoke, and policy drift
were denied. On hosts without an unlocked interactive session (for example
CI runners), the same test proves lease creation and every remote dispatch
fail closed and prints that the allowed path is UNVERIFIED there.

## UNVERIFIED owner procedures (not fabricated)

Exercising real lock, logoff, or suspend on the owner's machine is
disruptive and was not done automatically. To verify on a real PC:

1. `quntal remote enable --relay <origin>`, link a client, `quntal remote pair`,
   `quntal remote connect`, then `quntal remote allow --connection <rc-...>`.
2. Call `tools/list` and `fs_read` from the remote client: expect success.
3. Lock (Win+L), call again from the remote client: expect
   `REMOTE_SESSION_INACTIVE`. Unlock and call: expect success only while the
   lease is unexpired.
4. Sign out and back in, call again: expect `REMOTE_SESSION_INACTIVE` (new
   logon); a new `quntal remote allow` is required.
5. Sleep the PC past the lease expiry, wake, and call: expect
   `REMOTE_SESSION_INACTIVE`.
