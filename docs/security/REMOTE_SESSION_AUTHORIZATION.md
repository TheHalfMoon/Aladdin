# Cotra Remote Session Authorization

Status: IMPLEMENTATION-READY PROPOSAL
Date: 2026-10-01
Planning base: `5feff3f15cc7e20464cafffd7b87d713b65a012f`
Companions:
- `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md`
- `docs/security/REMOTE_PRINCIPAL_AUTH_MODEL.md`
- `docs/security/UNIVERSAL_CONNECTIVITY_THREAT_MODEL.md`
- `docs/security/CLIENT_CONNECTION_PROFILE_MODEL.md`

## 1. Why this boundary is required

A shared relay terminates the hosted provider's HTTPS connection and necessarily sees remote MCP request/result plaintext transiently.

Even with correct OAuth and device pairing, a compromised relay process that receives a valid remote connection can attempt additional MCP calls while that connection is valid.

Per-action SOFT/STRONG approval limits write/execute authority, but many useful read-only Cotra tools intentionally require no per-call approval in local mode. Without another local boundary, a compromised remote relay could invoke those read tools and exfiltrate data from an already-admitted workspace.

Therefore remote connectivity requires a **local remote-session authorization lease** in addition to remote OAuth, pairing, workspace policy, and per-action approval.

Cotra must not claim that relay compromise is harmless. The goal is to bound the period, workspace, profile, and remote connection for which remote requests are locally dispatchable.

## 2. Remote session lease

A remote session lease is a local `cotrad` authorization object created only through local user interaction.

It binds at minimum:

- exact stable `remote_connection_id` and its current short-lived `connection_id`;
- exact `device_id`;
- authenticated `provider_kind` / remote client identity metadata;
- allowed remote OAuth scope ceiling;
- locally enabled tool-surface profile;
- allowed workspace IDs;
- current local policy revision;
- lease creation time;
- hard expiry;
- local interruption/revocation epoch;
- remote connection/revocation epoch.

The lease contains no provider bearer token and no relay secret.

The relay cannot create, extend, widen, or renew it.

## 3. Required decision order

For every remote MCP tool call, effective authorization is:

```text
valid remote OAuth/client authentication with current token scopes, where required tool scopes are within both the current token scopes and the leased OAuth scope ceiling
AND exact paired remote connection/device route
AND active local remote-session lease
AND tool included in the leased local surface profile
AND workspace included in the lease
AND current policy revision still matches or locally controlled revalidation succeeds through a local presence-based step the remote model cannot satisfy
AND normal cotrad capability/provider-ceiling checks
AND required per-action SOFT/STRONG approval
```

Any denial wins.

A valid remote OAuth token without a local lease produces a typed local denial such as `REMOTE_SESSION_INACTIVE`; it does not create a prompt that the remote model can satisfy. `REMOTE_SESSION_INACTIVE` is part of the shared typed-failure vocabulary in `docs/canonical/UNIVERSAL_AI_ACCESS_PLAN.md` section 18.

## 4. Lease creation UX

A command/UX such as:

```text
cotra remote allow <connection>
```

shows locally:

- provider/client identity known to Cotra;
- user-chosen device/connection alias;
- requested remote scopes;
- local tool-surface profile;
- workspace list;
- lease duration;
- whether read-only calls are session-authorized or individually prompted.

Creating or widening a remote session lease is a local privileged security decision.

Initial remote enable/pair and any persistent/default-allow policy require STRONG local user presence.

A normal bounded lease creation or widening requires STRONG local user presence, and the remote model cannot satisfy it.

## 5. Duration

Remote leases are finite by default.

Requirements:

- hard maximum duration is 15 minutes unless a separately reviewed release policy sets a shorter limit;
- reconnect does not extend expiry;
- provider token refresh does not extend expiry;
- relay restart does not extend expiry;
- device restart invalidates or requires explicit safe restoration according to the proven lifecycle design;
- expired leases are not silently recreated;
- no remote API can request `forever`.

A future locally configured persistent policy may create new finite leases automatically only if separately authorized, STRONG-gated, visible in local status, and revocable. It is not part of the initial P15 target.

## 6. Workstation state

Default remote policy is interactive-user-first.

The remote session lease follows this deterministic state machine on every observed workstation transition:

- user logoff: lease invalidated immediately; in-flight remote calls fail closed; reconnect requires a new STRONG-gated lease;
- workstation lock: dispatch suspended immediately and in-flight calls fail closed; unlock alone does not resume dispatch without an active unexpired lease, and never extends expiry;
- suspend/hibernate: lease suspended on suspend; on resume the lease remains expired if its deadline passed, otherwise dispatch stays suspended until Cotra revalidates device, connection, and policy epochs locally;
- Cotra shutdown, emergency revoke, or device key/connection revoke: lease invalidated immediately with no retry window that outlives the original request/lease expiry.

