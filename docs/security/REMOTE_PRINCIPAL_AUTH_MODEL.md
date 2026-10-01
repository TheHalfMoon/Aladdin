# Cotra Remote Principal and Authorization Model

Status: IMPLEMENTATION-READY PROPOSAL
Date: 2026-10-01
Planning base: `5feff3f15cc7e20464cafffd7b87d713b65a012f`
Companions:
- `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md`
- `docs/security/UNIVERSAL_CONNECTIVITY_THREAT_MODEL.md`
- `docs/research/UNIVERSAL_CLIENT_COMPATIBILITY.md`

## 1. Problem

Hosted AI clients need a stable authenticated principal before Cotra can route a remote MCP request to one user's computer.

Cotra must provide that identity without making a paid identity provider, email/password database, social login, or founder-funded SaaS subscription mandatory.

At the same time, OpenAI public plugins and standards-compliant remote MCP clients need OAuth 2.1 semantics for private data and write actions.

This document defines the default Cotra solution.

## 2. Decision: device-backed remote principal

The v0.2 default is a **device-backed remote principal**.

The local Cotra installation is the root of user possession for initial remote linking. A hosted provider connection is paired to exactly one Cotra device by an explicit local flow.

No mandatory Cotra email account, password, phone number, social login, or third-party identity subscription is required.

The relay/auth service stores only opaque principal/device/connection identifiers, device public keys, OAuth client/token state, route/revocation epochs, and the minimum abuse/rate-limit metadata required to operate the service.

The device private key remains local.

## 3. Identity objects

### 3.1 `device_id`

A random stable identifier for one Cotra installation. It is not derived from hardware serials, Windows username, MAC address, machine SID, hostname, or other fingerprinting data.

### 3.2 device key pair

Generated locally during remote setup and stored in Cotra protected state.

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

**v0.2 deliberately binds one remote provider connection to one device.**

A user who wants to connect a second PC creates a second provider connection. This avoids ambiguous device selection and cross-device routing in the first universal release.

### 3.5 `client_session_id`

Generated locally by Cotra for each live MCP session. It remains the session identifier used by local audit/policy and is not replaced by provider or relay session IDs.

## 4. First-time remote setup

The user explicitly runs a local command such as:

```text
cotra remote enable
```

This operation:

1. creates the device key pair if absent;
2. creates the opaque `device_id`;
3. stores private material under existing Cotra protected-state rules;
4. registers only the device public key and minimal route metadata with the selected relay;
5. requires STRONG local user presence before the device is eligible for remote pairing;
6. does not grant any workspace, capability, tool profile, or action approval.

`remote enable` is a PRIVILEGED local lifecycle action and is never exposed as an MCP tool.

## 5. Provider linking / OAuth user authentication

The public Cotra authorization server uses OAuth 2.1 authorization-code + PKCE for supported hosted MCP clients.

The user authenticates the OAuth authorization transaction through **local device pairing**, not a cloud password.

### 5.1 Linking flow

1. The hosted AI client discovers Cotra's protected-resource and authorization-server metadata.
2. The hosted client begins authorization-code + PKCE with the exact Cotra MCP resource.
3. The Cotra authorization page asks the user to pair a Cotra device. It does not ask for a Cotra password.
4. On the PC, the user runs:

   ```text
   cotra remote pair
   ```

5. Cotra requires STRONG local user presence and creates a short-lived one-time pairing transaction.
6. The user either:
   - opens a device-generated HTTPS pairing URL/QR token; or
   - enters a high-entropy one-time code into the authorization page.
7. The relay/auth service sends a fresh challenge to the candidate device.
8. The device signs the challenge and confirms the exact remote OAuth client/provider, requested scopes, and target device alias locally.
9. The server creates/binds `remote_principal_id` and `remote_connection_id` only after successful device proof and local confirmation.
10. The authorization page shows the requested Cotra scopes and the selected device.
11. The authorization server issues a short-lived authorization code bound to client, redirect URI, PKCE challenge, resource, principal, device, scopes, and transaction.
12. The provider exchanges it for scoped tokens.

Pairing proves possession/control of the local Cotra installation. It still does not grant workspace trust or approve tool effects.

## 6. Pairing token requirements

A pairing transaction must be:

