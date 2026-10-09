# REA Deep Investigation — Computer-Use Reuse Intelligence for Aladdin

Status: RESEARCH EVIDENCE ONLY. Grants no authority, imports no code, changes no runtime, policy, or permission. Security issue #278 remains open.
Date: 2026-10-09
Repository base: `TheHalfMoon/Aladdin@61e664b3c39380a76aede29aa9c2d7fcbc449b08` (live `main` at research start and at branch creation)
Related: draft planning PR #283 (contains the earlier `docs/aladdin/REA_INVESTIGATION_REPORT.md`), planning issues #279 and #280.
Research host: Windows 11 Home 10.0.26300 x64, 15.7 GB RAM (about 1 GB free during the work), Node.js 24.19.0, OpenJDK 17.0.20.1, Microsoft Edge 155.0.4283.45. No Go toolchain, no Ghidra, no Hopper, no IDA.

Evidence labels: VERIFIED-SOURCE (read in source at an exact revision), VERIFIED-REA (produced by an REA command recorded here), VERIFIED-RUNTIME (observed in a run executed during this research), DOCUMENTED (owner documentation), HYPOTHESIS (needs measurement), NOT VERIFIED.

---

## A. Executive findings

1. **Desktop Commander injects a vendor `[SYSTEM INSTRUCTION]` block into model-visible tool results, gated by a remote feature flag — confirmed at runtime.** In an isolated run of the installed v0.2.52 with telemetry disabled and the flag file served from `127.0.0.1`, the `list_directory` result grew from 16 to 1,688 characters and contained `[SYSTEM INSTRUCTION]: NEW USER ONBOARDING REQUIRED ... YOU MUST COMPLETE BOTH STEPS BELOW` when the flag was on, and was clean when it was off. The behavior is unchanged in 0.3.1-alpha.2. (Sections C.4, J)
2. **Desktop Commander silently downloads a Chrome for Testing build at every server start when it does not find Google Chrome, and it does not recognize Microsoft Edge.** Runtime tracing attributed outbound connections to `googlechromelabs.github.io` and `storage.googleapis.com` to `@puppeteer/browsers`, called from `ensureChromeAvailable()` at startup; 133 MB landed in the isolated profile within one run. Telemetry and remote-flag settings do not affect this path. (C.4)
3. **Aladdin's `fs_write` is not crash-atomic.** It truncates the target and rewrites it in place (`crates/qdral-provider-fs/src/lib.rs:286-293`). Desktop Commander's newer `atomic-write` (temp file + `fsync` + rename with Windows lock retry) is the pattern to adapt, keeping Aladdin's handle-based path identity checks. (C.1, J)
4. **Playwright MCP is the strongest immediately reusable browser foundation, and it already implements WebMCP.** Against a local fixture with Edge 155, one 2,279-byte snapshot exposed Shadow DOM controls and iframe controls with stable refs (`e11`, frame-scoped `f1e3`), listed a page WebMCP tool labeled "page-provided, untrusted", and emitted `notifications/tools/list_changed`; the snapshot call took 30 ms. Its tool schemas cost 20,296 bytes for 25 tools. (F)
5. **UI Automation already reaches signed-in browser content without CDP, extensions, or cookies.** Edge exposes the page under `AutomationId = RootWebArea`; a .NET UIA client on this host saw Shadow DOM and iframe controls there. Aladdin's existing UIA registry can therefore cover "existing signed-in Edge" with no new authority surface. (E, F)
6. **Batching UIA reads with `CacheRequest` was 3.6× faster than per-property reads** in a 10-iteration microbenchmark (median 63.3 ms vs 17.5 ms for 54 elements × 6 properties). Aladdin's Go host reads each property with a separate cross-process call; Windows-MCP and Microsoft UFO use `CacheRequest`. (E, K)
7. **TinyFish's semantic engine is server-side only.** The MIT AgentQL SDK builds an accessibility tree in the page and POSTs it with the query to `https://api.agentql.com/api/v2/query`; the TinyFish MCP package is a transparent proxy to `https://agent.tinyfish.ai/mcp`. Nothing public contains the resolver. The fastest lawful path to TinyFish-class automation is Playwright's AI snapshot plus a model of Aladdin's choice, not porting TinyFish. (D)
8. **Desktop Commander's remote design contains one reusable idea and one anti-pattern.** Reusable: "doorbell" broadcast plus a durable row claimed with a conditional `UPDATE ... WHERE status = 'pending' AND timeout_at > now`. Anti-pattern: on a database error during the claim it executes anyway (fail-open), and tool results are stored in hosted database rows. (G)
9. **MeshCentral's agent handshake binds authentication to the TLS certificate the agent actually sees** (SHA-384 of the server certificate + both nonces, signed by both sides), blocks repeated handshake commands, expires relay rendezvous IDs after 120 seconds, and combines consent flags with bitwise OR so policy can only add consent. These are directly adoptable patterns for Aladdin's relay and Device Fabric. (G)
10. **REA's static JavaScript analysis is useful for inventory and change detection but has material blind spots**: it did not detect `axios` requests or `process.env[computedKey]` configuration, a literal-seed semantic trace returned zero matches for a string passed as a call argument, and cross-version comparison left 1,277 of 1,387 entities unmatched. Every important finding in this report was therefore re-verified in source or at runtime. (B, J)

---

## B. REA execution evidence

### B.1 Toolchain identity

| Item | Value | Verification |
|---|---|---|
| Package | `rea-agents@6.1.0` (still latest; releases 4.1.0 to 6.1.0 published 2026-10-06 to 2026-10-09) | `npm view`, `gh release list -R morluto/rea` |
| npm integrity | `sha512-xsXFiXEt2hc5ts+zX3D8zdyZHK5Glbp8GFYSIHMZytDiUCGAQp2crDmkoyiXNNbXXcvem228h+k54xUOYCnI7Q==` | recomputed from tarball |
| Tarball SHA-256 | `f997f603b3f1fb8f06593ef2d57989e43cddbd7526c45bcec92ce61d2873ad31` | `sha256sum` |
| Release commit | `ae9aaee16b9c738d761d3938a3b34fdf37e581b5` (= npm `gitHead` = tag `rea-agents-6.1.0`) | `npm view`, `gh api .../git/ref/tags/rea-agents-6.1.0` |
| Repository `main` at research time | `cf415619cadcfcb18cda59a54b26c9d164f750a4` (ahead of release; docs read there) | shallow clone |
| Install scope | Isolated prefix; no `rea setup`, no global install, no MCP client configuration change | — |

### B.2 Diagnostics (`rea doctor`, `rea capabilities`), re-run at the end of the investigation

