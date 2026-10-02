# SG-000055 Outbound-Only Device Uplink and Remote-Session Lease Note

Status: IMPLEMENTATION FOR COTRA-P15
SpecGrain: SG-000055
Base: `21d85fc94d3fd6fa2875bc7a599c13e20f4bdf5b`
Date: 2026-10-02
Companions:
- `docs/security/RELAY_PROTOCOL_CONTRACT.md` (SG-000052, frozen)
- `docs/security/SG-000053_DEVICE_IDENTITY_NOTE.md`
- `docs/security/SG-000054_OAUTH_AUTHORIZATION_NOTE.md`
- `docs/security/REMOTE_SESSION_AUTHORIZATION.md`
- `apps/cotra-mcp/src/device_uplink.ts`, `apps/cotra-mcp/src/uplink_state.ts`
- `apps/cotra-mcp/src/transports/relay_device.ts`
- `crates/cotra-policy/src/remote_session.rs`
- `crates/cotrad/src/remote_lease.rs`, `crates/cotrad/src/workstation.rs`
- `crates/cotra-lifecycle/src/remote.rs`

## 1. Purpose

SG-000055 implements the device side of remote access: an outbound-only,
authenticated device channel that carries frozen SG-000052 frames, and the
local remote-session lease that `cotrad` checks on every remote dispatch.
It also fixes, at its root, a pre-existing lifecycle tree-ACL verification
race. No public `/mcp` edge (SG-000056), no relay server or deployment
(SG-000057), no new MCP tool, no schema change, and no approval change for
any existing capability is introduced.

## 2. Outbound-only channel

- The device opens no listener and no inbound port. Every network operation
  is an outbound `POST` from the device to one configured relay origin:
  `/device/v1/challenge`, `/device/v1/session`, `/device/v1/poll`
  (long-poll), and `/device/v1/push`.
- The relay origin must be `https`, or `http` on an exact loopback IP
  literal for a self-hosted relay on the same machine. Origin only: no path,
  query, fragment, or credentials.
- Redirects are refused (`redirect: "error"`), credentials are omitted, every
  request has a timeout, and every relay response body is read with a hard
  byte cap (`16 * (1 MiB + 8 KiB) + 64 KiB`).
- Source tests prove the uplink sources import no `node:net`, `node:http`,
  `node:https`, `node:dgram`, `node:tls`, or `node:child_process`, call no
  `createServer` or `.listen(`, register no tools, use no WebSocket, and
  contain none of the SG-000052 denied capability tokens.

## 3. Channel authentication

The relay issues an SG-000053 device challenge. The device accepts it only
when it binds the exact local device identifier and epoch, has a lifetime of
at most 120 seconds, and has not expired; it then signs it with the local
Ed25519 key and receives an opaque 256-bit channel token. Every inbound frame
must carry that token (constant-time comparison); a relay that cannot bind
the challenge to this device never obtains a channel. Responses are
authenticated with the token current at push time.

## 4. Frame validation (before any dispatch)

Inbound frames must have exactly the 13 frozen envelope fields plus
`payload`, protocol `cotra-relay/1`, and kind `mcp_request`, `cancel`, or
`heartbeat`. In order, the device checks: types and identifier shapes;
channel token; `routeDeviceId` equals the local device; `remoteConnectionId`
is a locally paired connection; payload byte length equals `payloadLength`
and is at most 1 MiB; lifetime at most 120 seconds, not expired, not created
more than 30 seconds in the future; replay nonce shape and single use;
session-context scopes within the paired ceiling; JSON-RPC shape; the
device-verifiable authorization envelope (principal, connection, device,
recomputed request digest, device epoch, session context, route revocation);
connection binding (a short-lived connection never switches routes or
scopes); sequence (first frame is 1, then exactly the next number; lower is
replay, higher is a gap); duplicate correlation; bounds; and, for
`tools/call`, the SG-000054 default-deny scope to tool ceiling.

Frames that cannot be attributed to an authenticated, paired route are
dropped without any reply. Attributable rejections produce `mcp_error`
frames with frozen failure codes: `MCP_PROTOCOL_UNSUPPORTED`,
`REMOTE_AUTH_INVALID`, `ROUTE_MISMATCH`, `REMOTE_SCOPE_DENIED`,
`TOOL_SURFACE_DENIED`, `RELAY_REPLAY_DETECTED`, `RELAY_SEQUENCE_INVALID`,
`REMOTE_QUEUE_EXPIRED`, `REMOTE_RATE_LIMITED`, `DEVICE_REVOKED`,
`TRANSPORT_UNAVAILABLE`. Final rejections (scope, duplicate) consume the
sequence; transient backpressure (queue full, rate) does not, so the relay
retries the same sequence with a fresh nonce.

## 5. Bounds, backpressure, and no durable queue

- at most 16 queued requests per device, each dropped with
  `REMOTE_QUEUE_EXPIRED` if not started within 60 seconds or before frame
  expiry;
