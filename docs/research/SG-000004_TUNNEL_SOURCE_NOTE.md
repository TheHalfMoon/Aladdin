# SG-000004 source note

The official openai/tunnel-client is supervised as an external executable and is not vendored or reimplemented.

Security-relevant upstream behavior verified on 2026-09-24:
- stdio MCP commands are created with Go exec.Command without an explicit cmd.Env;
- therefore the stdio MCP child inherits tunnel-client's process environment;
- tunnel-client supports control_plane.api_key as file:/path/to/secret;
- Quntal uses file-reference delivery so the runtime key value is absent from the inherited environment.

Canonical upstream pin in docs/research/SOURCE_LEDGER.md remains the provenance reference until a dedicated pin refresh is approved.