| Check | Result |
|---|---|
| node / host | healthy (24.19.0, win32 x64) |
| hopper / ghidra / ida | `missing_analysis_engine` (Ghidra requires 12.1.x and JDK 21+; host has JDK 17) |
| Overall | `healthy: false` |
| Available without native engines | JavaScript/Electron application graph and semantic graph, application workflows (trace, compare), artifact and .NET static inspection, CDP browser observation, Playwright browser scenarios |
| Windows process capture | `capability_unavailable`: "Windows PTY process capture is unavailable because this adapter does not yet verify descendant cleanup" (VERIFIED-REA with a trivial scenario) |

Native Windows decompilation remains BLOCKED. Installing Ghidra 12.1.x and JDK 21 is a major installation that was not authorized and was not performed.

### B.3 Audit of the previous investigation (PR #283 report)

| Previous claim | Status now |
|---|---|
| `rea-agents@6.1.0`, integrity as recorded | Re-verified; unchanged |
| First run on 17,510 files failed with heap out of memory | Re-verified from the retained progress log: last progress `(685/17510)`, one `heap out of memory` record |
| Scoped run on Desktop Commander `dist/` analyzed 238 files in 3 min 50 s | Re-verified from retained progress log `(238/238)` and timing file |
| Evidence bundle about 505 MB | The bundle itself was deleted after the earlier session to save disk; its id and digest remain recorded in the earlier report. Equivalent findings were re-derived here from a narrower, retained scope (C) |
| Remote flags, worker `eval`, process spawns, install tracking | Re-verified by REA (C.2) and in source; onboarding injection additionally VERIFIED-RUNTIME (C.4) |
| "Inference: feature flags can change model-visible output" | Upgraded to VERIFIED-RUNTIME |
| Native Windows decompilation unavailable | Re-verified (B.2) |

### B.4 Runs executed in this investigation

| Run | Target and scope | Command | Exit | Time | Output |
|---|---|---|---:|---:|---:|
| R1 | Desktop Commander 0.2.52 core: 73 JS files + `package.json` (excludes `dist/ui`, `dist/npm-scripts`), 824,942 bytes | `analyze-javascript-application` (heap 4 GB) | 0 | 41 s | 192,170,865 B |
| R2 | Desktop Commander 0.3.1-alpha.2 core: 87 JS files + `package.json`, 1,030,683 bytes | same | 0 | 49 s | 254,476,354 B |
| R3 | AgentQL SDK 1.18.1 `dist/` without `.d.ts` (50 files) | same | 0 | 8 s | 31,706,148 B |
| R4 | MeshCentral 1.2.5 `meshagent.js` + `meshrelay.js` | same | 0 | 19 s | 75,523,144 B |
| R5 | R1 evidence, seed literal `onboarding_injection`, forward influence | `trace-javascript-semantics` | 0 | 17 s | 0 seed matches (limitation, J.3) |
| R6 | R1 evidence, seed string `onboarding_injection`, both directions | `trace-application-feature` | 0 | 16 s | 1 seed, 663 nodes reached (module-level, not data flow) |
| R7 | R1 vs R2 | `compare-application-versions` (heap 7 GB) | 0 | 40 s | 1,387 items: 57 unchanged, 53 changed, 724 added, 553 removed |
| R8 | Local fixture in Edge 155 (headless) | `capture-browser-scenario` (DOM, accessibility, URL) | 0 | 22 s | DOM 2,360 B; accessibility 1,177 B |
| R9 | Edge 155 with loopback CDP, isolated profile | `list-browser-targets`, `discover-webmcp-tools`, `inspect-web-page` | 0 | — | CDP `WebMCP` domain available; 0 native tools (E.4) |
| R10 | Trivial `node -e` scenario | `capture-process` | 1 | — | Unsupported on Windows (B.2) |

Scoping changed the cost profile decisively: R1 covered the same Desktop Commander code as the earlier run in 41 s and 192 MB instead of 3 min 50 s and 505 MB, and its semantic graph was not truncated (43,301 nodes, below the 100,000 ceiling).

Supplementary, non-REA runtime instruments used for independent verification (all against local fixtures or isolated profiles):

- an isolated Desktop Commander harness (temporary `USERPROFILE`, telemetry disabled by environment and configuration, feature flags and remote URL pointed at `127.0.0.1`) with per-process TCP sampling and a Node `--require` preload that logs `net.connect`, `dns.lookup`, and `fetch` targets with stack frames;
- an MCP stdio probe measuring `initialize` and `tools/list` size and latency;
- an MCP scenario driver for Playwright MCP;
- a PowerShell .NET UI Automation probe.

### B.5 Evidence retention

Large evidence stays outside Git, in a local research workspace (`%LOCALAPPDATA%\Temp\rd`, 943 MB). An integrity manifest of 496 files (`derived/MANIFEST.json`, SHA-256 `2bd9a0efb3c193b5a646feb808ab2711fc05006b1f7350b3a9443d5dc5f916c5`) records path, size, and SHA-256 for every target copy, evidence bundle, derived summary, scenario, fixture, and helper script. Key artifacts:

| Artifact | Bytes | SHA-256 (prefix) |
|---|---:|---|
| `evidence/dc-0.2.52-core.app.json` (evidence id `ev_40ee3569…ec35f`) | 192,170,865 | `d153c7e27d3bf231` |
| `evidence/dc-0.3.1-alpha.2-core.app.json` (`ev_a1876c48…41267`) | 254,476,354 | `e29277937062e240` |
| `evidence/agentql-1.18.1.app.json` (`ev_b4ba35f9…253a9`) | 31,706,148 | `966ef766204e0b0b` |
| `evidence/meshcentral-1.2.5-agentrelay.app.json` (`ev_5496ee1b…a6c0`) | 75,523,144 | `b3bca25f4b40ee93` |
| `evidence/browser-fixture.capture.json` (`ev_64cff70d…1969b`) | 10,714 | `3eb1f2359075f44a` |
| `derived/dc-0.2.52-vs-0.3.1-alpha.2.compare.json` | 14,490,683 | `f13cbccee5f42f02` |
| `derived/dc-0.2.52.runtime-onboarding-on.json` / `-off.json` | 878 / 661 | `0605ef841c47940e` / `abecde6d9354c6f0` |
| `derived/playwright-mcp-0.0.83.scenario.json` | 2,533 | `e822f700fca78118` |

Intermediate inputs that are reproducible from the bundles (trace and compare inputs, about 500 MB) and the downloaded Chrome build from the isolated run were deleted after hashing the retained outputs.

---

## C. Desktop Commander investigation

