# Competitor Research

Status: PROPOSED PLANNING ONLY. Research date 2026-10-09.

No head-to-head task benchmark was executed in this research. Every performance number below is either DOCUMENTED by its owner or CLAIMED by a third party, and is labeled as such. Nothing here supports a claim that Aladdin is faster or more accurate than any competitor. `BENCHMARK_AND_E2E_PLAN.md` defines what would.

## 1. Method and rights basis

| Target | Method | Rights basis |
|---|---|---|
| Desktop Commander v0.2.52 | Source reading at `ea3ed35`; REA static analysis of the locally installed npm package `dist/` (see REA report) | MIT source and package |
| Open Computer Use | Source reading (already imported into Aladdin) | MIT |
| UI-TARS-desktop | P20 ledger pin; no fresh analysis | Apache-2.0 |
| TinyFish / AgentQL | Public repository (examples only), public pricing and docs | MIT repository; hosted service not inspectable |
| Playwright MCP, Chrome DevTools MCP, browser-use, Stagehand | Source reading | Apache-2.0 / MIT |
| Microsoft UFO2/UFO3, cua, Windows-MCP, Agent-S | Source reading | MIT / Apache-2.0 (cua partly FSL) |
| MeshCentral / MeshAgent, RustDesk | Source reading (MeshAgent), metadata (RustDesk) | Apache-2.0; RustDesk AGPL-3.0 reference only |
| Claude computer use | Public API documentation only | Anthropic's terms prohibit reverse engineering; no local binary analysis was performed |
| OpenAI computer use and ChatGPT agent | Public documentation only | OpenAI's terms prohibit reverse engineering; no local binary analysis was performed |

## 2. Capability matrix

Legend: Y = present (VERIFIED in source or DOCUMENTED by owner), P = partial, N = absent, ? = unknown.

| Capability | Aladdin today | Desktop Commander | TinyFish | Claude computer use | OpenAI computer use | UFO2/UFO3 | cua | Playwright MCP | Stagehand v4 |
|---|---|---|---|---|---|---|---|---|---|
| Local files (read/search/edit) | Y (workspace-bound) | Y (whole disk by default) | N | via bash/text-editor tools | N | P | P | N | N |
| Fuzzy edit, ripgrep search, PDF/DOCX | P (exact edit only) | Y | N | text editor tool | N | N | N | N | N |
| Governed terminal sessions | P (Safe registered executables; Full User shell blocked) | Y (ungoverned) | N | bash tool (client-run) | N | P | Y (sandbox) | N | N |
| Windows UIA semantic actions | Y | N | N | N (pixels) | N (pixels) | Y | P (macOS-first driver) | N | N |
| Window capture and raw input | Y (Full User, exact window) | N | N | Y (client executes) | Y (client executes) | Y | Y | N | N |
| Live browser DOM automation | N (contract only) | N | Y (hosted) | pixels | Y (Playwright example) | P | P | Y | Y |
| Existing signed-in browser | N | N | ? | via desktop | N | P | P | Y (extension) | Y (extension) |
| Structured web extraction | N | N | Y | N | N | N | N | P | Y |
| Per-action human approval | Y (SOFT/STRONG) | N | N | developer responsibility | developer responsibility | P | N | N | N |
| Typed targets with staleness checks | Y | N | ? | N | N | P | P (`capture_id`) | Y (refs) | P |
| Multi-device control | P (one device via relay) | P (hosted relay) | N | N | N | Y (Galaxy, unauthenticated AIP) | P (FSL relay) | N | N |
| Audit receipts | Y (ledger, evidence) | P (usage stats) | ? | N | N | P | P | N | P (OTel) |
| Telemetry default | None found (no telemetry code in `apps/qdral-mcp/src` or `crates`) | On (opt-out) | n/a | n/a | n/a | ? | ? | ? | ? |
| Model-provider independent | Y | Y | N (hosted) | N | N | Y | Y | Y | Y |

