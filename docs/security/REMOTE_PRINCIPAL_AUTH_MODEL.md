# Qdral Remote Principal and Authorization Model

Status: IMPLEMENTATION-READY PROPOSAL
Date: 2026-10-01
Planning base: `5feff3f15cc7e20464cafffd7b87d713b65a012f`
Companions:
- `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md`
- `docs/security/UNIVERSAL_CONNECTIVITY_THREAT_MODEL.md`
- `docs/security/REMOTE_SESSION_AUTHORIZATION.md`
- `docs/research/UNIVERSAL_CLIENT_COMPATIBILITY.md`

## 1. Problem

Hosted AI clients need a stable authenticated principal before Qdral can route a remote MCP request to one user's computer.

Qdral must provide that identity without making a paid identity provider, email/password database, social login, or founder-funded SaaS subscription mandatory.

At the same time, OpenAI public plugins and standards-compliant remote MCP clients need OAuth 2.1 semantics for private data and write actions.

This document defines the default Qdral solution.

## 2. Decision: device-backed remote principal

The v0.2 default is a **device-backed remote principal**.

The local Qdral installation is the root of user possession for initial remote linking. A hosted provider connection is paired to exactly one Qdral device by an explicit local flow.

No mandatory Qdral email account, password, phone number, social login, or third-party identity subscription is required.

The relay/auth service stores only opaque principal/device/connection identifiers, device public keys, OAuth client/token state, route/revocation epochs, and the minimum abuse/rate-limit metadata required to operate the service.

The device private key remains local.

## 3. Identity objects

### 3.1 `device_id`

A random stable identifier for one Qdral installation. It is not derived from hardware serials, Windows username, MAC address, machine SID, hostname, or other fingerprinting data.

### 3.2 device key pair

Generated locally during remote setup and stored in Qdral protected state.

Used to authenticate the device channel and prove possession during pairing/refresh-sensitive transitions.

### 3.3 `remote_principal_id`

An opaque relay/auth identifier created during the first successful pairing.

It is not an email address or provider account identifier.

### 3.4 `remote_connection_id`

One provider MCP connection bound to:

- one `remote_principal_id`;
- one exact `device_id`;
- one OAuth client identity/provider connection;
- one granted OAuth scope set;
- one connection/revocation epoch.

`remote_connection_id` is the stable provider-connection identity used by leases, revocation, and audit. It is distinct from the short-lived transport `connection_id` defined in `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md` section 6. A lease binds the stable `remote_connection_id` together with its current short-lived `connection_id` and epoch, so lease and revocation checks never target different connection identities.

**v0.2 deliberately binds one remote provider connection to one device, and one remote principal to one device.**

A user who wants to connect a second PC creates a second provider connection under a new `remote_principal_id`. This avoids ambiguous device selection and cross-device routing in the first universal release. There is no cross-device enrollment of a new device into an existing principal in v0.2; each device is an independent principal with per-device revocation, and principal-wide revocation means revoking every connection owned by that single-device principal plus every paired device route for that connection. Cross-device principal roaming is an explicit non-goal for v0.2.

### 3.5 `client_session_id`

Generated locally by Qdral for each live MCP session. It remains the session identifier used by local audit/policy and is not replaced by provider or relay session IDs.

## 4. First-time remote setup

The user explicitly runs a local command such as:

```text
qdral remote enable
```

This operation:

1. creates the device key pair if absent;
2. creates the opaque `device_id`;
3. stores private material under existing Qdral protected-state rules;
4. registers only the device public key and minimal route metadata with the selected relay;
5. requires STRONG local user presence before the device is eligible for remote pairing;
6. does not grant any workspace, capability, tool profile, or action approval.

`remote enable` is a PRIVILEGED local lifecycle action and is never exposed as an MCP tool.

## 5. Provider linking / OAuth user authentication

The public Qdral authorization server uses OAuth 2.1 authorization-code + PKCE for supported hosted MCP clients.

The user authenticates the OAuth authorization transaction through **local device pairing**, not a cloud password.

### 5.1 Linking flow

1. The hosted AI client discovers Qdral's protected-resource and authorization-server metadata.
2. The hosted client begins authorization-code + PKCE with the exact Qdral MCP resource.
3. The Qdral authorization page asks the user to pair a Qdral device. It does not ask for a Qdral password.
4. On the PC, the user runs:

   ```text
   qdral remote pair
   ```

5. Qdral requires STRONG local user presence and creates a short-lived one-time pairing transaction.
6. The user either:
   - opens a device-generated HTTPS pairing URL/QR token; or
   - enters a high-entropy one-time code into the authorization page.
7. The relay/auth service sends a fresh challenge to the candidate device.
8. The device signs the challenge and confirms the exact remote OAuth client/provider, requested scopes, and target device alias locally.
9. The server creates/binds `remote_principal_id` and `remote_connection_id` only after successful device proof and local confirmation.
10. The authorization page shows the requested Qdral scopes and the selected device.
11. The authorization server issues a short-lived authorization code bound to client, redirect URI, PKCE challenge, resource, principal, device, scopes, and transaction.
12. The provider exchanges it for scoped tokens.