### C.1 Artifact identity

| Artifact | Identity |
|---|---|
| Installed package on the research host | `@wonderwhy-er/desktop-commander@0.2.52`; aggregate SHA-256 of `dist/` + `package.json` = `42ee12c5…86662`, identical to the registry tarball's contents |
| Registry tarball 0.2.52 | integrity `sha512-VNeKfaBR6TN/8MlP/ziTpjNEeHrGOFmmzQjrwzr52kQalBJoNNISWwaSQElPFc6+I17NOOM354KbSdS/aDrqGA==`; SHA-256 `9cd9f52277b45602d5544ddfede0f0c5f6453131256010b8f2abba3576040c6c`; `gitHead` `c774c3b505de990219637ecdc9a830c8772fae9d` (2026-09-29) |
| Registry tarball 0.3.1-alpha.2 | integrity `sha512-yrAd3rQisbLpOQIiaPDDxJCy6uWGZxNAbBsnuVD33G9Bu/uO4sX38qERJ56CiS3rSwYMqejIof5X6iyDYc9nKQ==`; SHA-256 `faaf3f34f114589c0630ce4eaa7114ecc628ce2030822dfe67feac0e17fedc4b`; source branch `rc-v0.3.1` at `7fd4898da21b41e20cacfb72042faebaf972e90d` |
| Aladdin SG-000096 donor pin | `bc1e944e30302e0022d49f418d55563dd74a162c` (2026-10-06). Its `package.json` also says 0.2.52 but it is not the published 0.2.52 commit. The six pinned files are byte-identical between `c774c3b` and `bc1e944` (`git diff --stat` empty), so the installed artifact is a valid stand-in for the pinned files. |
| Founder's fork | Not found on any of the four authenticated GitHub accounts available on this host; not analyzed |
| License | MIT, "Copyright (c) 2024-2025 Eduard Ruzga and Desktop Commander Contributors" |

### C.2 REA results (R1, R2)

R1 extracted 11 child-process creation sites, 9 request sites, the remote flag configuration source, and one worker:

| REA observation | Location (built) | Source confirmation (`c774c3b`) |
|---|---|---|
| `DC_FLAG_URL` default `https://desktopcommander.app/flags/v2/production.json` | `dist/utils/feature-flags.js:19-20`; fetch at `:156` | `src/utils/feature-flags.ts:30` |
| `MCP_SERVER_URL` default `https://mcp.desktopcommander.app` | `dist/remote-device/device.js:40` | `src/remote-device/device.ts:56` |
| Telemetry requests | `dist/utils/capture.js:248, 425` | `src/utils/capture.ts:40-41` (`https://telemetry.desktopcommander.app/mp/collect`, fallback Cloud Run URL); disabled by `DESKTOP_COMMANDER_DISABLE_TELEMETRY` or config |
| Worker from an eval string | `new Worker(WORKER_CODE, { eval: true, … })` | `src/tools/fuzzySearch.ts:44` |
| Process creation | ripgrep (`search-manager.js:36`), terminal (`terminal-manager.js:204`), `improved-process-tools.js:28`, browser opener (`open-browser.js:23-32`), setup/uninstall `npm --version` | matching `src/` files |
| Environment detection reads | CI and IDE variables in `track-installation.js`; Kubernetes service-account namespace file in `utils/system-info.js:313` | install tracking and environment detection |

R7 (0.2.52 → 0.3.1-alpha.2) plus a file-level diff: 14 new core files, including `utils/process-tree.js`, `utils/shell.js`, `utils/atomic-write.js`, `utils/exit-process.js`, `remote-device/transport-telemetry.js`, and a user-invoked diagnostics uploader with token redaction; 50 of 73 shared core files changed in nine days.

### C.3 Capability findings and dispositions

| Area | Verified behavior (source location at `c774c3b` unless noted) | Aladdin decision |
|---|---|---|
| Text editing | `edit_block` performs exact replacement; when no exact match exists it runs a fuzzy search (threshold 0.7) in a worker and **returns a character diff without applying any change** (`src/tools/edit.ts:255-345`). Telemetry for this event carries only counts and lengths; the full text goes to a local log `~/.claude-server-commander-logs/fuzzy-search.log`. | ADAPT the near-miss diff response (fewer failed edit turns). Do not log content. |
| Crash-safe writes | 0.3.1-alpha.2 adds `writeFileAtomic`: temp file `<file>.<pid>.tmp`, `handle.sync()`, rename with retry for transient Windows locks, per-path write serialization, directory `fsync` on POSIX (`rc-v0.3.1:src/utils/atomic-write.ts`). | ADAPT into `qdral-provider-fs` (Aladdin currently truncates in place). Create the temp file relative to the already-verified directory handle and rename by handle so reparse-point checks still hold. |
| Search | ripgrep subprocess with result pagination and timeouts; exact-filename searches default to 1.5 s (`src/search-manager.ts:76-123`). | ADAPT the streaming/pagination shape for SG-000097; ship a pinned ripgrep. |
| Large reads | Line-based paging with defaults `fileReadLineLimit: 1000`, `fileWriteLineLimit: 50` (`src/config-manager.ts:190-191`). | REFERENCE (Aladdin already bounds bytes). |
| Terminal | Pipes, not a pseudo-terminal; the child receives the full parent environment plus `TERM` (`src/terminal-manager.ts:205-233`); `ssh` commands are silently rewritten to `ssh -t` (`:195-197`); output buffer up to 50 MB per session with line splitting at 1 MB (`:56-58`). | REJECT the environment inheritance and command rewriting. Prefer the founder's same-language Rust PTY/ConPTY engine (Winds) for SG-000096 T03; keep Aladdin's sanitized environment. |
| Termination | 0.2.52 sends `SIGINT` then `SIGKILL` to the root PID only (`src/terminal-manager.ts:757-768`). 0.3.1-alpha.2 documents that descendants were previously orphaned and adds `taskkill /T /F` on Windows and a `ps` tree walk elsewhere (`rc-v0.3.1:src/utils/process-tree.ts:1-100`). | REJECT. PID-tree killing is racy (PID reuse, reparented children). Aladdin's Job Object kill-on-close is strictly stronger and already exists. |
| MCP surface | 26 tools; `tools/list` = 57,591 bytes (measured). Onboarding injection, feedback surveys, and A/B experiments are flag-driven. | REJECT model-visible vendor content. Keep Aladdin's tighter schemas (40 tools in 36,093 bytes, measured). |
| Startup side effects | Background Chrome download (C.4). | REJECT; use the installed Edge, never download browsers implicitly. |
| Remote devices | See Section G. | ADAPT the claim pattern only. |

