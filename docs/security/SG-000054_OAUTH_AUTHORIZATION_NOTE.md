# SG-000054 OAuth 2.1 Authorization Note

Status: IMPLEMENTATION FOR COTRA-P15
SpecGrain: SG-000054
Base: `26075a9966535f029053d379ca3ef1b910b633c2` (grain base `518406cffdd2cd37625f35a87580c0e2c49483c5`)
Date: 2026-10-02
Companions:
- `docs/security/RELAY_PROTOCOL_CONTRACT.md` (SG-000052, frozen)
- `docs/security/SG-000053_DEVICE_IDENTITY_NOTE.md`
- `docs/security/REMOTE_PRINCIPAL_AUTH_MODEL.md`
- `docs/security/REMOTE_SESSION_AUTHORIZATION.md`
- `docs/security/CLIENT_CONNECTION_PROFILE_MODEL.md`
- `apps/cotra-mcp/src/oauth_authorization.ts`
- `apps/cotra-mcp/src/oauth-authorization.test.ts`

## 1. Purpose

SG-000054 implements standards-based OAuth 2.1 authorization for remote
principal identity against the frozen SG-000052 relay contract and the
SG-000053 device identity. It is pure validation logic: no listener, no
socket, no HTTP server, no outbound request, no device uplink, no public
edge, no relay deployment, no new MCP tool, no schema change, no
capability widening, and no approval change.

OAuth proves remote principal identity and nothing else.

## 2. Authority boundary

A verified access token yields a `RemotePrincipalIdentity` whose
`authority` field is the literal `remote_identity_only`. It carries
opaque principal, connection, device, epoch, client, scope, family, and
token identifiers. It carries no workspace, trust, approval, presence,
capability, or lease field, and the contract tests assert that.

`OAUTH_LOCAL_AUTHORITY_GRANTED` records, and tests pin, that OAuth grants
none of: workspace trust, local approval, STRONG presence, capability
authority, filesystem authority, process authority, browser authority, UI
authority, or a remote-session lease. The module imports no kernel,
server, process, or transport module, and the SG-000052 relay device
transport skeleton does not import it.

Effective remote permission remains the intersection defined in
`REMOTE_PRINCIPAL_AUTH_MODEL.md` section 12: remote OAuth scope AND the
active local remote-session lease AND the locally enabled tool-surface
profile AND workspace policy AND the `cotrad` capability and provider
ceiling AND required local approval. Any denial wins.

## 3. Cryptography

- Only maintained `node:crypto` primitives are used: Ed25519 signing and
  verification, SHA-256, `randomBytes`, and `timingSafeEqual`. No
  cryptographic primitive is implemented from scratch and no new
  dependency is added.
- Access tokens are RFC 9068 shaped JWS compact tokens with header
  exactly `{alg, typ, kid}`, `alg` pinned to `EdDSA`, and `typ` pinned to
  `at+jwt`. `none`, symmetric (`HS*`), RSA, and any embedded or remote key
  header (`jwk`, `jku`, `x5u`, `crit`, and every other extra header)
  deny. The verification key is selected by `kid` from the configured
  set only, must be an Ed25519 public JWK, and must not carry private
  key material.
- Base64url decoding is strict and canonical; padding or non-canonical
  encodings deny. Tokens longer than 8192 characters deny before parsing.

## 4. Access token validation

Every remote request validates, in order: presence, size, three-segment
shape, header, algorithm, key, signature, exact claim set, claim types and
opaque identifier shapes, issuer equality, audience equality with the one
configured resource (single string; arrays deny), lifetime at most 600
seconds, issued-at and not-before within a 30-second skew, expiry,
scopes, token revocation, family revocation, SG-000053 route revocation
(hard device, principal-wide, all-route, emergency), and equality of the
token epoch with the current recorded device epoch. An unknown device or
any epoch drift denies.

The default access token lifetime is 300 seconds.

## 5. Authorization code flow with PKCE S256

- Authorization requests require a registered client, an exactly matching
  registered redirect URI (no redirect on mismatch), `response_type=code`,
  `code_challenge_method=S256` with a 43-character base64url challenge,
  `resource` exactly equal to the configured protected resource, known
  scopes within the client's allowed scopes, and CSRF `state` of 16 to
  512 characters. `plain` PKCE, implicit, password, client credentials,
  and device-code grants are not supported.
- Transactions expire after 600 seconds. The remote identity binding
  (principal, connection, device, epoch) is attached only after SG-000053
  device pairing and proof; malformed bindings refuse code issuance.
- Codes carry 256-bit entropy, only their SHA-256 hash is stored, they
  expire after 60 seconds, and they are one-shot. A matching code is
  consumed even if a later binding check fails. Replay of a consumed code
  denies and reports `revokeIssuedFamily` so tokens from it are revoked.
- Redemption binds client, redirect URI, resource, and PKCE verifier.
- The authorization response carries the RFC 9207 `iss` parameter and the
  client-side check rejects issuer mix-up and state mismatch.

## 6. Metadata

- Authorization-server metadata (RFC 8414) advertises only the `code`
  response type, `authorization_code` and `refresh_token` grants, `S256`,
  `none` and `private_key_jwt` client authentication, the three scopes,
  RFC 9207 issuer identification, and client ID metadata document
  support. Endpoints live under the issuer origin.
- Protected-resource metadata (RFC 9728) names exactly one resource, one
  authorization server, and header-only bearer usage.
- Discovered metadata is validated for exact issuer and resource, S256
  without `plain`, code-only response types, no forbidden grants, issuer
  identification, same-origin endpoints, and header-only bearer methods.
