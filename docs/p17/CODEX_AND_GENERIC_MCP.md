# Qdral with Codex and Generic MCP Clients (SG-000070)

Status: SOFTWARE QUALIFICATION FOR QDRAL-P17. Marketplace and account availability are external states and are not
claimed here.

## Codex plugin (local)

`distribution/codex/qdral/` is a portable Agent Plugins package: `plugin.json` (OpenAI/Codex presentation only under
`extensions["com.openai"]`), `mcp.json` with exactly one `stdio` server `qdral mcp stdio`, and `assets/logo.svg`. The
repository marketplace `.agents/plugins/marketplace.json` lists it from this repository:

```sh
codex plugin marketplace add TheHalfMoon/Qdral
```

The official plugin page reviewed shows `mcp.json` with `streamable-http` servers only; the `stdio` server shape and the
marketplace `policy` values (`AVAILABLE`, `ON_INSTALL`) follow the Agent Plugins MCP schema and current Codex practice and
are UNVERIFIED against an official example until re-checked before any marketplace claim.

## Codex configuration

`examples/mcp-clients/codex-config.toml` covers stdio and the loopback listener (`bearer_token_env_var =
"QDRAL_LOOPBACK_TOKEN"`). For a remote Qdral relay:

```toml
[mcp_servers.qdral_remote]
url = "https://<relay-host>/mcp"
```

then `codex mcp login qdral_remote`. Codex registers through dynamic client registration with a loopback redirect, which
the relay matches per RFC 8252 (port may differ; scheme, host, path, and query exact). The connection receives exactly the
`core` profile.

## Generic MCP clients

`examples/mcp-clients/generic-stdio.json` and `generic-http.json` are provider-neutral. Interoperability tests prove stdio
and loopback HTTP serve the identical 31-tool local profile, the relay serves exactly the 26-tool `core` profile, and every
shared tool's metadata (title, description, input schema, annotations, security schemes) is identical on every transport.

## Diagnostics

`qdral doctor` now reports `mcp_stdio_entrypoint` (the verified MCP server and qdrald behind `qdral mcp stdio`) and
`remote_integration` (not enabled, enabled but unpaired, or paired, naming only the relay origin). Neither check reads or
prints the device key or any token, and neither changes configuration or authority.

## Requirements re-verification

Checked on 2026-10-04 against `https://learn.chatgpt.com/docs/extend/mcp?surface=cli` and
`https://developers.openai.com/plugins/build/plugins/`.