- at most 4 concurrent requests per connection, 2 connections per device,
  and 60 accepted requests per minute per device;
- 4096 remembered nonces and correlations per bounded window;
- route revocation is rechecked immediately before dispatch
  (queue-after-revoke fails with `DEVICE_REVOKED`);
- `cancel` removes queued work and suppresses in-flight results; a running
  kernel request is not described as terminated;
- responses wait at most the 60-second reconnect grace and are never
  persisted; nothing is durably queued anywhere.

## 6. Dispatch

An accepted request becomes one MCP message for the authoritative
`buildCotraServer` catalog over an in-process frame transport (one server
per short-lived connection, closed when idle). The kernel for that server is
constructed with a server-supplied `RemoteDispatchContext` (principal, both
connection identifiers, device and epoch, provider kind, client profile and
revision, `core` profile, pinned scopes). Tool arguments cannot reach it.

## 7. Remote-session lease (`cotrad`)

- `cotrad` accepts `RemoteRequestEnvelope` lines: the unchanged internal
  request plus an optional camelCase `remote` context with
  `deny_unknown_fields`. Local transports never send it.
- Every remote-context request is checked against the protected lease store
  (`%LOCALAPPDATA%\Cotra\remote_leases.json`, override
  `COTRA_REMOTE_LEASE_PATH`, now listed in the protected-state overrides)
  before policy and dispatch. Any mismatch fails with the frozen
  `REMOTE_SESSION_INACTIVE`; local authority management (`remote.*`,
  `workspace.trust.*`, `trust.*`, `approval.*`, `lifecycle.*`) fails with
  `CAPABILITY_DENIED` regardless of any lease.
- A lease binds principal, stable connection, short-lived connection
  (pinned on first use, so reconnect never extends it), device and epoch,
  provider kind, client profile and revision, `core` profile, OAuth scope
  ceiling, workspaces with their trust revisions, read mode, policy
  revision, workspace-set digest, the Windows session and logon, creation,
  and hard expiry of at most 15 minutes.
- Creation and widening (`remote.lease.create`) require STRONG
  platform-mediated presence bound to a digest of every lease field; the
  lease starts when presence is confirmed and is re-validated after
  approval. Only trusted, configured workspaces on an unlocked interactive
  workstation can be leased.
- Revocation (`remote.lease.revoke`) is authority-reducing and needs no
  approval. Emergency revoke also revokes every lease.
- The lease is invalidated by expiry, revoke, emergency revoke, device epoch
  change (device revoke or key rotation), workspace trust change or revoke,
  client profile change, policy revision change, workspace-set change,
  workstation lock, a different logon (logoff), a disconnected or unknown
  session, and a different short-lived connection. Reconnect, OAuth refresh,
  relay restart, and provider retry never extend it.
- Read modes: `session` runs read tools without their own approval class;
  `per_request` requires a fresh local SOFT approval for every remote read;
  `disabled` denies remote reads. Writes and executes always keep their own
  approval class; a lease is never an action approval.
- A tampered, corrupt, or oversized lease store yields no leases.

## 8. Workstation state

`cotrad` queries `WTSQuerySessionInformationW(WTSSessionInfoEx)` for its own
session. Only an active session whose flags report unlocked is
`Unlocked(session, logon time)`; locked, disconnected, unknown, or failed
queries fail closed. Other platforms always report `Unknown`, so remote
dispatch never runs there. Exact lock, logoff, and suspend transitions on
real Windows are additionally qualified in the P15 adversarial exit
(SG-000058); CI does not fabricate workstation evidence.

## 9. Lifecycle commands

`cotra remote allow|revoke|status` talk to `cotrad` as the existing trust
commands do. `cotra remote connect` starts the uplink entrypoint with the
sanitized environment plus the uplink configuration, device key, and
revocation list paths under the Cotra state directory. The lifecycle CLI
reads only public device fields; it never reads, prints, or logs the private
key. Missing pairing state fails closed with `PAIRING_REQUIRED`.

## 10. Lifecycle tree-ACL race fix

`verify_tree_acl` listed the install tree and then read each entry's DACL. A
sibling atomic write (`write_bytes_atomic`) could rename its temporary file
away in between, so `GetNamedSecurityInfoW` returned not-found and
verification failed spuriously (release qualification runs `36897477978`
and `36992754562`). Entries that no longer exist are now skipped; every
entry that exists is still checked, a missing root still fails, and a
foreign ACE on an existing entry still fails. Deterministic Windows tests
reproduce the vanished-entry case and prove enforcement is unchanged.

## 11. Residual limitations

- A compromised relay that holds a valid route can still invoke tools within
  an active lease's exact workspaces, profile, scopes, and remaining time;
  this is the documented bounded residual risk.
- A remote call already dispatched when the workstation locks completes; the
  lock denies every later call.
- Pairing enrollment (`cotra remote enable` and `pair`), the relay server,
  and the public edge are delivered by SG-000056 and SG-000057.