- cryptographically random;
- at least 80 bits of effective entropy for any manually entered representation;
- preferably represented as an opaque URL token with at least 128 bits of entropy for QR/click flows;
- single use;
- short lived;
- bound to the candidate `device_id` and OAuth authorization transaction;
- rate limited by transaction, source, and device as appropriate;
- invalidated on success, expiry, revoke, or excessive failed attempts;
- excluded from application logs and metrics.

The authorization page must not reveal whether an arbitrary guessed device/principal identifier exists.

## 7. OAuth server requirements

The Cotra authorization server is security-critical.

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

Cotra minimizes the consequences of losing a device without requiring a mandatory cloud account recovery system.

Default policy:

- access tokens are short lived;
- refresh/token renewal is allowed only while the bound device connection is still valid under the current device/connection revocation epoch;
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
- revoke all remote connections for the device;
- rotate the device key;
- disable remote mode entirely;
- emergency revoke through the existing Cotra authority boundary.

Revocation increments the relevant epoch and invalidates:

- active relay routes;
- outstanding authorization transactions;
- access/refresh token families as applicable;
- device reconnect authorization for the revoked identity.

Provider-side disconnect is useful but is not the sole Cotra revocation mechanism.

## 10. Lost-device case

The default v0.2 system intentionally avoids pretending that an email/password cloud account exists when it does not.

If the only Cotra device is permanently lost:

- the short access-token lifetime limits residual access;
- device-gated renewal fails when the device cannot prove possession/current epoch;
- a replacement installation receives a new `device_id` and key pair;
- the user links the replacement as a new connection;
- provider-side connector removal remains an additional cleanup path.

An optional recovery credential or passkey-backed multi-device Cotra account may be designed later, but it is not required for the initial universal release and must not be silently introduced as a new cloud dependency.

## 11. Multiple devices

v0.2 does not route one provider connection dynamically among multiple computers.

Each connection is one device.

If a provider supports multiple connected accounts/connections, Cotra may expose a minimal authenticated profile identity so the user can distinguish connections, for example a user-chosen device alias. This profile must reveal no hostname, username, IP address, machine SID, or hardware fingerprint by default.

For OpenAI, any future profile tool must follow the platform's authenticated profile-tool convention and be independently reviewed as a tool-surface change.

## 12. Scopes and local profiles are independent

Initial remote OAuth scopes:

- `cotra.read`
- `cotra.write`
- `cotra.execute`

OAuth scopes are an outer remote ceiling only.

They do not:

- select or trust a workspace;
- enable a Cotra local tool-surface profile;
- grant SOFT or STRONG approval;
- widen an executable registry;
- bypass provider ceilings;
- allow a relay to create a capability token.

The local Cotra device must separately enable the relevant surface profile/workspace policy.

Effective permission is the intersection of:

```text
remote OAuth scope
AND locally enabled tool-surface profile
AND workspace policy
AND cotrad capability/provider ceiling
AND required local approval
```

Any denial wins.

## 13. Provider authentication compatibility

### OpenAI

Production public plugin path uses OAuth 2.1. OpenAI's MCP client identity mechanisms (CIMD/private-key JWT and/or mTLS where supported) authenticate the OpenAI client as defense in depth; device-backed pairing authenticates/binds the Cotra user/device authorization transaction.

### Claude

Remote connector path should use the same OAuth 2.1 authorization server when the configured Claude connector supports it. Local Claude paths bypass remote OAuth and authenticate only to local Cotra transport.

### Mistral Work

Current Mistral Work custom connectors auto-detect OAuth 2.1 with dynamic client registration, bearer, basic, or no-auth. The shared Cotra public relay should prefer OAuth 2.1/DCR rather than weakening to a static bearer token merely for convenience.

### Local Mistral Vibe Code

Current Vibe Code does not support OAuth-required MCP servers, so it uses Cotra local stdio/loopback mode rather than the public OAuth relay.

## 14. Self-hosted mode

Self-hosted remote deployments run the same authorization contract.

A self-host operator may integrate an external identity provider, but the reference Cotra deployment must retain the device-backed principal path so self-hosting does not require a paid IdP.

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

- mandatory Cotra cloud identity account;
- mandatory email/password login;
- social-login dependency;
- cross-device roaming under one live MCP connection;
- unattended remote approval;
- remote Windows Hello;
- cloud recovery of local Cotra secrets;
- enterprise SSO/SCIM;
- claims that provider login alone grants local authority.