- Issuer and resource identifiers must be `https` without query,
  fragment, or credentials. Redirect URIs must be `https`, or `http` on an
  exact loopback IP literal for native clients.

## 7. Refresh family controls

- A refresh family is created only from a redeemed code. Refresh tokens
  have the shape `crt.<family>.<256-bit secret>`; only hashes are stored.
- Rotation is mandatory on every renewal. Replay of a previously rotated
  token revokes the whole family (bounded history of 64 hashes).
- Every renewal requires a fresh device-key proof: the token endpoint
  issues an SG-000053 challenge for the family's exact device and epoch,
  and the device signs it with its local key. The proof is bound to the
  family, expires after 120 seconds, and its nonce is one-shot per family
  (bounded history of 64 nonces).
- A missing proof denies with `device_proof_missing`. A device that is
  offline cannot produce a proof, so a stolen refresh token cannot renew
  while the bound device is offline. A proof from another key denies.
- Device, principal, all-route, or emergency revocation denies renewal
  with `DEVICE_REVOKED` and revokes the family. A device epoch change
  (for example key rotation) denies with `stale_epoch` and revokes the
  family; the user re-links under the new epoch.
- Families have an absolute lifetime of 7 days regardless of rotation.
- Scope on refresh can only narrow; widening or unknown scopes deny.
- Client and resource are bound to the family.

## 8. Token revocation

`applyTokenRevocation` implements RFC 7009 semantics: revoking a refresh
token revokes its family, which also invalidates every access token
minted from it; revoking an access token revokes that token identifier.
Unrecognized input changes nothing and reveals nothing.

## 9. Provider client-identification hooks

Client registrations carry an identification method
(`client_id_metadata_document`, `dynamic_client_registration`,
`preregistered`, or `private_key_jwt`). Verification runs a pluggable hook
for that method. A missing hook, a method mismatch, a client mismatch, a
throwing hook, or any result other than exactly `true` denies.
Provider-specific hooks are deliberately absent: they are added only by a
later grain after current provider requirements are reverified.

## 10. Default-deny scope to tool and profile ceiling

`OAUTH_SCOPE_TOOL_MATRIX` states the required scopes for every one of the
20 canonical tools and is tested to cover exactly `CANONICAL_TOOL_NAMES`:

- `cotra.read`: `system_status`, `workspace_get`, `fs_stat`, `fs_list`,
  `fs_read`, `fs_search`, `git_status`, `git_diff`, `git_log`;
- `cotra.write`: `fs_write_preview`, `fs_write`, `git_branch_create`,
  `git_stage`, `git_unstage`, `git_commit`, `git_fetch_preview`,
  `git_fetch`, `git_push_preview`, `git_push`;
- `cotra.execute`: `process_spawn`.

Preview tools that exist to prepare a write require `cotra.write`. Every
required scope must be present in both the current token and the leased
OAuth scope ceiling. Only the `core` profile is mapped; `developer`,
`desktop_structured`, `coordinate_fallback`, and any other profile deny
with `TOOL_SURFACE_DENIED` until a later governed grain (SG-000065) maps
them. Unmapped tools (including prototype keys such as `__proto__`)
deny. Unknown scopes deny with `REMOTE_SCOPE_DENIED`. A positive scope
decision is an outer ceiling only and never sufficient for dispatch.

## 11. Failure vocabulary

Only frozen SG-000052 codes are used: `REMOTE_AUTH_REQUIRED`,
`REMOTE_AUTH_INVALID`, `REMOTE_SCOPE_DENIED`, `DEVICE_REVOKED`,
`ROUTE_MISMATCH`, `RELAY_REPLAY_DETECTED`, and `TOOL_SURFACE_DENIED`. A
detailed `reason` accompanies each denial for audit; reasons never carry
token, code, verifier, key, or payload material.

## 12. Privacy

Stored records hold only hashes of codes and refresh tokens. The
redacted identity summary carries opaque identifiers and scope names
only. No token, pairing code, PKCE verifier, private key, or MCP payload
appears in records, reasons, or summaries.

## 13. Negative coverage

`oauth-authorization.test.ts` proves fail-closed behavior for: expired
token, not-yet-valid token, over-long lifetime, revoked token, revoked
family, revoked device, principal, and emergency route, wrong issuer
(token, metadata, and authorization response), wrong audience (string,
multi-valued array, and resource mismatch), wrong resource (authorization
request, code redemption, refresh, and metadata), missing scope, unknown
scope, scope widening on refresh, stolen refresh token with the device
offline, missing device proof, forged device proof, mismatched proof
family, expired proof, replayed proof, stale device epoch on access and
refresh, refresh replay after rotation, authorization code replay and
expiry, PKCE mismatch and `plain`, redirect mismatch, unknown client,
malformed tokens, tampered payloads, `alg=none`, HS256 confusion, embedded
and remote key headers, unknown keys, forged signatures under a known
`kid`, extra claims, and non-opaque subjects.

## 14. Scanner precision change

`surface.test.ts` forbids references to the `cotra.exe` lifecycle
executable in tool sources. Its pattern `/cotra\.exe/i` also matched the
canonical OAuth scope name `cotra.execute`. The pattern is now
`/cotra\.exe\b/i`, which still matches every reference to the executable
and no longer matches the scope name. No other regression test changed.

## 15. Non-goals

No device uplink (SG-000055), no public `/mcp` edge (SG-000056), no relay
deployment (SG-000057), no remote-session lease implementation, no HTTP
endpoints for the authorization server, no provider-specific
identification, no provider submission, and no P16 tool-surface widening.
