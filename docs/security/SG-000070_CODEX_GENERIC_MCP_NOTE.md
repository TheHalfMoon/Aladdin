# SG-000070 Codex and Generic MCP Distribution Security Note

Status: IMPLEMENTATION FOR QDRAL-P17
SpecGrain: SG-000070
Base: `dc32bbf81527f5008c4b5cb9608b13cc9a9c7676`
Date: 2026-10-04

## Changes

- `distribution/codex/qdral/` Codex plugin package and `.agents/plugins/marketplace.json`: one local `stdio` server,
  `qdral mcp stdio`; no endpoint, environment, header, or secret. Tests pin the server exactly, the version to the release
  version, assets to packaged `./assets/` paths, and scan every file for secret patterns.
- `crates/qdral-lifecycle/src/doctor.rs`: `mcp_stdio_entrypoint` resolves the same verified launch `qdral mcp stdio` uses;
  `remote_integration` reads only `relayOrigin` (accepted only as `https://` or exact loopback, otherwise reported as
  unrecognized) and whether any connection is paired from the uplink record. The record also holds the device private key;
  tests prove it never appears in the output. Doctor still changes nothing.
- Interoperability: stdio and loopback discovery are identical; relay discovery is exactly `core`; shared tools are
  byte-identical across transports. A Codex-style OAuth client with a `127.0.0.1` loopback redirect on a different port
  reaches exactly `core` through the real relay.

## Authority delta

None. No tool, scope, profile, approval class, trust right, lease right, actuation, static relay credential, or generic
network authority is added.
