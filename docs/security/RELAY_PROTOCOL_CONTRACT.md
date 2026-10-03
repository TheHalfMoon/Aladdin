# Quntal Relay Protocol and Remote Threat Contract

Status: FROZEN CONTRACT FOR QUNTAL-P15
SpecGrain: SG-000052 (QUNTAL-P15, design-first)
Base: `a7002bfd56657fe2aa4f3710d10942031d66a258`
Date: 2026-10-01
Companions:
- `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md` (sections 6, 9, 10, 18, 20)
- `docs/security/UNIVERSAL_CONNECTIVITY_THREAT_MODEL.md`
- `docs/security/REMOTE_PRINCIPAL_AUTH_MODEL.md`
- `docs/security/REMOTE_SESSION_AUTHORIZATION.md`
- `docs/security/CLIENT_CONNECTION_PROFILE_MODEL.md`

## 1. Purpose and gate

This document freezes the P15 relay protocol and remote threat contract.
No relay, device uplink, OAuth, or public edge implementation code may merge
until this contract is canonical. Successor grains (SG-000053 through
SG-000057) implement against this text; any deviation requires a new
explicitly authorized SpecGrain.

The relay carries only Quntal-defined authenticated MCP traffic. It is never
a generic proxy. Anything not defined here is denied.

## 2. Roles

- `provider edge`: the public Streamable HTTP MCP endpoint. Authenticates
  the remote principal (OAuth 2.1, SG-000054) and admits only MCP frames.
- `relay broker`: routes authenticated frames between exactly one remote
  connection and exactly one paired device channel. Holds no local
  authority and creates none.
- `device uplink`: the outbound-only channel from the user's PC to the
  relay (SG-000055). No inbound home or router port exists.
- `local edge`: the on-device receiver that verifies the device-verifiable
  authorization envelope, checks the active local remote-session lease, and
  dispatches to the authoritative Quntal tool registry through `quntald`.

Compromise of any relay-side role must not mint local authority.

## 3. Routing

Every accepted remote request resolves to exactly:

```text
authenticated principal -> paired device -> active device channel -> local MCP session
```

Rules:

- route lookup is keyed by the authenticated principal plus the explicitly
  selected paired device; there is no fallback to another online device;
- cross-tenant or cross-device routing ambiguity is a hard failure with
  typed `ROUTE_MISMATCH` before any local MCP dispatch;
- reconnect binds to the same authenticated device key and route epoch;
  route epoch changes on revoke or re-pair, and there is no route migration
  without an explicit new pairing;
- results are correlated by tenant plus device plus connection plus request;
  the response path never uses a request ID alone globally, so a result
  from device A is never delivered to another authenticated client.

## 4. Frames

### 4.1 Protocol version

Every frame carries `protocolVersion` with the single frozen value
`quntal-relay/1`. Unknown versions fail closed with
`MCP_PROTOCOL_UNSUPPORTED`. Version negotiation, when added by a successor
grain, must refuse to downgrade silently.

### 4.2 Request frame fields

A request frame carries exactly these envelope fields plus the MCP payload:

- `protocolVersion`: frozen `quntal-relay/1`;
- `routeDeviceId`: opaque tenant-scoped device route identifier;
- `remoteConnectionId`: stable provider MCP connection bound to one
  authenticated principal and one exact device;
- `connectionId`: short-lived transport connection identity bound to the
  stable `remoteConnectionId` and the current route and connection epoch;
- `sequence`: per-connection monotonically increasing integer starting at
  1 for ordered frame classes;
- `replayNonce`: unpredictable per-frame value for replay and duplicate
  detection;
- `correlationId`: client-chosen request correlation echoed in the response;
- `messageKind`: one frozen frame kind (section 4.4);
- `payloadLength`: exact byte length of the MCP payload;
- `createdAt` / `expiresAt`: creation and expiry timestamps; maximum frame
  lifetime is 120 seconds;
- `channelAuth`: device-channel authentication material (challenge-derived,
  never a private key);
- `authorizationEnvelope`: the device-verifiable authorization envelope
  (section 5).

A frame missing any field, or carrying an unknown field that changes
routing, identity, or authorization semantics, is malformed and rejected.

### 4.3 Response frame fields

A response frame carries the request `correlationId`, the responding
`routeDeviceId`, `remoteConnectionId`, and `connectionId`, its own
`sequence` and `replayNonce`, `messageKind`, `payloadLength`,
`createdAt` / `expiresAt`, `channelAuth`, and either the MCP result or one
typed failure code from the frozen vocabulary (section 8).

### 4.4 Frozen frame kinds

Only these message kinds exist:

- `mcp_request`: one MCP tool call or MCP protocol message toward the device;
- `mcp_response`: the matching result or typed failure toward the provider;
- `mcp_error`: a transport-level rejection (duplicate, expired, oversized,
  malformed, cross-route, impossible order) that never reaches local dispatch;
