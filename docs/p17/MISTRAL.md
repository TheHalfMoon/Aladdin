# Qdral with Mistral Vibe (SG-000069)

Status: SOFTWARE QUALIFICATION FOR QDRAL-P17. Directory and account states are recorded only in
`distribution/mistral/distribution-state.json` from directly observed Mistral evidence.

## Vibe Code CLI (local)

Use `examples/mcp-clients/vibe-code-config.toml` in Vibe Code's `config.toml`:

- `stdio`: `command = "qdral"`, `args = ["mcp", "stdio"]` (preferred: no secret, no port);
- `streamable-http` to the loopback listener `http://127.0.0.1:<port>/mcp` started by `qdral mcp serve`, with the local
  bearer supplied through `api_key_env = "QDRAL_LOOPBACK_TOKEN"`, `api_key_header = "Authorization"`, and
  `api_key_format = "Bearer {token}"`; the token never appears in the file.

Both serve the local `desktop_structured` profile through qdrald's local policy and approval.

### Provider limitation

The Vibe Code CLI does not support MCP servers that require OAuth. It therefore has no remote path to a Qdral relay.
Qdral deliberately does not add a static API key or bearer credential to the public relay: the relay accepts only OAuth
access tokens bound to its resource, and tests prove static bearer, loopback-style, and Basic credentials receive 401 with
an OAuth challenge.

## Vibe Work custom connector (remote)

An administrator adds a custom MCP connector with the relay URL `https://<relay-host>/mcp`. Vibe Work detects OAuth 2.1
and registers through dynamic client registration with an https callback, which the relay matches exactly. The
connection pairs a device, is bound to a locally started remote-session lease, and receives exactly the `core` profile
with contract annotations and security schemes. Mistral's exact callback URL is not published in the documentation
reviewed, so qualification uses a representative https callback through the same registration path.

## Requirements re-verification

Checked on 2026-10-04 against `https://docs.mistral.ai/vibe/code/cli/mcp-servers` and
`https://docs.mistral.ai/le-chat/knowledge-integrations/connectors/mcp-connectors`. Re-check before any directory
submission.
