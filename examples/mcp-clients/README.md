# Local MCP client examples

Copy-paste configuration for independent MCP clients. Every example uses
only the supported installed entrypoints:

- stdio: command `cotra` with arguments `["mcp", "stdio"]`
- loopback HTTP: `http://127.0.0.1:<port>/mcp` from a running
  `cotra mcp serve` listener with a bearer credential

No example requires the OpenAI tunnel. No example contains a secret.
Replace `PORT` with the printed listener port and export a token of at
least 32 characters before serving:

```powershell
$env:COTRA_LOOPBACK_TOKEN = "<choose-at-least-32-characters>"
cotra mcp serve
```

## Files

- `claude-desktop.json`: paste under `mcpServers` in
  `claude_desktop_config.json`.
- `claude-desktop-mcpb-manifest.json`: manifest for packing a `.mcpb`
  Desktop Extension with the `mcpb` CLI (rename to `manifest.json` at the
  bundle root). Directory listing and review by Anthropic are separate
  external steps and are not claimed here.
- `codex-config.toml`: `[mcp_servers.cotra]` for `~/.codex/config.toml`.
  Uncomment the HTTP block to use loopback instead of stdio.
- `vibe-code-config.toml`: `[[mcp_servers]]` for Vibe Code `config.toml`,
  stdio plus a commented loopback block using the documented
  `api_key_env` fields.
- `generic-stdio.json` / `generic-http.json`: provider-neutral shapes for
  any MCP client and for the MCP Inspector `--config` file.
- `inspector.sh`: exact Inspector CLI invocations used to qualify these
  examples (stdio, loopback HTTP, and config-file modes).

## Inspector qualification

```sh
sh examples/mcp-clients/inspector.sh
```

The script drives `tools/list` through three independent client paths and
checks the 20-tool catalog on each. It needs Node.js, the repository
checkout (for `node apps/cotra-mcp/dist`), and a `cotrad` binary for
`COTRA_DAEMON`. stdio and HTTP paths that skip the inspector also work;
see the contract tests for the static guarantees.
