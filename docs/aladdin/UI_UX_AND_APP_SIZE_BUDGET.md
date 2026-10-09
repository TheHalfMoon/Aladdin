# Aladdin App — UI/UX and Size Budget (Proposed)

Status: PROPOSED PLANNING ONLY. There is no Aladdin desktop GUI on `main`; approvals use `MessageBoxW` (SOFT) and Windows Hello `UserConsentVerifier` (STRONG). Draft PR #282 (opened 2026-10-09 during this research) adds a read-only WPF preview app (`apps/aladdin-desktop-preview`) built with the in-box .NET Framework compiler; its description reports a 22,016-byte executable (DOCUMENTED by the PR, not re-measured here).

## 1. Stack decision

The UI must be a replaceable client of a stable local app API exposed by `qdrald` (status, task events, intents, device list). If that API is the contract, the UI toolkit becomes a reversible choice.

| Option | Download size | Fit | Decision |
|---|---|---|---|
| WPF on in-box .NET Framework 4.8.x (PR #282) | Smallest possible: no runtime to ship; tens of KB for the executable (DOCUMENTED in #282) | Windows-only; mature accessibility and RTL support; legacy framework with no new feature investment; a third language (C#) next to Rust/Go/TypeScript | ACCEPTABLE for the Windows MVP if the app stays a thin client |
| Tauri 2 + system WebView2 (Rust backend) | Small: WebView2 ships with Windows 11; app binary typically single-digit MB (ESTIMATE) | Same language as `qdrald`; cross-platform path to macOS/Linux; the founder's Winds project uses Tauri 2 | RECOMMENDED when macOS/Linux enter scope, or now if one UI codebase across platforms is preferred |
| Electron | Bundles Chromium and Node (typically well over 100 MB installed) | Large; duplicates the browser runtime | REJECT |
| WinUI 3 / Windows App SDK | Requires the Windows App SDK runtime | Windows-only, heavier than WPF on in-box .NET Framework | REJECT for v1 |
| Existing web stack in `apps/web` | Marketing site only | Not an app | n/a |

Decision rule for WP-08: freeze the local app API first; then choose WPF (smallest, Windows-only) or Tauri (cross-platform) by measuring installed size, idle memory, RTL/Arabic rendering quality, and accessibility on the same screens.

The app is a client of `qdrald`, never an authority. The Tauri command allowlist exposes only: connect to local `qdrald`, read status and task events, submit user intents, and open approval dialogs that `qdrald` itself renders. Approval dialogs must remain owned by `qdrald` (or a dedicated, protected approval process) so that a compromised web UI cannot click "Approve".

## 2. Screens and components

| Element | Behavior |
|---|---|
| Floating compact window | Chat input, push-to-talk button, device selector, current-task chip, Stop button; always-on-top optional |
| Expanded workspace | Task timeline (observe -> propose -> approve -> execute -> verify), observation preview (cropped, redacted), receipts, files, sessions |
| Device dashboard | Online state, active leases, queued tasks, last receipt per device, per-device kill switch |
| Approval dialog (protected) | Exact action, target, digest summary, consequence class, device; Approve once / Deny; STRONG via Windows Hello |
| Human takeover | "Take over" pauses the agent and revokes the input lease; physical input already invalidates leases (SG-000037 interruption epoch) |
| Stop | Global hotkey and button: emergency revoke of all leases, cancel queued work, release held keys |
| Privacy indicator | Shows whether the current step sends data to a model (Hala One, BYO provider) or stays local |
| Remote indicator | Screen-edge border plus tray badge whenever a remote session or remote lease is active |
| Settings | Profiles (Safe, Full User), workspaces, MCP client connections, devices, voice, privacy and retention |
| Error recovery | Every `outcome_unknown` shows what is known, the last observation, and the safe next steps |

Arabic and English UI with right-to-left layout support from the start.

## 3. Size budget

### 3.1 Measured inputs (VERIFIED on this machine, 2026-10-09)

| Artifact | Bytes | gzip bytes | Command |
|---|---:|---:|---|
| `qdrald.exe` (release build, toolchain 1.97.1) | 3,708,928 | 1,340,870 | `cargo build --release -p qdrald -p qdral-browser-host`; `ls -la`; `gzip -c | wc -c` |
| `qdral-browser-host.exe` (release) | 488,960 | n/m | same |
| Published release zip `cotra-0.1.0-windows-x64.zip` | 4,839,256 | (zip) | `gh release view v0.1.0` |
| External Node.js runtime `node.exe` 24.19.0 (required today, not bundled) | 92,825,416 | 34,716,108 | `ls -la`; `gzip -c | wc -c` |
| `deskal-computer-host.exe` (Go) | UNMEASURED (Go toolchain not installed) | | |

### 3.2 Budget (targets; items marked ESTIMATE must be measured)

| Component | Download (compressed) | Installed | Notes |
|---|---:|---:|---|
| `qdrald` + providers | 1.4 MB | 3.8 MB | Measured basis |
| Computer Host (Go) | 2.5 MB ESTIMATE | 6 MB ESTIMATE | Measure with `-trimpath -ldflags "-s -w"` |
| Browser engine (Rust CDP client, D4) | 1 MB ESTIMATE | 3 MB ESTIMATE | Uses installed Edge/Chrome; no bundled Chromium |
| Shell/PTY host (Rust, D5) | 0.5 MB ESTIMATE | 1.5 MB ESTIMATE | |
| MCP edge, option A: Rust (D7) | 1 MB ESTIMATE | 3 MB ESTIMATE | Removes Node entirely |
| MCP edge, option B: Node bundled | 35 MB | 93 MB + JS | Measured Node size; only if D7 is rejected |
| App UI: Tauri + assets, or WPF (PR #282) | Tauri 4 MB ESTIMATE; WPF under 1 MB | Tauri 12 MB ESTIMATE; WPF under 2 MB | WebView2 and .NET Framework are OS-provided on Windows 11 |
| Voice client | 0 | 0 | Server-side STT/TTS; only Opus encoding (OS or small library) |
| Notices, SBOM, manifests | 0.5 MB | 1 MB | Measured release notices are about 0.37 MB |
| Total with option A | about 11 MB | about 30 MB | Far below the 400 MB ceiling |
| Total with option B | about 45 MB | about 120 MB | Still below the ceiling |

Runtime disk: task cache capped (default 500 MB, user-configurable) with LRU eviction; receipts small; no model weights on device. Memory target on a 16 GB machine: idle under 150 MB across all Aladdin processes, active under 400 MB excluding the user's own browser.

Update size: delta updates per component (each host is a separate signed binary), so a typical update downloads one or two binaries (1 to 5 MB).

### 3.3 Reproducible measurement procedure

1. Build each component in release mode from a clean checkout at an exact commit with pinned toolchains (`rust-toolchain.toml`, `go.mod` toolchain, `package-lock.json`).
2. Record `sha256`, raw size, and `gzip -9` size for each artifact in `size-report.json`.
3. Build the installer; record signed installer size and unpacked installed size (`Get-ChildItem -Recurse | Measure-Object -Sum Length`).
4. Measure runtime: start `qdrald` and the app, idle 5 minutes, record working set and private bytes per process (`Get-Process`), then run the standard E2E task set and record peak values.
5. CI gate: fail the build if any component grows more than 10% or the total exceeds the budget without an approved budget change.
6. Optionally run REA `inspect-artifact` on each release candidate for an independent component inventory.

Code signing: the project currently ships unsigned binaries (zero-cost rule). SmartScreen friction is a real adoption cost; record "signing certificate or Microsoft Store/MSIX distribution" as an open founder decision.
