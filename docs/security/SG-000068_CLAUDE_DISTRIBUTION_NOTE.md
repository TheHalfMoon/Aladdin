# SG-000068 Claude Distribution Security Note

Status: IMPLEMENTATION FOR QDRAL-P17
SpecGrain: SG-000068
Base: `f4685f1755483b7b635f80472bd8b3f11dfd92ec`
Date: 2026-10-04

## Change

`apps/qdral-mcp/src/oauth_authorization.ts`:

- `isAcceptableRedirectUri` now accepts `http://localhost` redirect URIs in addition to `http://127.0.0.1` and
  `http://[::1]`, because Claude Code listens on `http://localhost:<port>/callback`. Issuer and resource identifiers
  still reject `localhost`.
- `redirectUriMatches` replaces exact list membership at the authorization endpoint. A request matches a registered
  redirect exactly, or, only when both are loopback `http` redirects, may differ in the port alone (RFC 8252 section 7.3).
  Scheme, hostname, path, and query must match exactly. The token endpoint still requires the exact redirect used at
  authorization.

## Threat analysis

- A remote attacker cannot receive a code through loopback matching: only the user's own machine listens on loopback,
  and hostnames other than `127.0.0.1`, `[::1]`, and `localhost` never match port-agnostically.
- `localhost` resolution is controlled by the local machine; a party able to rewrite it already controls that machine.
  RFC 8252 prefers IP literals, which remain supported.
- `https` redirects are never port-agnostic, so `https://claude.ai:8443/...` or any other host is refused.

## Evidence

- Unit tests: accepted and rejected loopback variants (port, path, query, scheme, host, IPv6, fragment, non-string).
- Relay end to end: a Claude web connector (`https://claude.ai/api/mcp/auth_callback`) and a Claude Code connector
  (registered `http://localhost/callback`, authorized and redeemed at `http://localhost:61264/callback`) both pair a
  device, discover exactly the `core` profile with contract security schemes, and are refused a local-only tool with
  403; six redirect variations are refused with 400 and no redirect.
- Local Claude Code configuration example pinned to the canonical stdio entrypoint.

## Authority delta

None. No tool, scope, profile, approval class, trust right, lease right, or actuation is added; Claude receives exactly
what every remote provider receives.