## 3. Findings per competitor

### 3.1 Desktop Commander (MIT)

- VERIFIED 26 MCP tools: files, search, edit_block, processes, sessions, PDF, config, usage stats, feedback.
- VERIFIED security posture: `allowedDirectories` defaults to `[]` (whole disk), `blockedCommands` is bypassable, and `SECURITY.md` calls the restrictions "safety guardrails" rather than a sandbox.
- VERIFIED telemetry default on (`telemetryEnabled: true`, `src/config-manager.ts:189`).
- VERIFIED (REA plus source): remote feature flags fetched from `https://desktopcommander.app/flags/v2/production.json` gate `onboarding_injection`, `user_surveys`, and `experiments`; onboarding text is injected into model-visible tool results when the flag is on (`src/utils/usageTracker.ts:406-450`).
- VERIFIED remote mode: an OAuth device flow, then a hosted Supabase realtime channel; the device session is persisted at `~/.desktop-commander-device/device.json`; on Windows no custom ACL is applied (per its README).
- Lesson for Aladdin: match DC's breadth of developer tools and convenience, never its ambient authority, vendor-controlled model-visible content, or default telemetry.

### 3.2 TinyFish (hosted)

- VERIFIED the public `agentql` repository contains only examples; the engine is hosted.
- CLAIMED pricing: pay-as-you-go about US$0.015 per agent step (sources conflict between $0.015 and $0.016), browser time about $0.002 per minute, failed runs free. Sources disagree on plan concurrency.
- Strengths (DOCUMENTED): natural-language queries with typed output, resilience to layout changes, works behind authentication.
- Lesson: semantic extraction with typed output is the feature to match locally. Aladdin's local browser engine plus Hala One or a BYO model can provide it without per-step fees; TinyFish stays an optional BYO-account provider.

### 3.3 Claude computer use (Anthropic API)

DOCUMENTED (platform docs, 2026-10-09):

- Current toolset `{"type": "computer_toolset_20260801"}` with no beta header; earlier `computer_20251124`.
- 17 member actions: `screenshot`, `zoom`, click variants, `left_click_drag`, `mouse_move`, `left_mouse_down/up`, `cursor_position`, `scroll`, `type`, `key`, `hold_key`, `wait`.
- The client executes actions; on failure, later actions in the batch must return "Not executed: an earlier computer action in this turn failed."
- Screenshot limits for current models: 2,576 px long edge and 4,784 visual tokens (about 3.75 MP); roughly 1,000 to 1,800 input tokens per screenshot.
- Prompt-injection classifiers scan returned screenshots and steer the model to confirm with the user.
- Recommended: dedicated VM, no credentials, domain allow-lists, human confirmation of consequential actions.

What this means for Aladdin: Claude's model plans; the trust boundary is the client executor. Aladdin can be that executor for Claude with a far stronger boundary than the reference Docker demo, while offering structured tools that avoid screenshots altogether.

Investigation of the executing environment of this research session: this session ran inside the Claude desktop app with documented tool schemas visible to the model (browser pane tools, an optional computer-use skill). The component that performs OS actions is the locally installed Claude desktop application. Its binaries were not analyzed because Anthropic's terms prohibit reverse engineering; only the visible tool contracts were studied. The computer-use tools were not enabled in this session, so no action-latency measurement was taken. Status: UNOBSERVED.

### 3.4 OpenAI computer use and ChatGPT agent

- DOCUMENTED (developer docs, 2026-10-09): tool type `computer`; the model returns a `computer_call` with an ordered `actions` array (`click`, `double_click`, `drag`, `move`, `scroll`, `keypress`, `type`, `wait`, `screenshot`); the app returns `computer_call_output` screenshots; examples use Playwright (browser) and PyAutoGUI (desktop).
- DOCUMENTED: Operator was folded into ChatGPT agent (secondary sources; confirm on OpenAI pages).
- ChatGPT MCP: OpenAI's help center (403 to automated fetch; content from search summaries) states full MCP with write actions is beta for Business, Enterprise, and Edu; Plus/Pro write support is disputed; no mobile support; local servers require the Secure MCP Tunnel. Status: verify manually before any compatibility claim.

