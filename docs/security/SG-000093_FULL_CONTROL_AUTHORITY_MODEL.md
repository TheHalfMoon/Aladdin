# SG-000093 Full-Control Authority and Profile Model

Status: IMPLEMENTATION STAGING
Program: DESKAL-P20
Grain: SG-000093
Canonical activation base: `c930bf57651e83654a42a46d88c5ddbff895cab2`

## Purpose

SG-000093 introduces the Deskal-owned authority model required before any P20 donor executor can be exposed.

This grain does **not** add desktop control, shell/process widening, expanded filesystem access, browser/web execution, a privileged broker, persistent administrator execution, or remote full-control execution. It defines only the local authority profiles and lease checks that future grains must consume.

## Authority modes

The exact mode vocabulary is:

- `Safe`
- `Full User`
- `Full Admin`
- `Persistent Admin`
- `Remote Full Control`

`Safe` is the sole default.

Only the authenticated local user control path can select or issue `Full User` or `Full Admin` authority.

`Persistent Admin` remains unavailable until SG-000099.

`Remote Full Control` remains unavailable until SG-000105.

No agent, MCP caller, donor runtime, relay, or remote principal can mint, widen, renew, persist, or convert authority.

## FullControlLease

A `FullControlLease` is bound to all of the following:

- exact Windows user SID;
- exact interactive logon session;
- exact device identity;
- exact Deskal session identity;
- exact authority profile;
- exact policy revision;
- exact authority epoch;
- issue time and bounded expiry.

Any mismatch fails closed.

Restart or Deskal-session replacement invalidates the old lease because the session identity no longer matches.

A policy revision change invalidates the old lease.

Emergency revoke advances the authority epoch so all prior leases fail before a new mutation can dispatch.

## Full Admin

`Full Admin` is a two-stage authority.

1. The local user selects `Full Admin` intent and receives a valid `FullControlLease`.
2. Administrative execution additionally requires a separate `AdminLease` bound to a legitimate Windows elevated-administrator authority proof.

SG-000093 does not acquire that elevated token and does not host an elevation broker.

Accepted elevation state:

- `ElevatedAdministrator`

Explicitly rejected:

- standard unelevated token;
- `SYSTEM`;
- `TrustedInstaller`;
- UAC bypass;
- Windows Hello bypass;
- secure-desktop automation;
- credential-provider bypass.

The admin lease cannot outlive its parent full-control lease and must match the same user, logon session, device, Deskal session, policy revision, and authority epoch.

## Future executor ceilings

The mode model freezes the maximum future executor classes without exposing them in this grain.

`Full User` may eventually authorize:

- desktop observation;
- desktop input;
- shell/process sessions;
- filesystem;
- browser;
- local web;
- visual-agent execution.

`Full Admin` may eventually authorize the same classes plus the privileged admin executor.

No current MCP tool or production executor is widened by SG-000093.

## Failure semantics

All authority checks are fail closed for:

- stale or expired lease;
- revoked lease;
- wrong Windows user;
- wrong logon session;
- wrong device;
- wrong Deskal session;
- policy drift;
- authority-epoch drift;
- mismatched elevation proof;
- unsupported authority mode;
- non-local grant origin.

The existing mutation-state vocabulary remains exact:

- `not_started`
- `dispatched`
- `completed`
- `cancelled`
- `outcome_unknown`

A lease failure after dispatch does not imply retry. Broader-authority fallback is never automatic.

## Audit privacy

Authority audit projection stores:

- event kind;
- opaque lease id;
- authority mode;
- SHA-256 binding digest;
- authority epoch;
- event time.

It does not store the raw Windows SID, raw device/session binding values, elevated token handles, credential material, or the raw elevation-proof id.

## Successor boundary

SG-000094 may begin only after SG-000093 closes canonically.

SG-000094 may import or adapt the pinned Open Computer Use Windows host behind private Deskal IPC. Desktop input remains owned by SG-000095.

No donor import is authorized by SG-000093.