Pairing proves possession/control of the local Qdral installation. It still does not grant workspace trust or approve tool effects.

## 6. Pairing token requirements

A pairing transaction must be:

- cryptographically random;
- at least 80 bits of effective entropy for any manually entered representation;
- preferably represented as an opaque URL token with at least 128 bits of entropy for QR/click flows, delivered so it is not persisted in browser history, bookmarks, or Referer headers and consumed/cleared as soon as the page redeems it;
- single use;
- short lived;
- bound to the candidate `device_id` and OAuth authorization transaction;
- rate limited by transaction, source, and device as appropriate;
- invalidated on success, expiry, revoke, or excessive failed attempts;
- excluded from application logs and metrics.

The authorization page must not reveal whether an arbitrary guessed device/principal identifier exists.

## 7. OAuth server requirements

The Qdral authorization server is security-critical.

It must use maintained standards libraries for OAuth/JWT/JOSE processing where practical and must not implement cryptographic primitives from scratch.

Required behavior includes:

- OAuth 2.1 authorization-code flow;
- PKCE S256;
- exact redirect allowlisting/registered client metadata;
- protected-resource metadata;
- authorization-server metadata;
- exact `resource` echo/binding;
- exact issuer handling;
- RFC 9207 authorization-response issuer identification when advertised;
- CIMD support for OpenAI where practical;
- DCR support where required by target clients;
- access-token signature/issuer/audience/resource/expiry/not-before/scope validation;
- refresh-token rotation or equivalent replay-resistant family handling;
- refresh family revocation;
- connection/device epoch checks;
- short authorization-code lifetime and one-shot redemption;
- CSRF transaction binding;
- token/key rotation strategy;
- no password grant;
- no implicit grant;
- no machine-to-machine grant as a substitute for the end-user connection.

## 8. Device-gated token lifetime

Qdral minimizes the consequences of losing a device without requiring a mandatory cloud account recovery system.

Default policy:

- access tokens are short lived;
- refresh/token renewal is allowed only while the bound device connection is still valid under the current device/connection revocation epoch;
- refresh additionally requires a fresh device-key proof or a short-lived device-signed online lease at the token endpoint, bound to the refresh family and the current connection epoch, so a stolen refresh token cannot renew while the legitimate device is offline or revoked;
- a revoked device or connection cannot renew a token family;
- a device that has been offline beyond the configured security window cannot silently renew indefinite remote access;
- reconnect never changes the bound `device_id`.

Exact lifetimes are implementation constants to be threat-tested and may be tightened per provider.

This design means loss/destruction of a device does not create an indefinitely renewable cloud credential merely because an old refresh token exists.

## 9. Revocation paths

The user can revoke remote access locally even when the provider connection remains configured.

Required commands/UX:

- list paired remote connections;
- revoke one provider connection;
- revoke every connection owned by one principal;
- revoke all remote connections for the device;
- revoke every paired device route for the account;
- rotate the device key;
- disable remote mode entirely, which revokes every connection and every paired device route;
- emergency revoke through the existing Qdral authority boundary, including a remotely reachable emergency revocation path that does not require physical access to a stolen or still-running device.

A remotely reachable emergency revocation path uses an authenticated principal session, such as a still-valid OAuth session or provider-side connector removal propagated to the relay, to revoke device routes and token families without requiring commands on the lost device. Short access-token lifetime, device-gated refresh with fresh device-key proof, and the finite local remote-session lease bound the residual window until revocation propagates.

Revocation increments the relevant epoch and invalidates:

- active relay routes;
- outstanding authorization transactions;
- access/refresh token families as applicable;
- device reconnect authorization for the revoked identity.

Provider-side disconnect is useful but is not the sole Qdral revocation mechanism.

## 10. Lost-device case

The default v0.2 system intentionally avoids pretending that an email/password cloud account exists when it does not.

If the only Qdral device is permanently lost:

- the short access-token lifetime limits residual access;
- device-gated renewal fails when the device cannot prove possession/current epoch with a fresh device-key proof;
- a stolen or still-running device that retains its valid key remains able to prove possession until revocation propagates, so the user must invoke remotely reachable emergency revocation through an authenticated relay session or provider-side connector removal, in addition to local commands when available;
- the finite local remote-session lease independently bounds remote dispatch after loss, and expiry, lock/logoff invalidation, or revocation denies even read-only dispatch;
- a replacement installation receives a new `device_id` and key pair;
- the user links the replacement as a new connection under a new principal;
- provider-side connector removal remains an additional cleanup path.

This document does not claim that a lost device is immediately contained without revocation. Containment requires revocation plus token and lease expiry.

An optional recovery credential or passkey-backed multi-device Qdral account may be designed later, but it is not required for the initial universal release and must not be silently introduced as a new cloud dependency.

## 11. Multiple devices

v0.2 does not route one provider connection dynamically among multiple computers.

Each connection is one device.