- `cancel`: remote cancellation propagated toward the local edge;
- `heartbeat`: channel liveness only; carries no request, no result, and no
  authority.

No other kind may be introduced without a new authorized grain.

### 4.5 Denied relay capabilities

The following are explicitly denied and must never appear as relay
capabilities, frame kinds, or payload shapes:

- arbitrary TCP forwarding;
- SOCKS proxying;
- HTTP CONNECT proxying;
- generic HTTP forwarding to arbitrary destinations;
- arbitrary WebSocket tunneling;
- arbitrary destination forwarding;
- remote shell transport;
- generic command or operation dispatch;
- schema-fetch-and-execute patterns;
- hidden subcommands inside safe-looking tools.

Contract tests prove these tokens are absent from the relay transport
sources.

## 5. Device-verifiable authorization envelope

Before local dispatch, the local edge verifies an envelope that binds at
minimum:

- the authenticated remote principal;
- the stable `remoteConnectionId` and current short-lived `connectionId`;
- the selected device;
- the request identity and SHA-256 request digest computed over the
  canonical frame binding (principal, connection identifiers, device,
  correlation ID, sequence, MCP method, and payload digest);
- the relevant route and connection epoch;
- the relevant session context.

A malicious or compromised relay must not successfully remap principal A's
request onto device B: local dispatch rejects a remapped principal-to-device
route even when the relay or device channel itself is authenticated, because
the envelope digest no longer matches the verified binding.

The envelope is verified before workspace policy, lease, profile, and
approval checks; any denial wins and failures are typed (section 8).

## 6. Replay, sequencing, duplication, and order

- ordered frame classes require an exact next `sequence`; gaps suspend the
  affected correlation with `RELAY_SEQUENCE_INVALID` and never silently skip;
- every frame carries a single-use `replayNonce`; a repeated nonce is a
  replay with typed `RELAY_REPLAY_DETECTED`;
- duplicate delivery (same `correlationId` with an already answered request)
  returns the recorded result once where the operation is idempotent and
  deduplicated by `quntald`, and otherwise fails closed without re-execution;
- reordered frames that create an impossible order fail with the typed
  transport error; ordering semantics are defined per frame kind and
  concurrency uses independent correlation IDs;
- reconnect never rewrites request IDs, correlation IDs, or approval
  digests; stale or expired approvals fail closed after reconnect.

## 7. Reconnect, queue, cancellation, and expiry

- transport reconnect has a bounded grace of at most 60 seconds and preserves
  original request and approval expiry semantics; reconnect extends nothing;
- while a device is briefly disconnected, at most 16 frames may wait with a
  queue lifetime of at most 60 seconds and never past request or lease
  expiry; queued items expire before dispatch, in order, without execution;
- there is no durable offline queue for mutating calls and no durable
  offline queue for read calls; a device that is offline, lease-suspended,
  locked, expired, or revoked produces typed `DEVICE_OFFLINE` or
  `REMOTE_SESSION_INACTIVE` rather than silently persisting work;
- when the user later restores a lease, old calls do not replay;
- remote cancellation propagates to the local MCP edge and kernel request
  where supported and is never described as verified process termination
  unless the existing provider establishes termination; transport close does
  not fabricate process state;
- mutating operations are never automatically retried after dispatch unless
  a stable operation ID is deduplicated by `quntald` with result replay;
  retries are otherwise limited to idempotent reads.

## 8. Frozen typed-failure vocabulary

These 17 codes are frozen. They remain distinct from local
`CAPABILITY_DENIED`, `WORKSPACE_DENIED`, and approval failures:

- `TRANSPORT_UNAVAILABLE`
- `REMOTE_AUTH_REQUIRED`
- `REMOTE_AUTH_INVALID`
- `REMOTE_SCOPE_DENIED`
- `DEVICE_OFFLINE`
- `DEVICE_REVOKED`
- `PAIRING_REQUIRED`
- `PAIRING_EXPIRED`
- `PAIRING_DENIED`
- `ROUTE_MISMATCH`
- `RELAY_REPLAY_DETECTED`
- `RELAY_SEQUENCE_INVALID`
- `REMOTE_RATE_LIMITED`
- `REMOTE_QUEUE_EXPIRED`
- `REMOTE_SESSION_INACTIVE`
- `MCP_PROTOCOL_UNSUPPORTED`
- `TOOL_SURFACE_DENIED`

A valid remote credential without an active local remote-session lease
produces `REMOTE_SESSION_INACTIVE`; it never creates a prompt the remote
model can satisfy.

## 9. Quotas, rate limits, and backpressure

Hard limits (a successor grain implements them; this contract freezes them):

