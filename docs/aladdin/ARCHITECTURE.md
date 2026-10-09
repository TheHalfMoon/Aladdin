# Aladdin Architecture (Proposed)

Status: PROPOSED PLANNING ONLY. Grants no authority.
Base: `main@61e664b3c39380a76aede29aa9c2d7fcbc449b08`.

## 1. Current-code inventory (what exists today)

All rows were read from source at the base revision. Line counts include tests.

| Component | Path | Lines | Language | State | Notes |
|---|---|---:|---|---|---|
| Authority daemon | `crates/qdrald` | 13,011 | Rust | VERIFIED, production-reachable | Single policy and dispatch kernel; MCP edge talks to it over stdio JSON (`apps/qdral-mcp/src/kernel.ts`). |
| Policy | `crates/qdral-policy` | 17,679 | Rust | VERIFIED | Workspace, executable registry, FullControlLease (`full_control.rs`, `full_control_store.rs`), remote session leases, per-SG policy modules. |
| Approval broker and ledger | `crates/qdral-approval` | 2,293 | Rust | VERIFIED, DEFECT REPRODUCED | SOFT dialog via `MessageBoxW`; STRONG via Windows Hello `UserConsentVerifier`; append-only JSONL ledger with SHA-256 chain. Issue #278 reproduced (see `SECURITY_THREAT_MODEL.md`). |
| UIA provider and registry | `crates/qdral-provider-uia` | 14,542 | Rust | VERIFIED | Typed process/window/element identities, generations, protected-surface exclusion, invoke/value/select/toggle/scroll, frames, proposals, coordinates, input leases, interruption epoch. |
| Windows Computer Host | `apps/deskal-computer-host` | 3,768 | Go | VERIFIED, size UNMEASURED | Private stdio host derived from Open Computer Use: `list_windows`, `observe_window`, `capture_window`, `cursor_position`, `semantic_action`, `window_action`, `input_move/click/drag/scroll/type_text/key/hotkey`. No listener. |
| Process provider | `crates/qdral-provider-process` | 4,788 | Rust | VERIFIED (Safe), SG-000096 in progress | Registered, hash-pinned executables; AppContainer and Job Object primitives; Full User shell not armed. |
| Filesystem provider | `crates/qdral-provider-fs` | 1,630 | Rust | VERIFIED | Workspace-bound read/write/edit/move/mkdir/remove with content-hash binding. |
| Git provider | `crates/qdral-provider-git` | 5,339 | Rust | VERIFIED | Status/diff/log/branch/stage/commit/fetch/push with destination policy (fetch/push fail closed in installed runtime). |
| Clipboard and network | `crates/qdral-provider-clipboard`, `crates/qdral-provider-network` | 2,592 | Rust | VERIFIED | Text-only clipboard with secret denial; HTTPS GET with public-address pinning. |
| Browser provider | `crates/qdral-provider-browser` | 7,123 | Rust | VERIFIED as CONTRACT-ONLY | Origin, redirect, download/upload governance is real; the DOM snapshot is a deterministic template (`lib.rs:1730`, "Deterministic structural template bound to the page origin"). There is no live DOM engine behind it. |
| Browser host | `crates/qdral-browser-host` | 7,894 | Rust | VERIFIED as SUPERVISION-ONLY | Launches Chrome/Edge headless with an ephemeral profile and serves only hello/ping/shutdown (`src/main.rs`). No CDP session is driven. |
| MCP edge | `apps/qdral-mcp` | 13,761 | TypeScript | VERIFIED | 40 tools: 26 in the remote `core` profile, 14 local-only (`tool_contract.ts`); stdio, loopback Streamable HTTP, relay device transports; OAuth 2.1; device identity and pairing. |
| Relay | `apps/qdral-relay` | 4,720 | TypeScript | VERIFIED | Self-hostable relay with route binding, quotas, file store; container built in CI. |
| Lifecycle | `crates/qdral-lifecycle` | 11,085 | Rust | VERIFIED | Per-user install, update, rollback, doctor, release qualification. |
| Website | `apps/web` | 738 | TypeScript | VERIFIED | GitHub Pages marketing site only. There is no desktop GUI application on `main`; draft PR #282 (opened during this research) proposes a read-only WPF preview in `apps/aladdin-desktop-preview`. |

Release state (VERIFIED via `gh release view v0.1.0`): the only published release is "Cotra 0.1.0" (`cotra-0.1.0-windows-x64.zip`, 4,839,256 bytes). It predates SG-000094 to SG-000096 and requires a separately installed Node.js 20+.

