# REA Investigation Report

Status: RESEARCH EVIDENCE. Date 2026-10-09. Host: Windows 11 Home 10.0.26300, x64, Node.js 24.19.0, JDK 17.0.20.1.

This report records only what was actually executed. Reconstructed or static results are labeled as static; no runtime behavior is claimed from static evidence.

## 1. REA identity and reconciliation

| Item | Value | How verified |
|---|---|---|
| Package | `rea-agents@6.1.0` | `npm view rea-agents version` |
| npm integrity | `sha512-xsXFiXEt2hc5ts+zX3D8zdyZHK5Glbp8GFYSIHMZytDiUCGAQp2crDmkoyiXNNbXXcvem228h+k54xUOYCnI7Q==` | Recomputed locally from the downloaded tarball; matches the registry value exactly |
| Tarball SHA-256 | `f997f603b3f1fb8f06593ef2d57989e43cddbd7526c45bcec92ce61d2873ad31` | `sha256sum rea-agents-6.1.0.tgz` |
| Published `gitHead` | `ae9aaee16b9c738d761d3938a3b34fdf37e581b5` | `npm view rea-agents gitHead` |
| GitHub tag `rea-agents-6.1.0` | `ae9aaee16b9c738d761d3938a3b34fdf37e581b5` | `gh api repos/morluto/rea/git/ref/tags/rea-agents-6.1.0` |
| GitHub `main` at research time | `cf415619cadcfcb18cda59a54b26c9d164f750a4` (ahead of the release) | shallow clone |
| License | MIT, "Copyright (c) 2026 morluto" | `LICENSE` |
| Engines | Node `^22.19.0 || ^24.11.0 || >=26.0.0` | `package.json` |

Installation scope: the exact tarball was installed into an isolated scratch prefix. `rea setup` was not run, no global install was made, no MCP client configuration was changed, Hopper/Ghidra/IDA were not installed, and no paid service was used.

## 2. Readiness diagnostics (`rea doctor --json`, `rea capabilities --json`)

| Check | Result |
|---|---|
| node | healthy (24.19.0) |
| host | healthy (win32 x64) |
| hopper | `missing_analysis_engine` |
| ghidra | `missing_analysis_engine` ("GHIDRA_INSTALL_DIR is not set"; REA requires an extracted Ghidra 12.1.x) |
| ida-registration | `missing_analysis_engine` |
| skill and client registrations | `config_drift` for every client (no REA setup performed; not changed) |
| Overall | `healthy: false` |

Available on this host without a native engine (from `rea capabilities`): `inspect_artifact`, `extract_artifact`, `inspect_managed_artifact`, `inspect_managed_members`, `inspect_managed_native_boundaries`, `decode_interface_builder`, `inspect_keyed_archive`, `trace_dylib_resolution`, plus the JavaScript/Electron application analyzer. Unavailable: all macOS-only native operations and native decompilation.

Native Windows decompilation gate: requires Ghidra 12.1.x and JDK 21+; this host has JDK 17 and no Ghidra. Installing them is a major tool installation and was not performed. Status: BLOCKED pending explicit approval.

## 3. Targets, scope, and results

### 3.1 Desktop Commander 0.2.52 installed package (MIT)

| Attempt | Scope | Command | Result |
|---|---|---|---|
| 1 | Whole installed package including `node_modules` (17,510 files) | `rea analyze-javascript-application <global npm path>/@wonderwhy-er/desktop-commander --json` | FAILED: "FATAL ERROR: ... JavaScript heap out of memory" after 412 s at file 685 of 17,510 (default heap) |
| 2 | Copied `dist/` and `package.json` only (238 files, 120 `.js`) | `NODE_OPTIONS=--max-old-space-size=6144 rea analyze-javascript-application <copy> --json` | SUCCEEDED in 3 min 50 s; evidence JSON 504,771,833 bytes; evidence id `ev_9f5faaf3c572c72efa2a5cf9e2a1f2d5b2d4e3b0a18fd91d1f88d9d811f4cf7a`; root artifact SHA-256 `a7bcd10ec138ca999eff03905e6188e4ab24feba971a6e08b5b02c51ed51a70a` |

Static statistics (attempt 2): 234 JavaScript files parsed, 525,614 AST nodes visited, 0 parse failures, 1,652 findings; application graph 1,293 nodes and 2,179 edges; semantic graph truncated at its 100,000-node ceiling with 70,717 relations and 46,777 recorded unknowns (mostly dynamic calls).