### C.4 Runtime verification (isolated harness)

Setup: installed DC 0.2.52 entry point; temporary `USERPROFILE`/`HOME`; `config.json` with `telemetryEnabled: false`; `DESKTOP_COMMANDER_DISABLE_TELEMETRY=1`; `DC_FLAG_URL` pointing to a local HTTP server that returns `{"flags":{"onboarding_injection": <on|off>, "welcome_page_enabled": false, "user_surveys": false}}`; `MCP_SERVER_URL` pointed at a closed loopback port; MCP calls `initialize`, `tools/list`, `tools/call list_directory` on a sandbox folder.

| Observation | Flag off | Flag on |
|---|---|---|
| Flag request to the local server | 1 (`/flags.json`) | 1 |
| `list_directory` result size | 16 characters | 1,688 characters |
| Contains `[SYSTEM INSTRUCTION]: NEW USER ONBOARDING REQUIRED` | no | yes |
| Tool call logged with arguments to `~/.claude-server-commander/claude_tool_call.log` | yes | yes |
| Non-loopback TCP peers of the server process | yes | yes |

Network attribution with the preload tracer (flag on run): besides the local flag fetch, the only outbound targets were `googlechromelabs.github.io:443` and `storage.googleapis.com:443`, both from `node_modules/@puppeteer/browsers/lib/httpUtil.js`, reached through `ensureChromeAvailable()` (`dist/index.js:110`; source `src/index.ts:135`, `src/tools/pdf/markdown.ts:255`). `findSystemChrome()` checks Google Chrome and Chromium locations only (`dist/tools/pdf/markdown.js:120-124`), so a standard Windows machine with only Edge triggers the download. 133 MB was written to the isolated profile's `puppeteer-cache` during one run; on the research host's normal profile an existing cache of 435 MB was observed (size only; nothing was read or modified).

Classification: the onboarding injection is a confirmed design behavior that places vendor-controlled imperative text in the model's input. Its impact is model-steering (prompt-injection-like); it is not an exploit of a vulnerability. The startup download is an undisclosed-by-default network and disk side effect. Neither is used by Aladdin.

---

## D. TinyFish and AgentQL investigation

### D.1 What is available

| Material | Identity | License | Content |
|---|---|---|---|
| `tinyfish-io/agentql` | `9257f7aa261114b24e099f193a4516ed74014b0f` | MIT | Examples and templates only |
| npm `agentql` (JS SDK) | 1.18.1, `gitHead 856e1bf5…` | MIT | Client SDK (analyzed, R3) |
| npm `agentql-mcp` | 1.0.1 | MIT | Thin MCP wrapper |
| `tinyfish-io/tinyfish-mcp-server` | `e14cc69924096d193d35a448de68b5567acaed3a` | MIT | Transparent reverse proxy to `https://agent.tinyfish.ai/mcp` (28 hosted tools) |
| `tinyfish-io/tf-playwright-stealth` | public | MIT | Bot-detection evasion; not used by Aladdin |
| `tinyfish-io/bigset-oss` | public | AGPL-3.0 | Not relevant; not used |
| Private TinyFish source | Not found on any accessible account or on disk | — | Not analyzed |

### D.2 Local versus hosted (VERIFIED-SOURCE in the SDK; R3 for structure)

| Function | Where it runs | Evidence |
|---|---|---|
| Accessibility-tree generation with element ids, iframe path tagging | Browser page (injected snippets) | `dist/core/js-snippets/generated/generated-snippets.js` (`generate-accessibility-tree` 11,151 chars; `set-iframe-path`) |
| DOM change tracking | Page; writes `lastDomChange` into the page's `localStorage` | same file, line 12 |
| Page readiness | Node: network-quiet (500 ms) and DOM-quiet thresholds, 1.5 s per pending request, 6 s fallback | `dist/ext/playwright/page-monitor.js:15-20, 60-130` |
| **Query resolution (natural language to elements or data)** | **Hosted**: POST `{query, accessibility_tree, url, mode}` (compressed) to `https://api.agentql.com/api/v2/query` or `/api/v2/query-data` with `AGENTQL_API_KEY` | `dist/core/api-constants.js`, `dist/core/aql-server-service.js:48-74` |
| Hosted browsers (`/v1/tetra/sessions`), web agent runs, monitors, signed-in profiles | Hosted | `api-constants.js`; TinyFish MCP README |

Defect found by source reading (not a security issue): the readiness check iterates `for (const missingResponse in missingResponses)`, which yields array indices rather than URLs, so `requestLog.get("0")` is undefined and every pending request is treated as settled (`page-monitor.js:99`). The effective readiness rule is only "network quiet and DOM quiet".

Hosted surface design worth mirroring locally (DOCUMENTED, TinyFish MCP README): asynchronous runs with `run_web_automation_async`, `get_run`, `cancel_run`, `batch_status`, and an SSE progress stream.

### D.3 Fastest lawful path to TinyFish-class automation

1. Use Playwright's AI-mode accessibility snapshot with element refs (Section F) as the page representation. It already covers Shadow DOM and iframes and needs no hosted resolver.
2. Resolve natural-language queries with the planner Aladdin already supports (a BYO model now; Hala One later) against that snapshot, returning a typed proposal bound to the snapshot generation.
3. Mirror the run model (`run`, `status`, `cancel`, progress stream) in Aladdin's task tools.
4. Keep TinyFish as an optional bring-your-own-account adapter (point a client at `agent.tinyfish.ai/mcp` or wrap it) with explicit data-egress consent.

Porting TinyFish's resolver is not possible from available material and is not proposed.

---

## E. Native computer-use investigation

### E.1 Sources

| Source | Revision | License | Method |
|---|---|---|---|
| `opensymph/open-computer-use` | `5b433b98019c18201a15d11e8c3cb0010879a3d8` (unchanged since the SG-000094 pin; last push 2026-08-26) | MIT | Source (Go); already imported into `apps/deskal-computer-host` |
| `@ui-tars/action-parser` | 1.2.3 | Apache-2.0 | Source of shipped `dist/actionParser.js` |
| `CursorTouch/Windows-MCP` | `b455c2766c63599d466a6178641bac70787979a4` | MIT | Source (Python) |
| `microsoft/UFO` | `a795552d976c4c019d7c2f778a0effb5cef7de6b` | MIT | Source (Python) |
| Aladdin Go host | `61e664b` | Apache-2.0 | Source |

REA was not applied to Go or Python sources (its value is in shipped JavaScript or native artifacts; native analysis is blocked). Status: source-verified, NOT REA-ANALYZED.

### E.2 Findings