If a provider supports multiple connected accounts/connections, Qdral may expose a minimal authenticated profile identity so the user can distinguish connections, for example a user-chosen device alias. This profile must reveal no hostname, username, IP address, machine SID, or hardware fingerprint by default.

For OpenAI, any future profile tool must follow the platform's authenticated profile-tool convention and be independently reviewed as a tool-surface change.

## 12. Scopes and local profiles are independent

Initial remote OAuth scopes:

- `qdral.read`
- `qdral.write`
- `qdral.execute`

Normative scope-to-tool/profile matrix (default deny):

- `qdral.read` authorizes only tools and profiles explicitly marked read-only in the reviewed matrix;
- `qdral.write` additionally authorizes only explicitly marked write tools and profiles;
- `qdral.execute` additionally authorizes only explicitly marked execute tools and profiles;
- a tool that requires more than one scope needs every listed scope present in both the current token and the leased OAuth scope ceiling;
- missing or unknown scope/tool/profile mappings deny;
- the relay enforces this matrix before dispatch, and `qdrald` re-enforces it with the leased ceiling.

OAuth scopes are an outer remote ceiling only.

They do not:

- select or trust a workspace;
- enable a Qdral local tool-surface profile;
- grant SOFT or STRONG approval;
- widen an executable registry;
- bypass provider ceilings;
- allow a relay to create a capability token.

The local Qdral device must separately enable the relevant surface profile/workspace policy.

Effective permission is the intersection of:

```text
remote OAuth scope
AND active local remote-session lease, including its OAuth scope ceiling, workspace set, tool-surface profile, policy revision, device/connection binding, and expiry
AND locally enabled tool-surface profile
AND workspace policy
AND qdrald capability/provider ceiling
AND required local approval
```

Any denial wins. A valid remote OAuth token without an active local lease denies with typed `REMOTE_SESSION_INACTIVE`.

## 13. Provider authentication compatibility

### OpenAI

Production public plugin path uses OAuth 2.1. OpenAI's MCP client identity mechanisms (CIMD/private-key JWT and/or mTLS where supported) authenticate the OpenAI client as defense in depth; device-backed pairing authenticates/binds the Qdral user/device authorization transaction.

### Claude

Remote connector path should use the same OAuth 2.1 authorization server when the configured Claude connector supports it. Local Claude paths bypass remote OAuth and authenticate only to local Qdral transport.

### Mistral Vibe Work

Current Mistral Vibe Work custom connectors auto-detect OAuth 2.1 with dynamic client registration, bearer, basic, or no-auth. The shared Qdral public relay should prefer OAuth 2.1/DCR rather than weakening to a static bearer token merely for convenience.

### Local Mistral Vibe Code

Current Vibe Code does not support OAuth-required MCP servers, so it uses Qdral local stdio/loopback mode rather than the public OAuth relay.

## 14. Self-hosted mode

Self-hosted remote deployments run the same authorization contract.

A self-host operator may integrate an external identity provider, but the reference Qdral deployment must retain the device-backed principal path so self-hosting does not require a paid IdP.

Alternative authentication methods must not be accepted by the public OpenAI plugin endpoint unless they satisfy that provider's requirements.

## 15. Data minimization

The shared service must not require or collect by default:

- Windows username;
- PC hostname;
- hardware serials;
- MAC addresses;
- phone number;
- email address;
- social-login profile;
- workspace paths;
- file names/content;
- process output;
- clipboard content;
- screenshots;
- Windows Hello biometric/PIN material.

User-chosen device alias is optional and should be treated as user data.

## 16. Abuse controls

A passwordless/device-backed service still needs abuse protection.

Required:

- pairing attempt rate limits;
- OAuth transaction limits;
- token endpoint limits;
- concurrent connection limits;
- device registration limits;
- generic/public error shapes that avoid account/device enumeration;
- no payload logging for abuse analytics;
- deletion/revocation of stale route/auth metadata.

Abuse controls must not create hidden paid dependencies.

## 17. Qualification requirements

Before remote release, prove at minimum:

- PKCE success/failure and verifier mismatch;
- state/CSRF rejection;
- exact redirect rejection;
- issuer mix-up rejection;
- wrong resource/audience rejection;
- wrong/insufficient scope rejection;
- authorization-code replay rejection;
- access-token expiry;
- refresh replay/rotation behavior;
- connection/device revoke blocks renewal;
- pairing code entropy/expiry/one-shot/rate-limit behavior;
- device challenge replay rejection;
- cross-device pairing substitution rejection;
- lost/offline device cannot renew indefinitely;
- remote scope cannot enable a disabled local profile;
- remote connection cannot change workspace trust or executable registry;
- logs contain no tokens, pairing codes, device private keys, or MCP payload data.

## 18. Non-goals for v0.2

- mandatory Qdral cloud identity account;
- mandatory email/password login;
- social-login dependency;
- cross-device roaming under one live MCP connection;
- unattended remote approval;
- remote Windows Hello;
- cloud recovery of local Qdral secrets;
- enterprise SSO/SCIM;
- claims that provider login alone grants local authority.
