# Cotra Client Connection Profile Model

Status: IMPLEMENTATION-READY PROPOSAL
Date: 2026-10-01
Planning base: `5feff3f15cc7e20464cafffd7b87d713b65a012f`
Companions:
- `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md`
- `docs/security/REMOTE_PRINCIPAL_AUTH_MODEL.md`
- `docs/security/REMOTE_SESSION_AUTHORIZATION.md`

## 1. Problem

Provider-neutral transport is not sufficient for least privilege.

If every local or remote MCP client receives the same Cotra tool surface and workspace access, adding multiple clients silently creates authority equivalence between ChatGPT, Claude, Mistral, Codex, IDEs, and generic MCP applications.

Cotra therefore needs a local **client connection profile** that is independent of provider OAuth scopes and independent of the transport implementation.

## 2. Security role

A client connection profile is a local policy object that identifies one configured Cotra integration and places a ceiling on what that integration may request.

It is not an approval token and is not proof that the AI host is trustworthy.

Effective authority is the intersection of:

```text
client connection profile
AND transport-specific authentication / remote OAuth scope
AND locally enabled tool-surface profile
AND workspace policy
AND cotrad capability/provider ceiling
AND remote-session lease when transport is remote
AND required per-action approval
```

Any denial wins.

## 3. Profile identity

Each profile has:

- random opaque `client_profile_id`;
- user-chosen display alias;
- expected `provider_kind` or `generic`;
- allowed transport kinds (`stdio`, `loopback_http`, `relay`);
- allowed tool-surface profile ceiling;
- allowed workspace IDs or an explicit no-workspace state;
- read authorization mode;
- optional per-workspace read mode override;
- allowed remote OAuth scope ceiling when remote;
- creation/update revision;
- enabled/disabled state;
- revocation epoch;
- audit label that does not include secrets.

The alias is never authority.

## 4. Local profile management

Profile creation, widening, deletion, or provider/transport rebinding is a local security operation.

A command family may resemble:

```text
cotra client add
cotra client show <alias>
cotra client list
cotra client disable <alias>
cotra client revoke <alias>
cotra client remove <alias>
```

Widening a profile requires STRONG local user presence.

No MCP tool may create, modify, enable, or widen a client profile.

Removing access may use a lower-friction local path when it is strictly authority-reducing, while still recording audit evidence.

## 5. Local stdio mode

A configured local integration launches Cotra for one exact client profile, for example conceptually:

```text
cotra mcp stdio --client-profile <opaque-or-local-alias>
```

The implementation should avoid placing bearer secrets directly in command-line arguments.

The profile ID itself is not a secret.

The stdio profile selector is caller-controlled, so the implementation must not trust an ID or alias alone. stdio must bind the selected profile to a protected per-profile launch credential, manifest, or OS-managed identity, and a malicious MCP host must not be able to launch Cotra under a higher-authority profile by supplying a different selector.

The direct stdio process relationship supplies the transport path, while the local profile supplies the policy ceiling and audit identity.

Cotra does not claim cryptographic isolation from arbitrary malicious code already running unrestricted under the same Windows user. A same-user attacker may be able to inspect process configuration or invoke installed executables. Cotra's protected-state, workspace, capability, and approval controls remain the security boundary against effects, but confidentiality from an already-unrestricted same-user process is outside the strict model.

This limitation must be documented honestly.

## 6. Loopback HTTP mode

Loopback mode requires both:

- one exact client profile; and
- a protected local listener credential/session secret.

Requirements:

- credential generated locally;
- stored only in protected local state and client configuration location appropriate to the integration;
- not passed in URL query strings;
- not logged;
- rotatable/revocable independently of the profile;
- bound to the profile and listener instance/epoch;
- Host and Origin validation still required;
- bearer possession does not bypass workspace/profile/approval policy.

## 7. Remote relay mode

A remote provider connection is bound to one client connection profile through explicit local configuration.

The relay cannot select or widen the local profile.

The local remote-session lease further narrows the profile to exact workspaces/surface/duration for the active remote session.