### 3.5 Microsoft UFO2 and UFO3 Galaxy (MIT)

- UFO2: Windows desktop agent with hybrid GUI and API actions.
- UFO3 Galaxy: multi-device orchestration with a "Constellation" task DAG, dynamic DAG editing, asynchronous execution, and the AIP protocol.
- VERIFIED: `aip/` transport is WebSocket; `wss` is optional; no device authentication primitive was found in the protocol package. Galaxy's orchestration ideas are useful; its security model is not suitable for Aladdin.

### 3.6 cua (MIT plus FSL)

- `cua-driver`: background computer-use driver over MCP stdio (macOS-first); a region-derived click must carry the same one-use `capture_id` as the observation, which matches Aladdin's frame-generation binding.
- `cua-bench`: benchmark runner with registry tasksets; candidate harness.
- Licensing trap: Spaces, relay, keyvault, teleport are FSL-1.1-MIT; the optional perception extension contains AGPL-3.0 OmniParser.

### 3.7 Browser automation stack

| System | Mechanism | Notable |
|---|---|---|
| Playwright MCP | Accessibility snapshot with refs | Persistent, isolated, and extension modes; Microsoft recommends CLI+Skills over MCP for token efficiency |
| Chrome DevTools MCP | CDP | Performance traces, network inspection |
| browser-use | Direct CDP (`cdp-use`) with indexed DOM serialization and watchdogs | Python |
| Stagehand v4 | MV3 extension (`debugger`, `scripting`, `tabs`) beside the browser; JSON-RPC; act/observe/extract with self-healing cache | Lower round-trip latency; supports WebMCP |
| WebMCP | Pages register tools via `document.modelContext.registerTool` | Draft W3C CG report; Chrome origin trial; API changed in 2026 |

### 3.8 MeshCentral / MeshAgent / RustDesk

- MeshAgent: outbound `wss` control channel to a self-hosted server pinned by certificate hash (`ServerID`), relay tunnels, notify bar and monitor-border indicators during remote sessions; runs as a SYSTEM service with a Duktape JS runtime.
- RustDesk: self-hostable rendezvous and relay servers with NAT traversal; AGPL-3.0, reference only.
- Lesson: copy the user-visible indicators, server pinning, and outbound-only model; keep Aladdin's user-level, per-action governed design rather than SYSTEM-level remote control.

## 4. Benchmark landscape (CLAIMED, third-party trackers)

| Benchmark | Status | Notes |
|---|---|---|
| OSWorld-Verified | Near saturation | Trackers in September 2026 show top scores of about 85 to 86% against a reported human baseline of about 72%. |
| OSWorld 2.0 (XLANG Lab) | 108 long-horizon tasks, checkpoint-graded, binary completion primary metric | Authors report no system above about 21% binary completion at a 500-step budget. |
| Holo4 model card | Vendor claim | Holo4-35B-A3B "30.9% at $0.61 per task" and Holo4-27B "61.7%" on OSWorld 2.0. These conflict with the authors' reported ceiling; the metric (partial versus binary) must be reconciled before use. |

Model-level scores are not evidence of Aladdin product success.

## 5. Where Aladdin can be measurably better

1. Native Windows reliability through UIA semantic actions with typed, generation-bound targets (fewer wrong-target actions, fewer screenshots).
2. Safety that is enforced, not advisory: per-action approvals, method ceilings, no self-approval, emergency revoke. Measurable as adversarial pass rate.
3. Lower cost per verified task: structured-first routing reduces model calls; Core has zero founder compute.
4. Multi-device control with authenticated, per-device grants and visible indicators (UFO3 lacks device auth; DC relies on a hosted relay).
5. Independent verification: deterministic postconditions and receipts rather than model self-report.
