# Qdral OpenAI Plugin Package (SG-000067)

Status: SOFTWARE PACKAGE FOR QDRAL-P17. Provider-controlled states are recorded only in
`distribution/openai/review/distribution-state.json` from directly observed provider evidence.

## What the package is

A portable Agent Plugins package for the current OpenAI Plugins Directory:

- `plugin.json` with the `https://agent-plugins.org/schemas/1.0.0/plugin.schema.json` schema, product metadata, and
  OpenAI display metadata confined to `extensions["com.openai"].interface`;
- `mcp.json` declaring exactly one `streamable-http` server: the owner's Qdral relay `/mcp` endpoint;
- `assets/logo.svg` (original Qdral mark);
- `review/`: five positive and three negative review cases, release notes, the demo-recording evidence slot, the
  distribution state record, and `tool-scan.json` produced by in-process discovery through the authoritative MCP builder
  on the relay transport;
- `SHA256SUMS.txt` over every packaged file.

## Build

```sh
npm run build -w @qdral/mcp
node scripts/build-openai-plugin.mjs --mcp-url https://<your-relay-host>/mcp --out dist/openai-plugin
```

The relay endpoint is supplied at build time and never committed. The builder refuses non-https, credentialed, query-bearing,
loopback, and non-`/mcp` endpoints, and refuses to write a package that fails validation: the tool scan must equal the
remote `core` profile exactly, every annotation and `securitySchemes` entry must equal the tool contract, no local-only
tool may appear, OpenAI settings may live only under `com.openai`, assets must be packaged `./assets/` paths, and no file
may match a secret pattern.

## Relay configuration for review

- Serve the relay over HTTPS on a host you control (see `docs/relay/SELF_HOSTING.md`).
- Set `openaiAppsChallenge` in the relay configuration to the token OpenAI issues; the relay then answers
  `GET /.well-known/openai-apps-challenge` with exactly that token as `text/plain`. Without it the path returns 404. The
  token proves domain control to OpenAI only and grants no workspace trust, approval, executable admission, device
  enrollment, or remote-session authority.
- ChatGPT registers through dynamic client registration; register its redirect URI
  `https://chatgpt.com/connector_platform_oauth_redirect` (issuer identification is supported) as shown on the OpenAI
  management page. Client ID metadata documents are not implemented and are not advertised.

## Authority

Nothing in the package or the challenge path is authority. Each call still needs an OAuth token with the tool's exact
scope and audience, a paired device, an active remote-session lease started locally, workspace trust, and local approval
for every change. Desktop, clipboard, and device-side fetch tools are local-only and never reach the package.

## External prerequisites (owner actions)

Recorded as SUBMISSION_READY blockers until observed: a deployed public HTTPS relay host, the OpenAI-issued domain
challenge token, owner review and publication of `docs/legal/PRIVACY.md` and `docs/legal/TERMS.md`, the demo recording,
and a reviewer test account with a paired computer. SUBMITTED, APPROVED, PUBLISHED, and ACCOUNT_VERIFIED are provider
states that no software or CI result can establish.

## Requirements re-verification

Checked against official OpenAI documentation on 2026-10-04:
`https://developers.openai.com/plugins/build/plugins/`, `https://developers.openai.com/plugins/deploy/submission`,
`https://developers.openai.com/plugins/build/auth`, `https://developers.openai.com/plugins/plugin-guidelines`. They must be
re-checked again before any submission.