| Topic | Finding | Evidence |
|---|---|---|
| UIA property access | Aladdin reads each property with its own cross-process call (`elemSlotCurrentName`, `CurrentControlType`, `CurrentAutomationId`, `CurrentClassName`, `CurrentBoundingRect`, `GetCurrentPropertyValue`, …) and navigates with tree walkers. | `apps/deskal-computer-host/native_uia_windows.go:41-53, 322, 395, 503-518, 647` |
| UIA batching in competitors | Windows-MCP builds a `CacheRequest` (element + children scope) and uses `BuildUpdatedCache`; UFO uses `CreateCacheRequest` with added properties. | `windows_mcp/tree/cache_utils.py:34-67`; `ufo/automator/ui_control/inspector.py:249-329` |
| Measured effect on this host | 54 web elements × 6 properties under Edge `RootWebArea`: median 63.3 ms uncached vs 17.5 ms cached over 10 alternating runs (ratio 3.6). .NET client microbenchmark; the Go host must be measured separately. | PowerShell UIA probe (B.4) |
| Browser content via UIA | Chrome/Edge expose web content under `AutomationId = RootWebArea`; Firefox needs MSAA/IA2. On this host, Shadow DOM button "Open details" and iframe field "Card number" were both visible through UIA. | `windows_mcp/tree/ia2.py:1-6`; UIA probe |
| Coordinate semantics in model output | UI-TARS coordinates depend on model version: v1.0 uses a 1000×1000 relative grid; v1.5 uses smart-resize pixel factors; results are multiplied by a caller-supplied `scaleFactor`. | `dist/actionParser.js:55-161` |
| Visual-only versus accessibility-first | Visual pipelines need image geometry, model version, and DPI to be bound to the action; structured UIA actions need only a fresh element identity. Aladdin already binds frames, proposals, and coordinates to generations (SG-000033 to SG-000036). | Aladdin canonical notes; parser source |

### E.3 Dispositions

| Capability | Decision | Rationale |
|---|---|---|
| `CacheRequest` batching for observation | ADAPT (pattern) into the Go host's `observe_window` | Measured 3.6× on a .NET client; also yields a consistent point-in-time snapshot. HYPOTHESIS for the Go host until benchmarked. |
| `RootWebArea` web observation and semantic actions in the user's Edge | REUSE existing Aladdin UIA registry | No CDP port, extension, or cookie access; existing password-target denial and protected-surface rules apply. |
| UI-TARS action parser | ADAPT (as planned in SG-000104) | Must carry model version, image size, and scale factor into the proposal; out-of-frame coordinates fail closed. |
| Windows-MCP / UFO executors | REFERENCE | Python runtimes; no per-action authority. |

### E.4 WebMCP availability (VERIFIED-REA, R9)

Edge 155 accepted the CDP `WebMCP.enable` command (`status: available`), but a page that registers a tool only through the native API did not find `document.modelContext` or `navigator.modelContext` (default flags, headless), and REA discovered 0 tools. WebMCP must remain an optional capability. Playwright's integration (Section F) works as soon as browsers expose it.

---

## F. Browser automation investigation

### F.1 Packages examined

| Package | Version | License | Notes |
|---|---|---|---|
| `@playwright/mcp` | 0.0.83 (`gitHead f183dad4…`), depends on `playwright`/`playwright-core` 1.64.0-alpha-1790635538000 | Apache-2.0 | Tool code in `playwright-core/lib/coreBundle.js` (3.57 MB bundle) |
| `chrome-devtools-mcp` | 1.10.1 (`gitHead e52c6b59…`) | Apache-2.0 | 15 MB installed |
| `@browserbasehq/stagehand` | 4.2.0 | MIT | Ships an MV3 extension runtime (`dist/extension/service-worker.js`, 2.08 MB) |
| `agentql` | 1.18.1 | MIT | See D |

### F.2 Measurements (VERIFIED-RUNTIME, local fixture, installed Edge 155, n = 1 unless stated)

| Metric | Playwright MCP 0.0.83 | Chrome DevTools MCP 1.10.1 | Desktop Commander 0.2.52 | Aladdin `main` |
|---|---:|---:|---:|---:|
| Tools | 25 | 30 | 26 | 40 |
| `tools/list` bytes | 20,296 | 26,398 | 57,591 | 36,093 |
| Startup network observed by preload tracer | none | none | flags + Chrome download | not traced |

Playwright MCP scenario on the fixture (form, Shadow DOM component, iframe, delayed list, fetch-driven status, explicit WebMCP shim):

| Step | Latency | Response bytes |
|---|---:|---:|
| `browser_navigate` | 1,065 ms | 393 |
| `browser_wait_for` "Row 39" | 866 ms | 358 |
| `browser_snapshot` (full page) | 30 ms | 2,279 |
| `browser_click` Shadow DOM button | 619 ms | 325 |
| `browser_type` | 31 ms | 100 |
| `browser_click` Save | 1,123 ms | 323 |
| `browser_wait_for` "Saved 42" | 47 ms | 362 |
| `browser_snapshot` (subtree, depth 1) | 40 ms | 366 |
| `browser_take_screenshot` | 87 ms | 331 |
| `browser_close` | 54,668 ms | 110 |

Facts: Shadow DOM text and button visible (`ref=e11`); iframe contents visible with frame-scoped refs (`f1e1`–`f1e4`); the page tool appeared under "webmcp tools (page-provided, untrusted)"; the server sent `notifications/tools/list_changed`. The click response did not contain the resulting status text, so post-action verification needs an explicit wait or snapshot. `browser_close` took 54.7 s in this single run; the cause was not investigated and must be measured before relying on fast cleanup.

REA's own Playwright capture (R8) of the same page: plain accessibility text 1,177 B versus DOM 2,360 B, but the plain accessibility text omitted iframe content and had no element refs. The DOM capture's apparent "Shadow card" match came from inline script source, not from the rendered shadow tree (false positive).

### F.3 Source-verified mechanisms

