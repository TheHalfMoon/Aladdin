# SG-000071 Cross-Provider Isolation Note

Status: QUALIFICATION FOR QDRAL-P17
SpecGrain: SG-000071
Base: `27d03f13f937ec976c98192a6a0e93bfa4d3da8d`
Date: 2026-10-04

## Topology

ChatGPT-style and Claude-style OAuth clients register separately on one relay and pair the same computer, producing two
remote connections on one device served by one outbound uplink. The P15 two-tenant suite
(`apps/qdral-relay/src/adversarial.test.ts`) continues to cover separate devices and principals.

## Proven negative cases

Relay, real uplink (`apps/qdral-relay/src/self-host.test.ts`):

- each provider's calls reach the device carrying only its own remote connection;
- a provider's access token on the other provider's MCP session is unknown (404);
- a provider's refresh token redeemed by the other provider's client fails with `client_mismatch`;
- a provider's authorization code redeemed by the other provider's client, redirect, and the original verifier fails;
- revoking one provider's connection stops that provider without dispatching anything to the device, while the other
  provider keeps working; only an explicit device revoke ends every provider on that computer.

qdrald lease gate (`crates/qdral-policy/src/remote_session.rs`, `cross_provider_contexts_never_share_a_lease`):

- with leases for two providers on one device, a context presenting one provider's connection with any other provider
  field (provider kind, client profile and revision, principal, device, device epoch, tool surface) is
  `REMOTE_SESSION_INACTIVE`, and one provider's identity on the other's connection is inactive;
- a connection pinned by one transport connection cannot be used by another;
- one provider's write ceiling never lends a scope to the other provider's read-only lease;
- revoking one provider's lease leaves the other active.

Workspace admission outside a lease remains covered by `workspace_scope_and_profile_ceilings_fail_closed`.

## Identical boundaries

Every known remote provider kind maps to the same `core` tool set; tool contract entries carry no provider-specific field;
approval class is a function of capability and operation only. No isolation defect was found, so no production code
changed.

## Authority delta

None.