Confirmed static findings (each re-verified by reading the corresponding TypeScript source at `ea3ed35`):

| Finding | REA evidence | Source confirmation |
|---|---|---|
| Remote feature-flag fetch | `config-source DC_FLAG_URL` default `https://desktopcommander.app/flags/v2/production.json` (`dist/utils/feature-flags.js:20`) | `src/utils/feature-flags.ts:30` |
| Remotely gated model-visible onboarding injection | flag keys `onboarding_injection`, `user_surveys`, `experiments`, `welcome_page_enabled` | `src/utils/usageTracker.ts:406-450` (`shouldShowOnboarding`) |
| Worker created from an eval string | `new Worker(WORKER_CODE, { eval: true, ... })` | `src/tools/fuzzySearch.ts:44` |
| Child processes | 8 `child-process` nodes: ripgrep search spawn, terminal spawn (`improved-process-tools.js:28`), remote-device spawn of the local MCP server, `npm --version` exec in setup/uninstall | `src/search-manager.ts`, `src/terminal-manager.ts`, `src/remote-device/*` |
| Install tracking reads CI and IDE environment | `GITHUB_ACTIONS`, `CIRCLECI`, `JOB_NAME`, `VSCODE_PID`, `TERM_PROGRAM` reads in `track-installation.js` | install telemetry scripts |
| Two bundled UI renderers | `dist/ui/file-preview/index.html`, `dist/ui/config-editor/index.html` | MCP app UI resources |

REA limitations observed:

- Memory: whole-package analysis needs a narrowed scope or a larger heap; the default heap failed.
- Output size: 505 MB evidence for a 2.9 MB input makes downstream review expensive.
- Noise: 15 "network endpoint" nodes were mostly false positives produced from generic `.get("...")` calls (`truncate`, `format`, `className`).
- Static only: no runtime execution was performed; nothing here proves a code path runs.

### 3.2 Targets not analyzed with REA, and why

| Target | Reason | Alternative used |
|---|---|---|
| Claude desktop app and its computer-use components | Anthropic's terms prohibit reverse engineering | Public API documentation; visible tool schemas |
| ChatGPT / Codex desktop apps | OpenAI's terms prohibit reverse engineering | Public documentation; open-source Codex CLI not needed for this scope |
| TinyFish hosted service | No local artifact; hosted SaaS | Public repository (examples only), public pricing |
| UI-TARS desktop binaries | Not installed locally; downloading was not authorized | Source reading per P20 ledger |
| Aladdin Go computer host | Go toolchain absent; source available anyway | Direct source reading |
| Native Windows binaries (any) | No Ghidra/Hopper/IDA | n/a (BLOCKED) |
| Open Computer Use, UFO, cua, Windows-MCP, browser-use, Stagehand, Playwright MCP | Source available; REA adds nothing over reading source | Direct source reading |

## 4. Per-target record (template applied)

| Field | Desktop Commander 0.2.52 |
|---|---|
| Version and artifact | npm package installed globally on this host, `package.json` version 0.2.52; analyzed directory SHA-256 `a7bcd10e...` |
| Scope and rights | MIT; local installed artifact owned by the user |
| Tools executed | `rea doctor`, `rea capabilities`, `rea analyze-javascript-application` (twice) |
| Methods | Static JavaScript application and semantic graph |
| Confirmed | Flags endpoint, onboarding injection gate, eval worker, process spawns, install tracking |
| Inference | The feature-flag service can change model-visible output for installed users without a release (inferred from the code path; not observed at runtime) |
| Unknowns | 46,777 unresolved semantic relations; runtime flag values |
| Strengths | Breadth of developer tools; fuzzy edit; streaming search; PDF support |
| Weaknesses | Ambient authority; telemetry default on; remote vendor control of model-visible content |
| Reusable patterns | Edit/search/pagination UX |
| Legal | MIT notice preservation |
| Aladdin decision | ADAPT edit/search/rich-file behaviors; REJECT flags, telemetry, onboarding, hosted remote |

## 5. Using REA in the Aladdin program

- REA is an offline developer research tool. It never participates in runtime authorization and is never shipped.
- Recommended next REA tasks, each needing explicit approval: (a) install Ghidra 12.1.x and JDK 21 in an isolated directory to analyze Aladdin's own release binaries for unexpected imports; (b) run `analyze-web-bundle` and `discover-webmcp-tools` against owned test pages for the browser engine work; (c) run `inspect-artifact` on each Aladdin release candidate to produce an independent component inventory for the size budget.