Documentation drift (VERIFIED): `README.md` lists 31 tools; the code exposes 40. `Cargo.toml` `repository` still points to `TheHalfMoon/Deskal`. The README product name is still Deskal. These are identity-migration items for #281, not authority changes.

### 1.1 Answers to the inventory questions

| Question | Answer |
|---|---|
| Implemented | Authority kernel, approvals, workspace FS, Git, clipboard, HTTPS fetch, UIA semantic actions, window capture, bounded raw input under Full User, device identity, pairing, OAuth, relay, lifecycle, release supply chain. |
| Production-reachable | Local: all 40 tools (desktop actions require a locally granted Full User lease). Remote: the 26-tool `core` profile only. |
| Test-only or contract-only | Browser DOM observation and actuation (template data), browser host (supervision only), remote full control (contract and lease isolation only). |
| Blocked by governance | Full User shell and interactive sessions (SG-000096 T02 to T05, gated by #278), Full Admin, Persistent Admin, remote full control, browser profile expansion. |
| Exists but not integrated | Browser host supervision is not connected to any DOM engine; Go host is not in any published release. |
| Duplicated (apparent; confirm in CU-01) | `qdral-browser-host` exports its own coordinate/input-lease (`src/coordinates.rs`) and capture (`src/capture.rs`) contracts, while `qdral-provider-uia` owns frames, proposals, coordinates, and input leases for the desktop. Before adding browser actuation, decide one owner for coordinate and capture identity (recommended: the UIA registry for desktop surfaces, the browser engine only for in-page DOM targets). |
| Extend directly | UIA registry, approval broker (after #278), FullControlLease, process provider, relay, device identity. |
| Replace | Browser provider snapshot/actuation internals (replace the template with a real CDP engine behind the same contracts). |
| Preserve as compatibility interface | `qdral` CLI, `qdrald`, `@qdral/*`, `QDRAL_*`, `qdral.*` OAuth scopes, storage paths, MCP name `qdral`, tool names, failure-code vocabulary. |

## 2. Target architecture

```text
            Untrusted intent sources
  Aladdin app chat/voice | Claude | ChatGPT | Codex | Cursor | other MCP
                 |                     |
                 |              MCP (stdio / loopback HTTP / relay)
                 v                     v
        +--------------------------------------------+
        |  Aladdin edge (MCP + local app API)        |  untrusted-input parsing,
        |  apps/qdral-mcp  (later: Rust edge, D7)    |  tool discovery, client binding
        +---------------------+----------------------+
                              | private stdio frames
        +---------------------v----------------------+
        |  qdrald  — THE ONLY AUTHORITY              |
        |  policy, profiles (Safe/Full User/...),    |
        |  leases, approvals (#278-repaired ledger), |
        |  method ceilings, egress classes, audit,   |
        |  task receipts, kill switch                |
        +--+--------+---------+---------+---------+--+
           |        |         |         |         |
     Computer   Browser    Shell/PTY   Files/Git  Device Fabric
     Host (Go)  Engine     Host        providers  (relay/peer)
     UIA+input  (CDP, D4)  (ConPTY,D5)            (D6)
           |        |         |
       Windows   Chrome/Edge  ConPTY + Job Object
```

Rules (unchanged from P20, restated as invariants):

1. `qdrald` is the only component that decides authority. Hosts, donors, models, the app UI, the relay, and Aladdin AI cloud services only propose or execute already-authorized work.
2. Every side effect carries an exact typed target, an observation generation, a method ceiling, and a consequence class. Stale targets fail closed.
3. A failure at one method level never authorizes a stronger method. Escalation needs a policy that already permits it.
4. Results use the P20 states `not_started`, `dispatched`, `completed`, `cancelled`, `outcome_unknown`. No automatic retry after dispatch.
5. Model output (Hala One, Reliance, Claude, GPT, UI-TARS-format parsers) is never authority.

## 3. Execution method hierarchy

| Level | Method | Status in Aladdin | Required authority |
|---|---|---|---|
| L1 | First-party app API (Git, filesystem, Office COM later) | VERIFIED for FS/Git | Capability grant plus SOFT where mutating |
| L2 | Structured browser (CDP DOM/AX tree, WebMCP) | CONTRACT-ONLY; engine missing (D4) | Browser capability plus origin binding |
| L3 | UI Automation semantic action | VERIFIED | Typed element, generation, SOFT |
| L4 | Exact-window targeted input (`PostMessage`/focus-bound `SendInput`) | VERIFIED under Full User | Full User lease plus SOFT |
| L5 | Bounded visual reasoning (frame -> proposal -> coordinates) | VERIFIED pipeline, no model attached | Frame binding, proposal generation |
| L6 | Coordinate fallback | VERIFIED under Full User | Separate method ceiling |
| L7 | Global input | NOT AUTHORIZED | Not in baseline |

## 4. Performance architecture

The largest latency source in screenshot-driven agents is the model round trip per action (typically 1 to 5 seconds per step for hosted frontier models; the Claude computer-use docs state roughly 1,000 to 1,800 input tokens per screenshot). Aladdin's latency strategy is therefore to reduce the number of model turns, not to micro-optimize input injection:

1. Structured-first routing (L1 to L3) removes screenshots entirely for most file, Git, browser-form, and standard-control actions.
2. Batched read-only observation: one call returns window list plus bounded UIA tree plus optional cropped capture, with a single generation token.
3. Observation reuse with revision validation: an observation is reusable for planning but never actionable after a process, window, document, layout, or target generation change. The UIA registry already enforces generation binding.
4. Region capture and zoom (as in Claude's `zoom` action) instead of full-screen capture; the Go host already captures exact windows.
5. Event-driven waiting: UIA structure-changed and focus events, CDP lifecycle events, and process exit events instead of fixed sleeps.
6. Action batching with fail-fast semantics: one model turn may propose an ordered batch; execution stops at the first failure and returns "not executed" for the rest (the Claude toolset contract uses the same rule).
7. Deterministic verifiers (postconditions) run locally and cheaply after each mutation, so the model is consulted for recovery only when a postcondition fails.
8. Remote routing: tasks run on the device that owns the target; only compact results and optional thumbnails cross the network.

## 5. Local host and MCP surfaces

### 5.1 Transports (VERIFIED existing)

| Transport | Path | Use |
|---|---|---|
| stdio | `apps/qdral-mcp/src/entrypoints/stdio.ts` | Claude Desktop, Claude Code, Codex, Cursor, generic MCP clients |
| Loopback Streamable HTTP | `apps/qdral-mcp/src/transports/loopback_http.ts` | Local clients that need HTTP; loopback only |
| Relay device uplink | `apps/qdral-mcp/src/device_uplink.ts` + `apps/qdral-relay` | Remote clients via outbound-only device connection and OAuth 2.1 |
| OpenAI Secure MCP Tunnel | documented in `README.md` | ChatGPT to a local machine without inbound ports |

### 5.2 Proposed additions

| Addition | Rationale | Authority delta |
|---|---|---|
| Task tools: `task_status`, `task_cancel`, `task_receipt` | Long operations need status and cancel without re-invoking side effects | None (observation and cancellation of owned work only) |
| Compact CLI + Agent Skill (`aladdin` command, skill file) | Microsoft's Playwright MCP README now recommends CLI + Skills over MCP for coding agents because large tool schemas and trees waste context. Offer both. | None; the CLI calls the same `qdrald` |
| Per-client identity binding | Bind each MCP session to a client profile id; already partly modeled in `client_profile` fields of `RemoteDispatchContext` | None |
| Device-scoped tool lists | When the Device Fabric lands, tools take an explicit `device_id`; no implicit "current device" for remote calls | Future, gated (D6) |

### 5.3 Client compatibility (verify before claiming)

| Client | Transport | State | Notes |
|---|---|---|---|
| Claude Desktop / Claude Code | stdio | DOCUMENTED in `distribution/claude` | Must be re-tested per release; not claimed here. |
| Codex | stdio | DOCUMENTED in `distribution/codex` | Same. |
| ChatGPT | Remote MCP via Developer Mode, local via Secure MCP Tunnel | DOCUMENTED with limits | OpenAI's help center states full MCP including write actions is in beta for Business, Enterprise, and Edu; Plus/Pro write support is disputed across sources; mobile does not support MCP apps; ChatGPT cannot connect directly to a local server. Treat writes as plan-dependent. |
| Cursor and other MCP clients | stdio | UNKNOWN until tested | |

## 6. What not to build

- A second policy engine (Kernux daemon, Agent-S, UFO, Desktop Commander config) inside Aladdin.
- A screenshot-only agent loop.
- A bundled local LLM or model weights in the desktop app.
- A hosted backend that receives OS authority. The cloud proposes; the device decides.
- A public, unauthenticated LAN or WAN listener on any device.
- Remote feature flags that change model-visible content or tool behavior (Desktop Commander's `onboarding_injection` pattern, verified in `src/utils/usageTracker.ts:417`).
- Firecrawl SaaS layers, cua Spaces (FSL), OmniParser (AGPL), RustDesk code (AGPL).