- maximum frame and result payload: 1048576 bytes;
- maximum concurrent MCP requests per connection: 4;
- maximum concurrent remote connections per device: 2;
- maximum queued frames while briefly disconnected: 16 with 60-second life;
- per-device request rate: 60 per minute with backpressure to the device
  channel; overload fails closed with `REMOTE_RATE_LIMITED` or
  `REMOTE_SESSION_INACTIVE`, never with unbounded queueing;
- approval-prompt rate from remote traffic: at most 6 per minute at the
  local edge (compromised-relay flooding and approval-fatigue defense);
- authentication failures: 5 per minute per source, then backoff;
- pairing attempts: 5 per code, then the code is invalidated;
- heartbeat every 30 seconds; idle device channel suspends after 120 seconds;
  idle sessions follow the lease, whose hard maximum is 15 minutes.

Quota exhaustion on a community relay fails closed with typed errors and
never overflows into paid capacity automatically.

## 10. Privacy and logging

- the relay holds request and result bodies only transiently in process
  memory and never persists payloads;
- application logs and metrics carry route and result classes only: no raw
  tool arguments, no results, no file or clipboard content, no process
  output, no screenshot bytes, no secrets, no approval payloads, and no
  browser content;
- authentication and rate-limit records keep minimum routing metadata with
  documented retention of at most 30 days and support deletion on
  revocation;
- support bundles exclude payloads; a secret scanner and log tests guard the
  relay sources;
- the privacy statement says transient processing honestly and never claims
  provider-to-device end-to-end encryption.

## 11. Offline and workstation semantics

Remote dispatch additionally requires an active finite local remote-session
lease as defined in `docs/security/REMOTE_SESSION_AUTHORIZATION.md`. When
the device is offline, the lease is suspended or expired, the workstation is
locked, or the user logged off:

- no read and no mutation executes;
- nothing is queued for later surprise execution;
- the remote client receives the typed unavailable or session-inactive
  result;
- a short transport retry window may exist only before local dispatch and
  cannot outlive the original request or lease expiry.

Lease creation and widening require STRONG local presence; reconnect, OAuth
refresh, relay restart, and provider retry never extend the lease.

## 12. Remote threat contract

The relay protocol answers each ratified threat as follows:

- cross-tenant routing (UC-T01): principal plus selected-device route key,
  exact device-channel authentication, no fallback device, hard
  `ROUTE_MISMATCH`;
- frame replay and reordering (UC-T11, UC-T12): connection-scoped sequence
  plus single-use nonce, duplicate rejection, typed replay and sequence
  errors, independent correlation IDs;
- route substitution after reconnect (UC-T13): same device key and route
  epoch binding, no migration without new pairing;
- offline queued mutation or disclosure (UC-T14): no durable queue for
  reads or writes, bounded grace only, `DEVICE_OFFLINE` and
  `REMOTE_SESSION_INACTIVE` instead of silent persistence;
- relay as pivot proxy (UC-T24): MCP frames only, named tools only at the
  local edge, denied capability list in section 4.5;
- relay operator impersonation (UC-T35): device-verifiable authorization
  envelope verified locally before dispatch, plus the mandatory local lease;
- compromised-relay flooding (UC-T30b): per-device rate, concurrency, and
  approval-prompt limits with backpressure and fail-closed overload;
- wrong-principal result delivery (UC-T22): tenant plus device plus
  connection plus request correlation on the response path;
- log and payload leakage (UC-T21): transient-only bodies, class-only logs,
  deletion on revocation;
- cancellation ambiguity (UC-T29): propagation without fabricated
  termination claims;
- result amplification (UC-T36): per-tool output ceilings inherited from
  providers plus the relay hard result ceiling with explicit truncation.

OAuth, device identity, pairing, revocation, uplink, edge, deployment, and
adversarial exit details belong to SG-000053 through SG-000058 and must stay
consistent with this contract.

## 13. Successor implementation duties

SG-000053 implements device identity, pairing, and revocation against the
envelope and epoch bindings frozen here. SG-000054 implements OAuth 2.1 and
the default-deny scope matrix without changing this frame contract.
SG-000055 implements the outbound-only uplink with the grace, queue, and
backpressure bounds frozen here. SG-000056 implements the public edge with
the same tool registry and no edge-specific tools. SG-000057 ships the
open-source relay and reference deployment with the quota and privacy rules
frozen here. SG-000058 proves the adversarial matrix, including relay
compromise, flooding, replay, reorder, reconnect, and queue-after-revoke
cases, against this contract.

## 14. Non-goals of this contract

This contract does not implement, deploy, or qualify any network path. It
does not choose relay hosting, relay code, cryptography libraries, OAuth
providers, or deployment accounts. It does not widen any local tool,
approval, workspace, or installer authority. The relay device transport
remains a fail-closed skeleton until its implementation grain is canonical.
