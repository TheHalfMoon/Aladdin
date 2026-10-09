# Source Reuse Matrix (Proposed)

Status: PROPOSED PLANNING ONLY. No code is imported by this document. Every future import still requires the P20 donor-ledger checklist (`docs/canonical/DESKAL_P20_DONOR_LEDGER.md` Section 9) at the exact diff.

Dispositions: COPY (verbatim), ADAPT (copy and modify), WRAP (ship unmodified behind an Aladdin boundary), REFERENCE (patterns only, no code), REJECT.

All revisions below were resolved on 2026-10-09 by shallow clone (`git rev-parse HEAD`) unless marked otherwise.

## 1. Founder repositories

Access: private repositories were read with the founder's authenticated `TheHalfMoon` GitHub session available on this machine. All listed founder repositories are Apache-2.0 or MIT (dual for Winds) at the inspected revisions. Founder ownership simplifies rights, but every import still needs provenance records and third-party dependency review. Because this repository is public, private-repository rows are summarized at capability level only; file-level evidence is kept in a private appendix delivered to the founder outside this repository, consistent with #279 ("Public plan must not disclose private-repo code/data").

| Repository @ revision | Visibility | Relevant files | Tests observed | Reuse opportunity | Decision | Effort | Dependency impact | Security notes |
|---|---|---|---|---|---|---|---|---|
| `kernux` (private) @ `2085b6ed1121b1a94c66c076bdd6b578da4dad0d` | private | Policy vocabulary crates, secret-handle crate, SQLite event/artifact store, KRP v1 protocol schema, ComputerUse donor plan | Unit tests per crate | Method-ceiling vocabulary (`ACCESSIBILITY_SEMANTIC`, `WINDOW_TARGETED_INPUT`, `COORDINATE_FALLBACK`, `GLOBAL_INPUT`), egress classes for screenshots sent to Hala One, consequence classes, proposal-normalization rules; opaque zeroizing secret handles; event/CAS store design for receipts | ADAPT vocabulary and secret handles; REFERENCE store and protocol | S to M | `zeroize`; `rusqlite` only if the store is adopted | Must not introduce a second authority. Map onto `qdral-policy`; never choose the less restrictive of two policies. |
| `Winds` (private) @ `3bfe45fec5c36ef1e407edf12c96ff5444f03847` | private | Rust PTY/ConPTY terminal session engine with Windows-native lifecycle, soak, and replay tests; Tauri 2 desktop shell | Yes (native Windows) | Same-language interactive-session engine for SG-000096 T03 instead of a TypeScript port of Desktop Commander; Tauri app skeleton reference | ADAPT (sessions); REFERENCE (app shell) | M | `portable-pty` (already lock-audited in Winds), Tauri 2 | PTY ownership is lifecycle ownership, not confinement; keep Aladdin Job Object supervision and Full User gating. |
| `Kodac` (private) @ `406b335277f2df1e3dedf24cdb45847dff919d44` | private | Provider-neutral model layer (OpenAI-compatible and Responses), bounded agent loop with repetition detection and model-free tool-result pruning | Yes | Server-side Aladdin AI orchestrator foundation | ADAPT (server only) | M | TypeScript server only | Tool authority stays out of the model loop; the orchestrator emits typed proposals only. |
| `Sentrdel` (private) @ `f5747319a50831ef7cee983d253c0ca5503c9a64` | private | Secret detectors and redaction-before-persistence | Yes | Redaction of task logs and OCR text | ADAPT (detectors) | S | None | Redaction is defense in depth, not a guarantee. |
| `Ascout@ca6b6f514e5a8881e2cfa2789b5e5ea43e3aaaad` | public | `src/assurance/publication/publication-receipt.ts`, `schemas/` | Large suite | Receipt shape: exact inputs, what passed, failed, and was never checked | REFERENCE | S | None | Receipts must say "not verified" explicitly rather than omit. |
| `Orcel@0c58d681740957bc03aa1eb637f6287b44d55666` | public | Whole framework (derived from `vercel/eve@9c36b7c`, Apache-2.0 with NOTICE) | e2e suite | Durable agent workflow structure (instructions, tools, skills on the filesystem) for Aladdin AI server workflows | REFERENCE; possible WRAP on the server | M | Node server stack | Upstream Vercel NOTICE obligations apply. Not for the device. |
| `Morize@62fc04d01d398da93b4dacf0ab2f33e3f1440462` | public | Contracts only ("foundation planning") | n/a | Typed memory mutations (STORE, SUPERSEDE, FORGET, REDACT) for Aladdin AI conversation memory | REFERENCE | S | None | Not implemented; do not depend on it. |
| `SpecGrain@5de7d6499bb0a9e3a191fc0934399cf099d1980a`, `Diffcipline@1e6d14f77b95bb132b42276f10d67f1018ab5bb6` | public | Governance tooling | Yes | Already Aladdin's delivery governance | KEEP as tooling | 0 | None | |
| `commandF@f82565cca917d119e1c774b2c470e2ac20e0d6dd` | public | FHIR change intelligence | n/a | Not relevant to computer use | REJECT | 0 | | |