Unlock/resume does not silently widen or extend an expired lease. Race handling is fail closed: if a transition and a dispatch race, the transition wins.

Exact Windows behavior must be qualified on real Windows; CI must not fabricate workstation-state evidence.

## 7. Read authorization modes

A workspace/remote connection can choose among locally configured read modes.

The effective read mode is the most restrictive of the client profile, workspace override, and lease: `disabled` denies, and `per_request` cannot be downgraded to `session`.

### `session`

Default target. Read-only tools that already require no local approval within the lease's exact workspace/profile may run during the active lease without an approval dialog for every call; read tools with an existing approval class retain it.

### `per_request`

Every remote read requires a fresh local SOFT approval in addition to the active remote-session lease. Intended for sensitive workspaces.

### `disabled`

Remote reads are denied even when the remote connection is otherwise paired.

The remote provider cannot change this mode.

## 8. Write and execute behavior

A remote session lease is **not** an action approval.

File writes, Git mutation, process execution, UI actuation, clipboard changes, network operations, and other effects retain their existing per-action approval classes and exact digest/state binding.

The lease cannot turn a SOFT or STRONG operation into a no-approval operation.

STRONG operations remain locally mediated and cannot be satisfied remotely.

## 9. No offline surprise execution

When a device is offline, lease-suspended, locked, or expired:

- mutating calls are not durably queued for later execution;
- read calls are not durably queued for later disclosure;
- the remote client receives a typed unavailable/session-inactive result;
- a short transport retry window may exist only before local dispatch and cannot outlive the original request/lease expiry.

When the user later restores a lease, old calls do not replay.

## 10. Relay compromise residual risk

This boundary reduces but does not eliminate the consequences of a shared relay compromise.

During an active `session` read lease, a compromised relay that can impersonate the valid remote connection may attempt additional read calls within the exact leased workspace/profile and may observe their results.

Mitigations are:

- finite local lease;
- one-device connection binding;
- narrow workspace/profile scope;
- optional `per_request` read approval;
- OAuth scope ceiling;
- immediate local revoke/emergency stop;
- no relay ability to widen/renew the lease;
- no-action approval bypass;
- data-minimized tool outputs;
- no durable relay payload logs.

Users who require stronger trust can use local stdio/loopback mode or self-host the relay.

Cotra documentation and marketing must state this residual risk rather than claim the shared relay is outside the confidentiality boundary.

## 11. Audit

Local audit records should include:

- remote connection ID in opaque/redacted form;
- provider kind;
- lease ID;
- workspace/profile ceiling;
- lease create/expire/revoke event;
- read authorization mode;
- whether the call was allowed by session lease or per-request approval;
- normal request/effect evidence.

Do not record raw OAuth tokens, pairing codes, device private keys, provider prompts, or unrelated relay metadata.

## 12. Revocation

The following invalidate the lease immediately:

- local connection revoke;
- remote-mode disable;
- emergency revoke;
- device key rotation when the connection is bound to the prior key;
- workspace trust/policy change that invalidates the binding;
- local tool-profile removal;
- lease expiry;
- relevant interruption/security epoch change.

Revocation is checked immediately before provider dispatch, not only when the remote request first reaches Cotra.

## 13. Threats addressed

This boundary specifically limits:

- compromised relay impersonation;
- stolen but still valid remote access token use;
- unattended read exfiltration after the user leaves the workstation;
- reconnect-induced authorization persistence;
- remote scope/profile drift;
- offline queued disclosure;
- stale workspace/policy authorization.

It does not replace OAuth, pairing, provider authentication, local workspace policy, action approval, redaction, or provider ceilings.

## 14. Qualification requirements

Before P15 exit, prove at minimum:

- no remote request dispatches without an active exact-connection lease;
- relay/provider cannot create or extend a lease;
- lease expiry fails closed;
- reconnect/refresh does not extend expiry;
- workspace/profile/policy drift invalidates the lease or forces safe revalidation;
- emergency revoke invalidates it;
- device/connection revoke invalidates it;
- `per_request` read mode actually requires local approval;
- `disabled` read mode denies reads;
- writes/executes still require their original action approval after lease authorization;
- stale/offline requests are not replayed after a new lease is created;
- cross-provider and cross-device lease substitution fails;
- Windows lock/logoff/suspend behavior matches the documented state machine on real Windows.

## 15. Non-goals

- remote approval delegation;
- unattended permanent read authority by default;
- relay-created local leases;
- automatic widening based on provider identity;
- replacing workspace trust;
- replacing per-action approvals;
- claiming shared-relay confidentiality during an active session lease.
