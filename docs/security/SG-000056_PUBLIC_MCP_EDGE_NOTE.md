# SG-000056 Public Streamable HTTP MCP Edge Note

Status: IMPLEMENTATION FOR QDRAL-P15
SpecGrain: SG-000056
Base: `d44666bd10d7c53a88af78d4f3f34acdf98155ea`
Date: 2026-10-02
Companions:
- `docs/security/RELAY_PROTOCOL_CONTRACT.md` (SG-000052, frozen)
- `docs/security/SG-000054_OAUTH_AUTHORIZATION_NOTE.md`
- `docs/security/SG-000055_DEVICE_UPLINK_LEASE_NOTE.md`
- `apps/qdral-relay/src/edge.ts`, `device_channel.ts`, `server.ts`, `store.ts`

## 1. Purpose

SG-000056 adds the relay side of remote access as a separate open-source
workspace package, `@qdral/relay`: the public Streamable HTTP `/mcp` edge,
RFC 9728 protected-resource metadata, and the relay side of the SG-000055
device channel. The relay reuses the frozen SG-000052 vocabulary, the
SG-000053 device identity checks, the SG-000054 token verification and scope
ceiling, and the SG-000055 frame digest and session-context helpers; nothing
is redefined. The authorization server endpoints, pairing enrollment,
durable storage, quotas, self-host packaging, and the reference deployment
are SG-000057.

## 2. Public `/mcp` edge

- One JSON-RPC 2.0 message per `POST`; batches, client responses, and
  non-JSON bodies are rejected with 400. `GET` returns 405 because no
  server-initiated stream is offered; `DELETE` ends a session.
- `Content-Type` must be `application/json` (415), `Accept` must allow
  JSON (406), and a present `MCP-Protocol-Version` must be one of the SDK's
  supported versions (400).
- Bodies are capped at 1 MiB of payload (413) before parsing.
- Sessions: `initialize` creates a session and returns `Mcp-Session-Id`
  (256-bit). Every later request must carry it (400 when absent, 404 when
  unknown, expired, or bound to another principal, route, device, or epoch).
  At most 2 sessions per paired route (the device connection bound) and
  1024 in total (429). Sessions expire after 100 idle seconds, below the
  device's 120-second idle suspend, so a session never outlives its device
  state. An `initialize` that fails or returns a JSON-RPC error leaves no
  session.
- Origin: a request carrying an `Origin` header not on the configured
  allowlist is refused (403); the default allowlist is empty. No CORS header
  is ever emitted. The `Host` header must equal the configured public origin
  host (421), defeating DNS-rebinding-style requests.

## 3. Authentication and authority

- Every request requires `Authorization: Bearer`. A missing token returns 401
  with `WWW-Authenticate: Bearer resource_metadata="<origin>/.well-known/
  oauth-protected-resource/mcp"`; any verification failure adds
  `error="invalid_token"`.
- Tokens are verified with SG-000054 `verifyAccessToken` against the
  configured issuer and this exact resource, the stored verification keys,
  token and family revocation, route revocation, and current device epochs.
- The verified identity must match a stored, unrevoked paired route
  (principal, device, client) and an unrevoked device; otherwise 401.
- Session scopes are the token scopes intersected with the route's paired
  ceiling; a later token lacking the session scopes is refused with 403
  `insufficient_scope`.
- `tools/call` is checked with the SG-000054 default-deny scope to tool
  ceiling before any frame is created: missing scope returns 403
  `insufficient_scope`; unknown or unmapped tools return 403 with
  `TOOL_SURFACE_DENIED`.
- Remote context (principal, stable and short-lived connection, device,
  epoch, scopes) is derived only from the verified token and the stored
  route. Payloads and tool arguments never select it.
- The relay cannot create, widen, renew, or extend the local remote-session
  lease, and cannot satisfy any approval; the device and `qdrald` re-check
  everything (SG-000055), so the edge's typed lease denial is returned to the
  client unchanged.

## 4. Mapping onto the device

Each MCP session is one short-lived device connection (`cn-` plus 128-bit
hex). Each request becomes one frozen `mcp_request` frame with the next
connection sequence, a fresh replay nonce and correlation, a 60-second
lifetime, and the device-verifiable authorization envelope (principal,
stable connection, device, recomputed request digest, epoch, session
context). Notifications are forwarded and answered with 202. The response
must be a JSON-RPC message with the same id (otherwise 502). Typed device
failures map to HTTP: `DEVICE_OFFLINE` and `TRANSPORT_UNAVAILABLE` 503,
`REMOTE_RATE_LIMITED` 429, `REMOTE_QUEUE_EXPIRED` 504, scope, tool, route,
and revocation failures 403, authentication failures 401.

