# SG-000065 Tool Profiles and Metadata Contract Note

Status: IMPLEMENTATION FOR COTRA-P16
SpecGrain: SG-000065
Base: `dfac619b72f830bb69903700f70535ab9c187f3c`
Date: 2026-10-02
Code: `apps/cotra-mcp/src/tool_contract.ts`, `apps/cotra-mcp/src/server.ts`
(`allowedToolsFor`, contract-gated registration),
`apps/cotra-mcp/src/oauth_authorization.ts` (derived scope matrix),
`crates/cotra-provider-fs/src/lib.rs` (`MAX_LIST_ENTRIES`). Generated record:
`docs/p16/TOOL_CONTRACT.md`.

## 1. One contract

`TOOL_CONTRACT` holds exactly one entry per canonical tool: kernel shape,
effect class, approval class, remote OAuth scopes or `local_only`, output
bound, title, and MCP annotations. Everything else is derived from it:

- registration: every `registerTool` call passes through a contract gate; a
  tool without an entry fails server startup, a tool outside the
  transport's profile is not registered, and the entry's title and
  annotations are attached on every transport;
- `OAUTH_SCOPE_TOOL_MATRIX`, `REMOTE_TOOL_NAMES`, and
  `LOCAL_ONLY_TOOL_NAMES`;
- profile membership (`SURFACE_PROFILES`).

## 2. Profiles and discovery

| Profile | Served by | Members |
| --- | --- | --- |
| `core` | relay | the 26 tools with remote scopes |
| `desktop_structured` | stdio, loopback HTTP | `core` plus the 5 local-only tools |

Before this grain the relay path advertised all 31 tools to remote clients
(the 5 local-only tools were denied on call, but discovery leaked them).
The relay now lists exactly `core`. Discovery hiding is not authorization:
the relay edge and device uplink scope checks, the cotrad remote-session
scope table, the lease, workspace policy, and local approval still decide
every call. `developer`, `coordinate_fallback`, and unknown profiles are not
mapped and throw at server construction.

## 3. Providers

Known remote provider kinds (`generic`, `openai`, `anthropic`, `mistral`,
`codex`) all map to `core`; no provider gains or loses a tool relative to
another in v0.2. A relay context with any other provider kind, or with a
profile other than its provider's mapping, throws `TOOL_SURFACE_DENIED` at
server construction, which the device uplink reports as a transport failure
without dispatching anything. Local transports serve `desktop_structured`
whatever provider kind is configured; provider kind is metadata, never proof
of identity.

## 4. Annotations

- `readOnlyHint`: never changes local state (reads, previews, desktop
  observation, clipboard read, `web_fetch`).
- `destructiveHint`: may delete or overwrite existing user data
  (`fs_write`, `fs_edit`, `fs_remove`, `clipboard_write`, and
  `process_spawn`, whose registered executable's effects are not
  modelled). Additive and index-only changes are not destructive.
- `idempotentHint`: repeating the call has no further effect.
- `openWorldHint`: reaches network peers or runs a registered executable
  (`git_fetch`, `git_push`, `web_fetch`, `process_spawn`).

Tests enforce the invariants: observation effects are read-only, state
changes are not, read-only is never destructive, destructive and execute
effects declare destructive, network and execute effects declare open
world and nothing else does, approval is required exactly for effects
beyond observation, and only `fs_remove` requires STRONG presence (as
cotrad requests it).

## 5. Output bounds

Every entry states its output bound. Recording them found one unbounded
output: `fs_list` returned every entry of a directory. It now returns at
most 5000 entries, sorted by name before truncation so the result is
deterministic, and reports `truncated`. This only narrows output.

## 6. Cross-checks

`apps/cotra-mcp/src/tool-contract.test.ts` pins:

- contract keys equal the canonical catalog; titles unique; bounds stated;
- each entry's kernel shape equals the parity inventory's record for that
  tool;
- each remote entry's scope equals cotrad's `required_scopes` mapping in
  `crates/cotra-policy/src/remote_session.rs`, each local-only shape has no
  remote mapping there, and the OAuth matrix and `core` ceiling equal the
  26 remote tools;
- exact profile membership and fail-closed unknown profiles and providers;
- real MCP discovery over an in-memory transport: stdio and loopback list
  31 tools with the contract's titles and annotations, and the relay lists
  exactly the 26 `core` tools for every known provider;
- the contract module is metadata only;
- `docs/p16/TOOL_CONTRACT.md` matches the contract row by row.

## 7. Authority

No tool, kernel shape, scope, approval class, or authority is added. The
remote discovery surface and the `fs_list` output only narrow.