Not accessible: the founder's Desktop Commander fork and any private TinyFish source. A search of all four authenticated GitHub accounts on this machine (`TheHalfMoon`, `AbdulazizShehri`, `IamShehri`, `wepld`) found no Desktop Commander fork and no TinyFish repository; no TinyFish artifacts were found on disk. Status: UNKNOWN / NOT PROVIDED. No permission is inferred.

## 2. External sources

| Source @ revision | License (verified) | Relevant parts | Decision | Rationale |
|---|---|---|---|---|
| `opensymph/open-computer-use@5b433b98019c18201a15d11e8c3cb0010879a3d8` | MIT | Already imported into `apps/deskal-computer-host` (SG-000094/95) | KEEP as imported; REFERENCE upstream | The pin equals current upstream HEAD (last push 2026-08-26, 14 stars). Low maintenance signal: Aladdin now owns this code and must maintain it. |
| `wonderwhy-er/DesktopCommanderMCP@ea3ed35a7be9f2a3ea3e89185ff9bbb03fe5ab57` (v0.2.52; SG-000096 pin remains `bc1e944`) | MIT | `src/tools/edit.ts` (exact plus fuzzy search/replace), `src/search-manager.ts` (ripgrep streaming search with pagination), `src/tools/pdf/*`, `src/terminal-manager.ts` | ADAPT edit/search/rich-file behaviors for SG-000097; REFERENCE terminal mechanics (prefer Winds Rust ConPTY); REJECT remote device service, telemetry, feature flags, onboarding injection | Verified: telemetry on by default (`config-manager.ts:189`), remote flag file `https://desktopcommander.app/flags/v2/production.json`, remote-gated `onboarding_injection` into tool results, hosted Supabase realtime relay for remote. Self-declared "guardrails, not a sandbox". |
| `tinyfish-io/agentql@9257f7aa261114b24e099f193a4516ed74014b0f` | MIT | Repository contains only examples and templates; SDKs call the hosted API with an API key | REFERENCE (query-language idea, structured output shape) | The semantic resolution engine is not open source. "Port TinyFish" is not possible from public code. A hosted TinyFish adapter remains an opt-in, bring-your-own-account provider. |
| `microsoft/playwright-mcp@b8b4183e099f136cbec0388a6088d4aa2f6b9685` (depends on `playwright 1.64.0-alpha`) | Apache-2.0 | Accessibility snapshot with element refs; persistent, isolated, and extension (existing browser) modes | REFERENCE for snapshot/ref design; consider WRAP of `playwright-core` only if a Node sidecar is retained | Playwright brings a large Node dependency. A Rust CDP client avoids shipping Node (see D4/D7). |
| `ChromeDevTools/chrome-devtools-mcp@2744afa8922e2f28a3c512a7aaf6f72d88bbd9a0` | Apache-2.0 | CDP tool vocabulary, performance tracing | REFERENCE | |
| `browser-use/browser-use@c75e8476e26d18b7617643bc2ae082fae8eae431` | MIT | Direct CDP (`cdp-use==1.4.5`), DOM serializer with indexed elements, watchdogs | REFERENCE (DOM serialization and watchdog patterns) | Python runtime is undesirable on device. |
| `browserbase/stagehand@2d605b099996e10c3e3ab0dcf293db8d482a2c97` | MIT | v4 runs as an MV3 extension (`packages/extension/manifest.json`: `debugger`, `scripting`, `tabs`, `<all_urls>`), JSON-RPC protocol, self-healing act cache, deep shadow-DOM locators, WebMCP | REFERENCE now; ADAPT extension pattern for the "existing signed-in browser" mode | Extension with `debugger` permission is powerful; must be bound to an Aladdin pairing key and visible indicator. |
| `webmachinelearning/webmcp@b206dae8bad34ea8e6ff7213b417b50b14a37e9a` | W3C CG report | `document.modelContext.registerTool` (draft) | REFERENCE; implement discovery when stable | Draft Community Group report; Chrome origin trial; API changed in 2026 (`navigator.modelContext` deprecated, `provideContext` removed). |
| `microsoft/UFO@a795552d976c4c019d7c2f778a0effb5cef7de6b` | MIT | UFO2 Windows hybrid GUI+API agent; UFO3 Galaxy: Constellation DAG orchestration, AIP protocol (`aip/`), heartbeat and reconnection | REFERENCE (DAG scheduling, heartbeat, reconnection); REJECT AIP as security design | Verified: AIP transport is WebSocket with optional `wss`; no device authentication found in `aip/` (grep for token/auth returned only URL handling). |
| `trycua/cua@5a364bbe60e1f8a901ceacd889606b6367dc96ab` | MIT root; FSL-1.1-MIT for Spaces, relay, keyvault, teleport, media (per `LICENSING.md`) | `libs/cua-bench` (benchmark runner), `libs/cua-driver` (MCP stdio driver; one-use `capture_id` binding) | REFERENCE; possible WRAP of cua-bench as a test harness | Do not import FSL directories. The optional perception extension includes AGPL-3.0 OmniParser: REJECT. |
| `CursorTouch/Windows-MCP@b455c2766c63599d466a6178641bac70787979a4` | MIT | Python UIA tools, DOM mode via UIA for browsers | REFERENCE (competitor baseline) | Aladdin's Rust/Go UIA path is more governed. |
| `simular-ai/Agent-S@15776a8dc659658b1963816ea006f7af1978e595` | Apache-2.0 | Agent framework, behavior best-of-N | REFERENCE (benchmark baseline) | |
| `bytedance/UI-TARS-desktop` (pinned in P20 ledger at `2ff41a9e515828c5bd5b276e493d73aa0bdf4a3a`) | Apache-2.0 | `packages/ui-tars/action-parser` | ADAPT as planned in SG-000104 | Parser output is proposal-only. |
| `Ylianst/MeshAgent@26846b1c7f0841d71938f6bebc96eef5a947d217` | Apache-2.0 per readme (GitHub detects no LICENSE file); bundles OpenSSL, libjpeg-turbo, WebRTC | Outbound `wss` control channel, server certificate hash pinning (`ServerID`), notify bar and monitor border indicators, relay tunnels | REFERENCE only | C plus Duktape runtime installed as a SYSTEM service; far larger attack surface than Aladdin needs. |
| `Ylianst/MeshCentral` (Apache-2.0, GitHub API) | Apache-2.0 | Server-side device groups, user/device rights, relay | REFERENCE | Node server; Aladdin already has a relay. |
| `rustdesk/rustdesk` (AGPL-3.0, GitHub API) | AGPL-3.0 | Rendezvous/relay (hbbs/hbbr), NAT traversal | REFERENCE ONLY (no code, no linking) | AGPL network-use obligations; commercial implications unresolved. |
| `morluto/rea` npm `rea-agents@6.1.0` (tag commit `ae9aaee16b9c738d761d3938a3b34fdf37e581b5`) | MIT | Developer research tool | USE as offline research tool; never in product runtime | See `REA_INVESTIGATION_REPORT.md`. |
| `firecrawl/firecrawl@7120d16926544483513782b5393ae9850691d52a` (P20 pin) | AGPL-3.0 public root; founder-asserted direct permission | Scrape pipeline | KEEP GATED as in P20 ledger | Durable written permission evidence is still required before any import. |

## 3. Models

| Model | License (from model card) | Decision |
|---|---|---|
| `Hcompany/Holo4-35B-A3B` | Apache-2.0 weights; base Qwen3.6-35B-A3B MoE; verify base and processor licenses at the exact revision | CANDIDATE base for Hala One |
| `Hcompany/Holo4-27B` | CC BY-NC 4.0 | REJECT for commercial service |
| `LiquidAI/d1-3B`, `LiquidAI/d1-omni-600M` | `lfm1.0` (LFM Open License v1.0): commercial use only while annual revenue is at or below US$10,000,000; above that a commercial license is required | CANDIDATE base for Reliance with a recorded revenue-threshold risk |

## 4. Avoiding whole-product imports

No donor product is imported wholesale. Each row names a bounded file set. The largest planned import is the Firecrawl scrape subset, which remains gated. The Device Fabric reuses the existing Aladdin relay and device identity code instead of MeshCentral or RustDesk.
