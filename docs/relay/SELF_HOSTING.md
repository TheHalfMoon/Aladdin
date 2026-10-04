# Self-Hosting the Deskal Relay

Status: OPERATOR GUIDE (SG-000057)

The Deskal relay is open source (Apache-2.0) and self-hosting is the canonical,
zero-cost way to give a hosted AI client remote access to your own computer.
Nothing in this guide requires a paid API, paid cloud, paid database, paid
identity provider, or paid relay.

## What the relay does and does not do

- It terminates the public MCP endpoint (`/mcp`), runs the OAuth 2.1
  authorization server for device-backed pairing, and holds the outbound-only
  channel that your computer opens to it.
- Your computer never opens an inbound port. `qdral remote connect` only makes
  outbound HTTPS requests to the relay.
- The relay sees MCP requests and results in transit while it forwards them.
  It never stores payloads, and logs carry route and status classes only.
- The relay cannot grant anything on your computer. Every remote call still
  needs an active local remote-session lease (`qdral remote allow`, Windows
  Hello, at most 15 minutes) plus the normal workspace policy and approvals.
- When a quota is exhausted the relay fails closed with
  `REMOTE_RATE_LIMITED`. It never scales out or spends money.

## Requirements

- A machine that can accept HTTPS from your AI provider: a home server, a
  small VM, or any host you control.
- A public HTTPS name and certificate. Free options include your own domain or
  a free dynamic-DNS name with a free Let's Encrypt certificate, for example
  through Caddy's automatic HTTPS.
- Node.js 24 or Docker.

## Run with Docker

```sh
docker build -f apps/qdral-relay/Dockerfile -t qdral-relay .
mkdir -p /srv/qdral-relay/config /srv/qdral-relay/state
cp apps/qdral-relay/relay.example.json /srv/qdral-relay/config/relay.json
# edit publicOrigin to your HTTPS origin; keep listenHost 127.0.0.1 behind a proxy,
# or set it to 0.0.0.0 inside the container and publish only to 127.0.0.1.
docker run -d --name qdral-relay --restart unless-stopped \
  -p 127.0.0.1:8787:8787 \
  -v /srv/qdral-relay/config:/etc/qdral-relay:ro \
  -v /srv/qdral-relay/state:/var/lib/qdral-relay \
  qdral-relay
```

The image is built from a digest-pinned `node:24.19.0-alpine3.23` base and
runs as the unprivileged `node` user.

## Run with Node.js

```sh
npm ci --ignore-scripts
npm run build
node apps/qdral-relay/dist/main.js --config /etc/qdral-relay/relay.json
```

## TLS termination (example: Caddy)

```
relay.example.com {
    reverse_proxy 127.0.0.1:8787
}
```

Caddy obtains and renews a free certificate automatically. The relay checks
that the `Host` header equals the host of `publicOrigin`, so the proxy must
preserve the original host (Caddy does by default).

## Configuration

`relay.json` is strict: unknown fields are rejected.

| Field | Meaning |
| --- | --- |
| `publicOrigin` | Public HTTPS origin, no path. Also the OAuth issuer and the base of the `/mcp` resource. |
| `listenHost`, `listenPort` | Listener (default `127.0.0.1:8787`). |
| `stateDir` | Durable state directory (owner-only files). |
| `allowedOrigins` | Browser origins allowed to call `/mcp`; default none. |
| `openaiAppsChallenge` | Optional OpenAI domain-verification token (8 to 512 URL-safe characters) served as plain text at `/.well-known/openai-apps-challenge`; grants no authority (SG-000067). |
| `quotas` | Optional limits; each value is bounded by a hard ceiling. |

Quotas (defaults / hard ceilings): `maxDevices` 1000 / 100000,
`maxClients` 1000 / 100000, `registrationsPerHour` 60 / 10000,
`authorizeAttemptsPerMinute` 30 / 600, `tokenRequestsPerMinute` 120 / 6000,
`mcpRequestsPerMinutePerRoute` 60 / 60, `dailyRequestBudget` 100000 /
10000000. There is no setting that enables paid overflow.

## State

`stateDir/relay-state.json` holds device public keys, client registrations,
route identifiers, refresh-family hashes, revocations, and the relay's own
token signing key. It is written atomically with an integrity checksum. If it
is corrupt or tampered with, the relay refuses to start instead of forgetting
revocations. Back it up like any secret: anyone with the file can mint tokens
for this relay (they still cannot act on a computer without a local lease).
The canonical backup/restore procedure, signing-key compromise response,
incident runbook, and recovery objectives are in
`docs/relay/OPERATOR_RECOVERY.md`.

## Linking a computer and an AI client

1. On the computer: `qdral remote enable --relay https://relay.example.com`
   (Windows Hello).
2. In the AI client, add a custom MCP connector with the URL
   `https://relay.example.com/mcp`. The client discovers the authorization
   server, registers, and opens the authorization page.
3. On the computer: `qdral remote pair` (Windows Hello). Enter the code it
   prints on the authorization page, then type `yes` on the computer after
   checking the client name, return address, and scopes.
4. On the computer: `qdral remote connect` (keep it running) and
   `qdral remote allow --connection <rc-...>` whenever you want to allow a
   finite remote session.

Revoke at any time with `qdral remote revoke`, or everything at once with
`qdral emergency-revoke`.

## Health

`GET /healthz` returns `{"ok":true}` without any detail.
