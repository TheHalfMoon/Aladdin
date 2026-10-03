# SG-000057 Open-Source Relay, Self-Host Mode, and Zero-Cost Deployment Note

Status: IMPLEMENTATION FOR QUNTAL-P15
SpecGrain: SG-000057
Base: `ca494131070a5cdb9c133bef0db69e6292c2d085`
Date: 2026-10-02
Companions:
- `docs/security/SG-000054_OAUTH_AUTHORIZATION_NOTE.md`
- `docs/security/SG-000055_DEVICE_UPLINK_LEASE_NOTE.md`
- `docs/security/SG-000056_PUBLIC_MCP_EDGE_NOTE.md`
- `docs/relay/SELF_HOSTING.md`, `docs/relay/REFERENCE_DEPLOYMENT.md`
- `apps/quntal-relay/src/authorization.ts`, `file_store.ts`, `quotas.ts`,
  `config.ts`, `main.ts`, `Dockerfile`
- `apps/quntal-mcp/src/remote_enrollment.ts`,
  `apps/quntal-mcp/src/entrypoints/remote_admin.ts`
- `crates/quntald/src/remote_lease.rs` (`authorize_enrollment`),
  `crates/quntal-lifecycle/src/remote.rs` (`run_enrollment`)

## 1. Purpose

SG-000057 completes the relay as a first-class self-hostable service with
zero mandatory cost: the authorization server with device-backed pairing,
device-proof refresh over the live channel, durable state, explicit
fail-closed quotas, a validated self-host runtime and container, and an
optional reference deployment definition that is never required.

## 2. Authorization server

- RFC 8414 metadata at `/.well-known/oauth-authorization-server`; the issuer
  is the relay's public origin and the protected resource is `<origin>/mcp`.
- RFC 7591 dynamic registration for public clients only
  (`token_endpoint_auth_method` `none`, PKCE required): unknown fields,
  non-https redirect URIs (except exact loopback), grants other than
  `authorization_code` and `refresh_token`, response types other than `code`,
  and unknown scopes are rejected. Registrations are bounded by quotas.
- `/oauth/authorize` validates the request with SG-000054
  `validateAuthorizationRequest`. Unknown clients and mismatched redirects get
  an error page and are never redirected; other errors redirect with
  `error`, `state`, and the RFC 9207 `iss`. The page shows the client name,
  return origin, and scopes, asks for a pairing code, and is served with a
  restrictive CSP (`default-src 'none'`, no script), `X-Frame-Options: DENY`,
  and `Referrer-Policy: no-referrer`.
- `/oauth/token`: authorization code with PKCE S256 through SG-000054
  `redeemAuthorizationCode` (one-shot; replay revokes every token issued from
  the code), and refresh through `redeemRefreshToken` with a fresh device
  proof. `password`, implicit, and client-credentials grants do not exist.
- `/oauth/revoke` implements RFC 7009 for refresh tokens (whole family) and
  access tokens (token identifier), bound to the presenting client, and
  always answers 200.

## 3. Device-backed pairing

- `quntal remote enable --relay <origin>` and `quntal remote pair` first ask
  `quntald` for STRONG presence (`remote.enrollment.authorize`, digest bound to
  the action, relay origin, workspace, and policy revision). Remote contexts
  can never reach this capability (SG-000055 local-only `remote.` prefix).
- `enable` creates the Ed25519 device key locally (owner-only file), writes
  the uplink configuration, and registers only the public identity. The relay
  refuses a different key for an existing device identifier (409) and any
  revoked device (403).
- `pair` creates one SG-000053 pairing code (256-bit) and registers only its
  SHA-256 hash as an offer signed with a fresh device challenge; offers live
  at most 300 seconds and each device has at most one.
- The user types the code on the authorization page; at most 5 attempts per
  authorization transaction, comparison is constant-time, and the page never
  reveals whether a device exists.
- The device then shows the exact client name and identifier, return origin,
  scopes, and route, and the user must type `yes`. The confirmation is signed
  with a fresh one-shot device challenge. Only then does the relay record the
  route and issue a one-shot authorization code; the device records the
  pairing only when the relay's result matches what it approved. Declining
  sends `access_denied` to the client.
- Pairing proves possession of the device only. It grants no workspace
  trust, no approval, and no remote-session lease.

## 4. Refresh requires the device to be online

The token endpoint creates an SG-000054 family-bound challenge for the bound
device and epoch and delivers it on the live device channel (long-poll
`proofs`); the device signs only challenges for its own identifier and
epoch with a lifetime of at most 120 seconds that have not expired, at most
16 per poll. Without a live channel the request fails immediately with
`device_proof_missing`, so a stolen refresh token cannot renew while the
device is offline. Rotation, replay detection (family revocation), absolute
lifetime, scope narrowing, and revocation checks are the SG-000054 rules.

## 5. Durable state

`relay-state.json` is written atomically with owner-only permissions,
protected by a SHA-256 checksum, capped at 64 MiB, and bounded per list. A
corrupt or tampered file stops the relay from starting rather than dropping
revocations. Revoking a device or route deletes its refresh families. The
file holds no payloads, bearer tokens, or device private keys; it does hold
the relay's own signing key and must be protected as a secret.

## 6. Quotas and no-billing overflow

Every limit has a configured value bounded by a hard ceiling; unknown quota
fields are rejected. Exhaustion of the per-hour registration, per-minute
authorization, token, and per-route MCP windows, or of the daily request
budget, fails closed with HTTP 429, `REMOTE_RATE_LIMITED`, and `Retry-After`.
There is no autoscaling and no paid overflow setting.

## 7. Self-host runtime

`node dist/main.js --config relay.json` loads a strict configuration (https
or exact-loopback public origin without path, IP-literal listener defaulting
to loopback, bounded quotas). The container image builds from a
digest-pinned Node 24.19.0 Alpine base and runs as an unprivileged user with
a health check. CI builds the image, runs it from a configuration, and checks
health, both metadata documents, the unauthenticated 401, durable state
creation, and that logs contain no token material.

## 8. Reference deployment

`docs/relay/REFERENCE_DEPLOYMENT.md` defines an optional free-tier shape and
an operator verification checklist. The project does not operate or claim a
live hosted relay; self-hosting remains the canonical zero-cost path.

## 9. Tests

`apps/quntal-relay/src/self-host.test.ts` drives the complete standards flow
against a real loopback relay with durable state: dynamic registration,
authorization page, device pairing with local confirmation, PKCE code
exchange, the device uplink, MCP over the public edge with the 20-tool
catalog, device-proof refresh with rotation, refresh replay revoking the
family, and access-token revocation at the edge; plus code replay revoking
issued tokens, offline refresh refusal, device decline, strict registration
and authorization validation with attempt limits, restart persistence with
the same signing key, corrupt-state refusal, quota exhaustion, and strict
configuration. Device-side tests cover proof-signing rules; quntald and
lifecycle tests cover the STRONG enrollment gate and relay origin shapes.