## 5. Device channel (relay side)

- `challenge` issues an SG-000053 challenge only for a registered, unrevoked
  device at its exact epoch; unknown devices get the same generic 401.
- `session` verifies the signed challenge once (the challenge is consumed
  whether or not verification succeeds) and opens exactly one channel per
  device with a 256-bit token, closing any previous channel.
- `poll` long-polls for at most 25 seconds and returns at most 16 frames;
  frames past their expiry are discarded, never delivered late.
- `push` accepts at most 16 frames, each with exactly the 14 response fields,
  the frozen protocol, this device, this channel token, a single-use nonce,
  an unexpired lifetime, a correlation to a pending request on a session
  bound to the same device and route, and either a bounded response payload
  or a frozen failure code. Anything else is dropped. Pushes may arrive
  concurrently, so response order is not required; correlations are
  one-shot.
- A device is online only while it has polled within 35 seconds. Requests to
  an offline device, or beyond the 16-frame queue, are refused before a
  connection sequence is allocated, so a session's sequence stays
  contiguous. Nothing is persisted or replayed. A revoked device loses its
  channel on its next poll or push.
- Pending requests time out after 55 seconds, below the 60-second frame
  lifetime. A request that was never delivered is converted in place into a
  `cancel` for the same correlation and sequence (fresh nonce, recomputed
  digest), so the device can never execute it late and the connection stays
  contiguous; the client receives `REMOTE_QUEUE_EXPIRED`. A request already
  delivered has an unknown outcome and is reported as
  `TRANSPORT_UNAVAILABLE`, never as not executed. Closing a channel reports
  `DEVICE_OFFLINE` for undelivered and `TRANSPORT_UNAVAILABLE` for delivered
  requests.
- Failures that break or obscure sequence continuity (`DEVICE_OFFLINE`,
  `TRANSPORT_UNAVAILABLE`, `RELAY_SEQUENCE_INVALID`,
  `RELAY_REPLAY_DETECTED`, `ROUTE_MISMATCH`, `DEVICE_REVOKED`) end the MCP
  session, so the client re-initializes instead of continuing a broken
  connection.

## 6. Resource bounds and logging

`/mcp` bodies 1 MiB plus framing, push bodies 16 full frames, other device
bodies 16 KiB, 4 concurrent requests per session, 2 sessions per route,
1024 sessions total, 16 queued frames per device, 8192 remembered response
nonces, header timeout 10 s, request timeout 70 s, keep-alive 5 s, 64
headers. Logs record route and status classes only (`mcp_<status>`,
`device_<endpoint>_<status>`, `body_too_large`, `host_rejected`), never
tokens, payloads, arguments, or results.

## 7. Loopback self-host identifiers

SG-000054 resource and issuer identifiers now also accept plain `http` on an
exact loopback IP literal (`127.0.0.1` or `[::1]`), matching the SG-000055
relay-origin rule, so a self-hosted relay can serve on the same machine.
Host names, including `localhost`, and every non-loopback `http` origin
remain rejected, and tests pin both directions.

## 8. Source guarantees

Relay sources contain none of the SG-000052 denied capability tokens, use no
`node:child_process`, `node:net`, WebSocket, `eval`, or `new Function`,
register no MCP tools, and make no outbound requests: the relay never opens
connections anywhere, devices connect to it.

## 9. Tests

`apps/qdral-relay/src/relay.test.ts` runs a real loopback relay, a real
SG-000055 device uplink client, and the authoritative MCP builder with a
stub kernel: an authenticated client initializes, lists exactly the 20-tool
catalog, calls `fs_read`, and receives the typed `REMOTE_SESSION_INACTIVE`
lease result with the kernel seeing exactly the token-derived remote
context. Negative tests cover missing, malformed, expired, wrong-audience,
wrong-issuer, unknown-scope, revoked-token, revoked-route, and stale-epoch
tokens; missing scope, unknown tools, and narrowed tokens; cross-route
session use and the per-route session bound; an offline device; batches,
client responses, bad JSON, content type, Origin, protocol version, body
size, Host, and unknown paths; and unknown devices, wrong epochs, forged and
replayed challenge responses, bad channel tokens, and revoked devices on the
channel endpoints. None of the rejected requests reaches the device kernel.

`apps/qdral-relay/src/device-channel.test.ts` proves refusals precede
sequence allocation, a request timing out before delivery becomes a
`cancel` that the real SG-000055 device core accepts without executing it
while executing the next request, timeouts after delivery and channel
closure report unknown outcomes honestly, push accepts only exact,
correlated, single-use responses for the channel's device and route, and
revoked or unproven devices never hold a channel.
