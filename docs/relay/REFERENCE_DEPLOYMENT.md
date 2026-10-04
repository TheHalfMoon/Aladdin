# Qdral Relay Reference Deployment (Optional, Zero-Cost)

Status: REFERENCE DEFINITION (SG-000057). Not deployed or operated by the
project. Self-hosting (`SELF_HOSTING.md`) is the canonical zero-cost path.

## Policy

- A hosted relay is never required to use Qdral. Local clients use stdio or
  loopback HTTP, and remote clients can always use a self-hosted relay.
- A reference deployment may use only free tiers and must run in the relay's
  no-billing mode: quotas fail closed with `REMOTE_RATE_LIMITED`, and no
  autoscaling, paid add-on, or paid overflow is configured.
- Free-tier terms change. Before deploying, the operator must verify on the
  provider's current pages that the chosen tier is free and that billing
  cannot be triggered automatically. Some providers ask for a payment card
  for identity verification even on free tiers; Qdral never requires one, and
  such a provider should be skipped if that is unacceptable.
- No document in this repository may claim a hosted Qdral relay is live
  unless it has actually been deployed and verified.

## Shape

Any free host that can run one long-lived container (or Node.js process)
with a public HTTPS name works:

- the image from `apps/qdral-relay/Dockerfile`;
- one persistent volume for `stateDir` (the relay refuses to start on corrupt
  state, so ephemeral disks lose pairings on restart but never fail open);
- conservative quotas, for example the values in `relay.example.json`;
- TLS terminated by the host or by a free reverse proxy with a free
  certificate.

Hosts that put idle services to sleep still work: devices reconnect with
bounded backoff, and requests to an offline device fail closed with
`DEVICE_OFFLINE` rather than queuing.

## Verification checklist for an operator

1. `GET https://<host>/healthz` returns `{"ok":true}`.
2. `GET https://<host>/.well-known/oauth-protected-resource/mcp` names
   `https://<host>/mcp` and the issuer `https://<host>`.
3. `POST https://<host>/mcp` without a token returns 401 with a
   `WWW-Authenticate: Bearer resource_metadata=...` header.
4. Link a test computer with `qdral remote enable` and `qdral remote pair`,
   allow a lease, and call `tools/list` from an MCP client.
5. Revoke the lease and confirm the next call returns
   `REMOTE_SESSION_INACTIVE`.
6. Confirm the provider dashboard shows no billable usage.

The same checks run automatically against the container image in CI
(`Relay container / ubuntu-latest`) on loopback.