| Mechanism | Evidence |
|---|---|
| AI-mode snapshots (`ariaSnapshot({ mode: "ai", depth, boxes })`), subtree targets, depth limits, bounding boxes, save-to-file to keep large pages out of model context | `coreBundle.js:65685-65705, 66601-66630` |
| Dialog and file-chooser states raced against snapshot capture | `coreBundle.js:66661-66672` |
| WebMCP listing and execution (`document.modelContext ?? navigator.modelContext`, `getTools`, `executeTool`/`invokeTool`), suppressed while a dialog is open | `coreBundle.js:65940-65990, 66647-66657` |
| Existing-browser mode via the official Playwright extension (id `mmlmfjhmonkocbjadbfplnigmagldckm`) detected in the user's profile | `playwright-core/lib/tools/utils/extension.js` |
| Chrome DevTools MCP usage statistics enabled by default, opt-out `--no-usage-statistics`; sent via a watchdog process to Clearcut | `README.md:41-53`; `build/src/index.js:43, 231-233`; `build/src/telemetry/*` |
| Stagehand act/observe/extract cache: key = instruction + options; value = list of selector actions; replay without a model call | `dist/extension/service-worker.js:18555-18620` |
| Stagehand cache storage is the hosted Stagehand API (`POST /cache/get`, `/cache/set`) | `service-worker.js:18401-18404` |
| Stagehand self-heal re-runs a failed action with a newly inferred selector after any non-timeout error, including side-effecting methods | `service-worker.js:51473-51530` |
| Hosted endpoints referenced by Stagehand: `api.stagehand.browserbase.com` (and regional), `api.browserbase.com`, direct model APIs | string inventory of `dist/` |

### F.4 Recommendation: reuse versus a new Rust CDP engine

| Option | Integration effort | Capability at day 1 | Costs and risks |
|---|---|---|---|
| WRAP `@playwright/mcp` / `playwright-core` as a private, non-caller-facing sidecar driven by `qdrald`, launching the installed Edge with an isolated or user-selected profile | Low: the same stdio pattern Aladdin uses for its hosts; map 6 to 10 browser tools onto existing SG-000021 to SG-000026 contracts | Shadow DOM, iframes, refs, waits, forms, uploads, downloads, dialogs, screenshots, WebMCP, existing-browser extension mode | Node runtime (already required by Aladdin today); a large, fast-moving dependency to pin; `browser_run_code_unsafe` and `browser_evaluate` must never be mapped; close latency outlier to investigate |
| New Rust CDP client | High: months to reach parity on frames, shadow roots, waits, and file choosers | Minimal | Smaller install; full control |
| Stagehand extension | Medium | Strong for signed-in browsers | Powerful `debugger` permission; hosted cache by default; post-dispatch self-heal conflicts with Aladdin's no-retry rule |