Provider OAuth scopes are an outer remote ceiling and may be narrower than the profile. They can never widen the profile.

Changing the profile bound to an existing remote connection requires local authorization and invalidates the old remote-session lease/token epoch as appropriate.

## 8. Workspace behavior

A client profile must not automatically inherit every workspace registered with Cotra.

New workspaces default to **not admitted** to existing client profiles.

Adding a workspace to a client profile is an explicit local action.

Workspace trust and client-profile admission are separate:

- workspace trust answers whether Cotra may perform the relevant operation in that workspace;
- client-profile admission answers whether this configured AI integration may request that workspace at all.

Both must allow the request.

## 9. Tool-surface behavior

The profile references an explicit surface ceiling such as:

- `core`;
- `developer`;
- `desktop_structured`;
- `coordinate_fallback`.

A profile may use a narrower generated manifest than the maximum surface profile.

Clients should ideally see only tools they are allowed to call, but server-side `cotrad` enforcement remains mandatory because discovery hiding is not authorization.

A transport adapter cannot register extra tools outside the single reviewed registry.

## 10. Provider kind

`provider_kind` is useful for audit, compatibility, and provider-specific response shaping, but it is not sufficient proof of provider identity.

For remote mode, provider/client identity is derived from the authenticated OAuth/client path and must match the profile's allowed provider kind.

For local mode, the provider kind is configured by the user and remains metadata/policy context. Cotra must not claim it cryptographically proves that a local process is Claude, Codex, Mistral, or another named host.

## 11. Cross-provider isolation

Tests must prove:

- profile A cannot request profile B's workspace;
- a remote connection bound to profile A cannot select profile B in tool arguments;
- loopback credential A cannot authenticate as profile B;
- revoking profile A does not revoke unrelated profile B unless a broader emergency revoke is used;
- provider A OAuth scopes/tokens cannot bind to a profile restricted to provider B;
- tool manifests cannot drift between profile discovery and kernel enforcement;
- audit records identify the correct profile without logging credentials.

## 12. Default profiles and onboarding

Cotra may generate a minimal profile during an integration setup assistant, but defaults must be least privilege.

Recommended defaults:

- no workspace admitted until the user selects one;
- `core` surface unless the integration workflow explicitly needs more;
- no remote connectivity unless explicitly enabled;
- remote read mode `session` only after an active local remote-session lease;
- no developer executable registry widening during client setup;
- no browser/UI/clipboard/network exposure merely because a provider supports those tools.

## 13. Lifecycle and update

Profiles live in protected Cotra configuration and participate in policy revision/evidence.

Updates must preserve them without silently widening them.

If a new Cotra release adds tools to a named surface profile, existing client profiles must not silently inherit new authority unless the profile/versioning design explicitly proves that behavior safe. Prefer versioned surface manifests or an explicit local migration/approval for widening.

Rollback must not interpret a newer unknown profile field as allow.

Unknown/corrupt profile state fails closed.

## 14. Audit

Record, in bounded redacted form:

- profile create/change/disable/revoke/remove;
- workspace admission/removal;
- surface ceiling change;
- transport/provider binding change;
- listener credential rotation event, never the credential;
- remote-connection binding event;
- profile revision used for each request.

## 15. Qualification requirements

Before P14 exit:

- at least three independent local clients use separate profiles;
- no profile inherits a newly added workspace automatically;
- stdio profile selection cannot be overridden by MCP tool arguments;
- loopback credentials are profile-bound and revocable;
- disabled/revoked profile fails closed;
- tool discovery and kernel policy agree on the profile ceiling;
- profile change invalidates stale policy/session bindings;
- rollback/unknown profile state fails closed;
- logs do not contain listener credentials.

Before P17 exit:

- local and remote provider connections have separate profiles;
- cross-provider profile substitution tests pass;
- provider authentication metadata cannot widen local profile authority.

## 16. Non-goals

- cryptographic attestation that a local same-user process is a branded AI client;
- protection against arbitrary unrestricted same-user malware;
- remote profile administration;
- provider-managed workspace trust;
- implicit access to every Cotra workspace;
- one global profile shared by all AI integrations.
