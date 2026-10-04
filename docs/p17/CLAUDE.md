# Qdral with Claude (SG-000068)

Status: SOFTWARE QUALIFICATION FOR QDRAL-P17. Directory and account states are recorded only in
`distribution/claude/distribution-state.json` from directly observed Anthropic evidence.

## Local paths

- Claude Desktop: add `examples/mcp-clients/claude-desktop.json` under `mcpServers`, or pack
  `examples/mcp-clients/claude-desktop-mcpb-manifest.json` as a `.mcpb` Desktop Extension. Both launch `qdral mcp stdio`.
- Claude Code: use `examples/mcp-clients/claude-code.mcp.json` as a project `.mcp.json`, or
  `claude mcp add qdral -- qdral mcp stdio`.

Local transports serve the `desktop_structured` profile through qdrald's local policy and approval.

## Remote custom connector

Add a custom connector in Claude with the owner's relay `https://<relay-host>/mcp` URL and choose automatic registration.
The relay supports dynamic client registration, PKCE S256, exact `resource`/audience binding, and RFC 9207 `iss` on every
authorization response. Client ID metadata documents are not implemented and are not advertised, so Claude registers
dynamically.

- Claude web, desktop, and mobile register `https://claude.ai/api/mcp/auth_callback`; it is matched exactly.
- Claude Code registers `http://localhost:<port>/callback` and binds a new port per session. Following RFC 8252 section 7.3,
  a registered loopback `http` redirect matches a request that differs only in the port; scheme, hostname, path, and query
  must match exactly, and `localhost` is accepted for redirect URIs only, never as an issuer or resource identifier.

Every Claude connection pairs a device through the same device-backed authorization, is bound to a local remote-session
lease started on the computer, receives exactly the `core` profile with contract annotations and `_meta.securitySchemes`,
and is refused local-only tools at the relay edge. Provider kind is metadata, never authority.

## Requirements re-verification

Checked on 2026-10-04 against `https://www.claude.com/docs/connectors/custom/remote-mcp` and
`https://support.claude.com/en/articles/11503834`, plus the Claude Code loopback-port behavior documented in
`https://github.com/anthropics/claude-code/issues/37747`. Re-check before any directory submission.