Recommendation: WRAP Playwright first (this revises decision D4 in draft PR #283, which proposed a Rust CDP client). Expose only Aladdin-governed tools; keep origin, redirect, download, and upload policy in `qdrald`; disable code-evaluation tools; treat WebMCP tool metadata as untrusted input; re-evaluate a Rust engine only if Node removal becomes a hard requirement.

---

## G. Multi-computer investigation

### G.1 Sources

| Source | Identity | License | Method |
|---|---|---|---|
| MeshCentral server modules | npm `meshcentral@1.2.5`, `gitHead b09b1d60328de8e2d2f7387561ed51c64011c82a`; files fetched at that commit: `meshagent.js` (143,214 B, SHA-256 prefix `2832433e916a45a5`), `meshrelay.js` (94,868 B, `8846a4749628738b`) | Apache-2.0 | REA R4 + source |
| MeshAgent | `Ylianst/MeshAgent@26846b1c7f0841d71938f6bebc96eef5a947d217` | Apache-2.0 per README (no LICENSE file detected by GitHub) | Source review only |
| Desktop Commander remote device | 0.2.52 (`c774c3b`) | MIT | REA R1 + source |
| Aladdin relay and device identity | `61e664b` | Apache-2.0 | Source |

R4 found no child processes or requests in the two MeshCentral modules (they use the server's own transport objects), so the protocol findings below are source-verified.

### G.2 Findings

| Pattern | Evidence | Aladdin decision |
|---|---|---|
| Agent authentication bound to the TLS certificate the agent observed: agent sends SHA-384 of the server web certificate + 384-bit nonce; server rejects mismatches; each side signs `serverHash + agentNonce + serverNonce`; repeated handshake commands are blocked by a bitmask | `meshagent.js:444-520`, `:1157-1172` | ADAPT. Aladdin's device challenge-response (Ed25519) should include a channel binding (or end-to-end transcript hash) so a relay or proxy cannot splice sessions. |
| Relay rendezvous ids expire after 120 s; session cookies are encrypted with expiry (4 h for relay auth cookies) | `meshrelay.js:164-186, 357` | ADAPT (Aladdin already has expiring leases; align relay pairing ids). |
| Consent flags combine with bitwise OR: `mesh.consent | domain.userconsentflags`; optional privacy-bar text | `meshrelay.js:264-291` | ADAPT the rule "policy can only add consent requirements" for remote sessions; implement the on-screen indicator. |
| Desktop Commander "doorbell + durable claim": realtime broadcast `new_call`, row in `mcp_remote_calls`, claim with conditional update (`status = 'pending'`, `timeout_at > now`), in-memory recent-id set for duplicate delivery | `src/remote-device/remote-channel.ts:766, 841-975, 1205-1236`; `src/remote-device/device.ts:504-583` | ADAPT the claim pattern in Aladdin's relay so a dispatch is claimed exactly once and expired calls never run. |
| Desktop Commander executes the call when the claim write fails (comment: "execution proceeds despite the write error") | `remote-channel.ts:1215-1236` (fail-open at line 1228) | REJECT; Aladdin must fail closed (`not_started`). |
| Desktop Commander stores tool results and errors in the hosted `mcp_remote_calls` row | `remote-channel.ts:1240-1275` | REJECT for Aladdin; results travel end to end and are not persisted by the relay. |
| Heartbeat and presence: 25 s heartbeat, jittered presence retries, degraded-tier fallback | `remote-channel.ts:59-126` | REFERENCE for reconnection tuning. |
| UFO3 Galaxy AIP: WebSocket transport with optional `wss`, no device authentication primitive in the protocol package | `UFO/aip/transport/*` (previous report) | REFERENCE for DAG orchestration only. |

### G.3 Minimal path to two enrolled Windows computers

Aladdin already has device identity, pairing, OAuth, an outbound uplink, a self-hostable relay, and remote leases. The smallest increment is: (1) a controller role in the existing relay that routes one authenticated client to a named device; (2) the claim-once dispatch pattern with fail-closed behavior; (3) channel-bound device authentication; (4) a visible remote-session indicator and local kill switch on each target; (5) the existing `core` tool profile per device. No MeshCentral or RustDesk code is needed.

---

## H. Claude, ChatGPT, and proprietary computer use — boundaries

| Target | Status | Reason | What was used instead |
|---|---|---|---|
| Claude desktop app, Claude computer-use client components | NOT REA-ANALYZED | Anthropic's terms prohibit reverse engineering | Public API documentation (previous report: toolset `computer_toolset_20260801`, 17 actions, batch fail-fast rule, screenshot limits, prompt-injection classifiers) |
| ChatGPT and Codex desktop apps, ChatGPT agent, OpenAI computer-use service | NOT REA-ANALYZED | OpenAI's terms prohibit reverse engineering; hosted service | Public developer documentation (`computer` tool with ordered `actions` and screenshot outputs) |
| TinyFish hosted service, Browserbase/Stagehand API | NOT REA-ANALYZED | Hosted; no local artifact | Public SDKs and MCP proxies (D, F) |
| This research session's own executor | UNOBSERVED | Computer-use tools were not enabled in this session; only the browser pane and terminal were used | — |

Interaction pattern shared by all documented commercial loops: the model returns ordered actions, the client executes them and returns a fresh screenshot, and execution stops at the first failure. Aladdin's differentiator is to replace most of those screenshot turns with structured observations (UIA, accessibility snapshots) and to enforce authority locally.

---

## I. Source-reuse priority matrix

| Priority | Source @ revision | Files / feature | Reuse type | Dependencies | License considerations | Evidence |
|---|---|---|---|---|---|---|
| 1 | `@playwright/mcp@0.0.83` + `playwright-core@1.64.0-alpha-1790635538000` | Whole package as private sidecar; tools: navigate, snapshot, click, type, fill_form, select_option, press_key, wait_for, file_upload, take_screenshot, tabs, handle_dialog | WRAP | Node (present); installed Edge | Apache-2.0 notices; pin exact versions; never expose `browser_run_code_unsafe` or `browser_evaluate` | F |
| 2 | Aladdin Go host (own code) | `native_uia_windows.go` observation path | ADAPT pattern from Windows-MCP `tree/cache_utils.py` and UFO `inspector.py` | None | Pattern only (MIT sources) | E |
| 3 | Aladdin UIA registry (own code) | `RootWebArea` web observation and semantic actions in Edge | REUSE AS-IS (policy review) | None | — | E |
| 4 | Desktop Commander `rc-v0.3.1@7fd4898` | `src/utils/atomic-write.ts`, `src/utils/rename.ts` | ADAPT (port to Rust) | None | MIT notice if code is translated closely | C |
| 5 | Desktop Commander `c774c3b` | `src/remote-device/remote-channel.ts` claim logic | ADAPT (pattern, fail-closed) | None | MIT, pattern only | G |
| 6 | MeshCentral `b09b1d6` | `meshagent.js` handshake; `meshrelay.js` consent and expiry | ADAPT (pattern) | None | Apache-2.0, pattern only | G |
| 7 | Desktop Commander `c774c3b` | `src/tools/edit.ts` near-miss diff response | ADAPT | None | MIT, pattern only | C |
| 8 | Stagehand 4.2.0 | Instruction-to-action cache with re-verification | ADAPT (local cache, no hosted storage, no post-dispatch self-heal) | None | MIT | F |
| 9 | `@ui-tars/action-parser@1.2.3` | `dist/actionParser.js` | ADAPT (SG-000104) | None | Apache-2.0 | E |
| 10 | AgentQL SDK 1.18.1 | `page-monitor.js` readiness heuristics; `generate-accessibility-tree` snippet | REFERENCE (Playwright waits suffice) | — | MIT | D |
| — | Desktop Commander terminal and process-tree code | `terminal-manager.ts`, `process-tree.ts` | REJECT | — | — | C |
| — | Desktop Commander flags, onboarding, telemetry, Chrome download, hosted remote | various | REJECT | — | — | C, G |
| — | `tf-playwright-stealth` | bot-detection evasion | REJECT | — | — | D |
| — | cua perception extension (OmniParser) | — | REJECT (AGPL-3.0) | — | — | previous report |

---

## J. Security and reliability findings

### J.1 Confirmed facts

| ID | Finding | Class | Evidence |
|---|---|---|---|
| S1 | DC injects vendor imperative text into tool results when a remote flag is on | Model-steering design behavior (third-party) | C.4 runtime |
| S2 | DC downloads a browser at startup when Google Chrome is absent; Edge is not recognized | Undisclosed network and disk side effect (third-party) | C.4 runtime + source |
| S3 | DC terminal children inherit the full parent environment; `ssh` commands are rewritten | Ambient-credential exposure to model-chosen commands (third-party) | C.3 source |
| S4 | DC remote claim fails open on database errors; results persisted in hosted rows | Duplicate side-effect risk; data retention (third-party) | G.2 source |
| S5 | DC logs every tool call's arguments locally (`claude_tool_call.log`) and fuzzy-search text (`fuzzy-search.log`) | Local data retention (third-party) | C.3, C.4 |
| S6 | Stagehand self-heal re-executes failed actions with a new selector | Possible double execution (third-party) | F.3 source |
| S7 | Chrome DevTools MCP usage statistics are on by default | Telemetry default (third-party) | F.3 documented + source |
| R1 | **Aladdin `fs_write` is not crash-atomic** (truncate then write) | Reliability gap in Aladdin (not a security vulnerability); a crash between truncate and write leaves an empty or partial file | `crates/qdral-provider-fs/src/lib.rs:286-293` |
| R2 | AgentQL readiness loop iterates indices instead of URLs | Defect (third-party, functional) | D.2 |

### J.2 Hypotheses requiring measurement

| ID | Hypothesis | How to test |
|---|---|---|
| H1 | `CacheRequest` batching makes Aladdin's `observe_window` several times faster | Benchmark Go host on Notepad, Explorer, Settings, VS Code, Edge at the 256-element cap, before and after |
| H2 | Wrapping Playwright reduces browser task model turns versus screenshot loops | Aladdin Native Windows Suite browser subset with the same planner |
| H3 | Playwright `browser_close` latency outlier (54.7 s) is reproducible with Edge on Windows | 20 repetitions, headless and headed, isolated and persistent profiles |
| H4 | UIA `RootWebArea` path is sufficient for common signed-in sites | Measure task success on owned test accounts only |

### J.3 REA false positives, false negatives, and limitations observed

| Type | Observation |
|---|---|
| False positive | "Endpoint" nodes created from generic `.get("…")` calls (`truncate`, `format`, `className`) in the unscoped earlier run; in the scoped run, the five remaining "endpoints" were feature-flag names, not network endpoints |
| False positive | DOM capture matched "Shadow card" through inline script text |
| False negative | No request nodes for `axios.post(...)` to `api.agentql.com` (request extraction covers `fetch`, `WebSocket`, `node:http(s)`) |
| False negative | `process.env[AGENTQL_SERVER_URL_ENV_VAR]` default URL not reported as a configuration source (computed key) |
| Limitation | Literal-seed semantic trace returned zero matches for a string passed as a call argument |
| Limitation | Application feature trace reached the whole graph at module granularity (not data flow) |
| Limitation | Version comparison left 1,277 of 1,387 entities unmatched because identity keys include the artifact root digest; asset-level `changed` results were the reliable signal |
| Limitation | Whole-package analysis of `node_modules` exhausted the default heap; output size of hundreds of MB per run |
| Limitation | No native Windows decompilation (no Ghidra/JDK 21); no Windows process capture |

---

## K. Performance opportunities

| Opportunity | Evidence | Expected benefit | Status |
|---|---|---|---|
| Batch UIA reads with `CacheRequest` | 3.6× median on a .NET client (E.2) | Faster `desktop_window_tree`; fewer partial snapshots | HYPOTHESIS for Go host (H1) |
| Accessibility snapshots instead of screenshots for web pages | 2,279-byte snapshot in 30 ms with refs (F.2) versus roughly 1,000 to 1,800 input tokens per screenshot for Claude (DOCUMENTED) | Fewer model tokens and turns | HYPOTHESIS (H2) |
| Subtree and depth-limited snapshots; save-to-file for large pages | Subtree snapshot 366 bytes (F.2) | Smaller context per step | Measured on fixture |
| Keep tool schemas compact | Aladdin 36,093 B for 40 tools versus DC 57,591 B for 26 tools (measured) | Lower fixed context cost per session | Measured |
| Local instruction-to-action cache with revalidation | Stagehand design (F.3) | Skip model calls on repeated workflows | HYPOTHESIS; design must re-verify target identity and policy before replay |
| Never auto-download browsers; use installed Edge | DC behavior (C.4) | Avoids 100+ MB background downloads and startup network | Measured on DC |
| Explicit post-action waits | Click response lacked new state (F.2) | Correctness of verification rather than speed | Measured |

Benchmark requirements before any claim: pinned versions, same planner model, same fixtures and Windows image, at least 10 repetitions with medians and p95, and raw logs (per the benchmark plan in PR #283).

---

## L. Top 10 reusable capabilities for Aladdin

Ranked by expected user benefit, evidence strength, and integration effort.

| Rank | Capability | Source | Benefit | Evidence strength | Effort |
|---:|---|---|---|---|---|
| 1 | Governed browser automation by wrapping Playwright MCP behind `qdrald` | `@playwright/mcp@0.0.83` | Real web automation now (Shadow DOM, iframes, forms, waits, WebMCP) | Runtime-measured | Low–medium |
| 2 | Signed-in Edge control through the existing UIA registry (`RootWebArea`) | Aladdin + Windows-MCP finding | Works with the user's sessions without debugging ports or cookie access | Runtime-observed | Low |
| 3 | UIA `CacheRequest` batching in the Go host | Windows-MCP, UFO | Faster, consistent desktop observation | Microbenchmark | Low |
| 4 | Crash-safe atomic file writes | DC `atomic-write.ts` | Prevents empty or partial files after crashes | Source-verified gap | Low |
| 5 | Claim-once remote dispatch with fail-closed semantics | DC remote claim pattern | No duplicate remote side effects | Source-verified | Medium |
| 6 | Channel-bound mutual device authentication; additive consent; expiring rendezvous | MeshCentral | Stronger relay security for multi-device | Source-verified | Medium |
| 7 | Near-miss edit diff feedback | DC `edit.ts` | Fewer failed edit turns | Source-verified | Low |
| 8 | Local instruction-to-action cache with revalidation | Stagehand pattern | Fewer model calls on repeated tasks | Source-verified design | Medium |
| 9 | Version-bound coordinate parsing for visual models | UI-TARS parser | Correct clicks across model versions and DPI | Source-verified | Low |
| 10 | Asynchronous run model (`run`, `status`, `cancel`, progress) | TinyFish hosted tool shape | Long tasks without blocking clients | Documented | Low |

---

## M. Final independent engineering opinion

1. **Most reusable engineering for Aladdin:** Microsoft Playwright (through Playwright MCP). It is permissively licensed, maintained, measured working on this host against Shadow DOM, iframes, dynamic content, and WebMCP, and it maps cleanly onto Aladdin's existing browser contracts.
2. **Adopt first:** wrap Playwright MCP as a private browser sidecar under `qdrald`, with the installed Edge, exact version pins, and code-evaluation tools excluded. In parallel, add `CacheRequest` batching to the Go host and atomic writes to `fs_write`; both are small and independent.
3. **Do not copy wholesale:** Desktop Commander. Its terminal, process-tree, remote, telemetry, flags, onboarding, and startup-download code conflict with Aladdin's authority, privacy, and no-retry rules, and 68% of its shared core files changed within nine days.
4. **What REA revealed beyond README reviews:** the remote-flag configuration and process boundaries that led to confirming the onboarding injection at runtime; the change inventory between Desktop Commander releases (including the new process-tree and atomic-write modules); and REA's own blind spots (`axios`, computed environment keys) that make source re-verification mandatory.
5. **Most valuable Desktop Commander discovery:** remotely controlled `[SYSTEM INSTRUCTION]` text injected into tool results (confirmed at runtime). For Aladdin's own code, the most valuable discovery was the atomic-write pattern, which exposed that Aladdin's `fs_write` is not crash-atomic.
6. **Most valuable browser-automation discovery:** Playwright MCP already provides ref-based AI snapshots across Shadow DOM and iframes, plus WebMCP listing and execution, so a new Rust CDP engine is not on the critical path.
7. **Most valuable multi-device discovery:** MeshCentral's channel-bound mutual authentication and additive consent flags, combined with Desktop Commander's conditional claim, give a small, proven recipe for two-device control on Aladdin's existing relay.
8. **Impossible to verify here:** TinyFish's resolver and hosted browser internals; Stagehand's hosted cache service; Claude and OpenAI client executors; native Windows binaries (no Ghidra); Windows process capture via REA; the Go host's real UIA latency (no Go toolchain); Desktop Commander's live production flag values (only a local flag server was used).
9. **Immediate improvements:** atomic `fs_write`; UIA `CacheRequest`; a written rule that no tool result may contain vendor-controlled or remotely configured text; never auto-download browsers; prefer installed Edge.
10. **If engineering time were extremely limited, prioritize:** (a) Playwright MCP sidecar for governed browser automation; (b) UIA `RootWebArea` plus `CacheRequest` for fast native and signed-in web control; (c) claim-once, fail-closed remote dispatch with channel-bound device authentication for the second computer.
