# Qdral Governed Computer-Use Successor Plan

Status: IMPLEMENTATION-READY SUCCESSOR PLAN
Planning date: 2026-10-03
Prerequisites: close SG-000066 / COTRA-P16, then complete COTRA-P17 (SG-000067 through SG-000072) before activating QDRAL-P18 implementation grains.
Pinned donor study input: `bytedance/UI-TARS-desktop@2ff41a9e515828c5bd5b276e493d73aa0bdf4a3a`.

## 1. Product decision

Qdral will use selected UI-TARS Desktop subsystems as donor/reference material for a governed computer-use engine. Qdral does not become a UI-TARS fork and UI-TARS does not become part of Qdral's security boundary.

Qdral remains the sole authority for policy, trust, approval, typed target identity, lease issuance and consumption, destination policy, audit, revocation, tool exposure, and remote reach.

UI-TARS-derived code or concepts may implement low-authority browser, capture, parsing, rendering, or execution backends only after a separate SpecGrain authorizes the exact authority delta and the backend is qualified against Qdral's existing contracts.

## 2. Non-negotiable invariants

- Model output is untrusted input and never grants authority.
- No model/provider adapter may mint approvals, trust, workspace authority, target identities, capture leases, input leases, or remote-computer leases.
- No arbitrary JavaScript, generic CDP/DevTools tool, unrestricted shell, PowerShell/cmd, generic script execution, generic socket/proxy, personal browser profile, silent elevation, credential/security-dialog automation, remote approval, or model self-approval is introduced.
- No browser, desktop, screenshot, or computer-use capability is exposed before its live provider implementation is qualified shape by shape.
- Structured browser/UIA actions are preferred over coordinate input. Coordinate input is an explicit lower-ceiling fallback and never a silent fallback.
- Mutating operations are never automatically retried after dispatch.
- Lock, logoff, session transition, process/window/document generation drift, workspace revoke, policy revision drift, lease expiry, human interruption, or Qdral restart invalidate the affected volatile authority.
- All donor reuse preserves provenance, applicable license notices, modified-file notices where required, and third-party dependency obligations.

## 3. Donor reuse decisions

| UI-TARS subsystem | Default successor classification | Qdral rule |
| --- | --- | --- |
| Browser MCP/Puppeteer implementation | ADAPT / PORT | Browser engine only; never a second MCP authority edge |
| `@ui-tars/action-parser` | ADAPT | Syntax-to-proposal adapter only; cannot authorize execution |
| Operator abstraction | PORT CONCEPT | Split observation/proposal/validation/execution instead of direct model-to-execute |
| NutJS desktop operator | REFERENCE_ONLY | Raw model-to-mouse/keyboard path is not imported as authority |
| Screenshot/DPI handling | PORT_SELECTED_LOGIC | Capture must be exact-window scoped and protected-surface aware |
| RemoteComputerOperator | REFERENCE_ONLY | Qdral remote principal/device/session/lease model remains authoritative |
| Event stream architecture | ADAPT CONCEPT | Presentation/telemetry layer over canonical Qdral audit, never the audit source of truth |
| Electron IPC/UI patterns | REFERENCE_ONLY | Qdral renderer remains sandboxed, context-isolated, and low authority |
| Agent loop/model runtime | REJECT_FOR_CORE | Qdral is provider-neutral and does not delegate authority to a donor agent loop |
| `browser_evaluate` / arbitrary JS | REJECT | No caller-facing arbitrary JavaScript |
| `run_command` / `run_script` | REJECT | Existing bounded process registry remains the only execution model |
| Personal browser profile | REJECT | Dedicated Qdral automation profile only |
| Raw model -> mouse/keyboard | REJECT | All execution flows through Qdral proposal, validation, approval, lease, and postcondition gates |

## 4. Target architecture

```text
AI client / model
       |
       v
Qdral MCP / provider adapter
       |
       v
Canonical ComputerActionProposal
       |
       v
qdrald policy kernel
  | trust / policy / approvals
  | typed identities / leases
  | destination policy / audit
  | revocation / remote reach
       |
       +--------------------+---------------------+
       |                    |                     |
       v                    v                     v
Browser provider      Desktop UIA provider   Coordinate provider
       |                    |                     |
       v                    v                     v
low-authority host    Win32/UIA/capture      bounded native input
       |
       v
Chromium/Edge automation profile
```

No donor backend receives direct MCP exposure. Qdral's tool contract remains the only tool-surface source of truth.

## 5. QDRAL-P18 grain sequence

The identifiers below are reserved for this plan only after confirming they remain unused when P18 is activated.

### SG-000073 — Computer-use architecture and provenance freeze

No authority delta. Pin donor revisions/paths/licenses, freeze the canonical action/target identity vocabulary, record reuse classifications, define process boundaries, and prove no donor dependency is reachable from the shipped runtime yet.

### SG-000074 — Live isolated browser-host foundation

Introduce a low-authority `qdral-browser-host` process. It is launched only by Qdral, communicates through a private local framed channel, has no independent MCP/HTTP/LAN listener, owns only a dedicated Qdral automation profile, uses a supported local Chromium-family executable, and never attaches to a personal profile.

Required defaults:

- ephemeral automation profile;
- browser sandbox enabled;
- no `--no-sandbox`;
- no remote-debugging TCP listener;
- no extension inheritance;
- no inherited unrelated secrets;
- bounded lifetime/resources;
- child termination on parent/session invalidation;
- all browser/page identities server allocated by Qdral.

### SG-000075 — Browser network mediation and real navigation

Make navigation real without converting Chromium into an unbounded network client.

Every top-level destination continues to use Qdral destination validation, address resolution, origin binding, redirect policy, fresh SOFT approval, and typed page generation. Browser subresource traffic is mediated by an egress policy that blocks private, loopback, link-local, metadata, disallowed schemes, external protocol handlers, and transport bypasses.

Explicitly cover redirects, iframes, workers/service workers, fetch/XHR, WebSockets, DNS rebinding, QUIC/DoH/WebRTC bypass paths, `file:`, `javascript:`, browser-internal URLs, and download-trigger behavior.

### SG-000076 — Live DOM and accessibility observation

Bind the closed `browser.snapshot/observe` contract to live DOM/Accessibility evidence. Preserve exact workspace/profile/page/origin/page-generation/document-generation binding, server-issued node identity, max node/depth/byte limits, password/secret redaction, and stale-node failure.

Raw JavaScript evaluation, generic selectors supplied as authority, cookie/session-store reads, generic CDP, and DevTools exposure remain denied.

### SG-000077 — Live structured browser actuation

Bind the closed structured click/fill contract to the live engine. Revalidate exact page/document/node/role/state immediately before execution, require fresh approval as already specified, consume one-shot approval before mutation, observe postconditions, and advance/invalidate generations deterministically.

No silent fallback to coordinates. Password-field fill remains denied.

### SG-000078 — Live bounded browser transfers

Connect the existing bounded download/upload contracts to the live engine without weakening them.

Downloads land first in protected Qdral staging, then pass destination, type, size, digest, same-origin/redirect, approval, and create-only workspace checks. Chromium never writes directly to arbitrary workspace paths and downloaded content is never auto-opened/executed/extracted.

Uploads remain restricted to server-recorded admissible artifacts; no caller-controlled arbitrary path is accepted. Exact file-input node identity, source digest/size/media type, workspace trust revision, approval, and one-shot source consumption remain mandatory.

### SG-000079 — Browser tool exposure and profile qualification

Expose only live-qualified shapes through the authoritative Qdral tool contract. Create an explicit `browser_structured` local profile rather than silently widening existing profiles. Remote mapping remains off until separately authorized.

Do not expose `browser_evaluate`, raw CDP, generic JS, personal-profile operations, generic browser filesystem, or raw browser process controls.

### SG-000080 — Exact-window capture

Implement Windows-first exact-window capture using a native API suitable for per-window capture. Bind capture to process instance, HWND/window identity, window generation, dimensions/DPI, session, policy revision, and a server-issued capture generation.

Deny Qdral approval/trust/revoke windows, Qdral-owned windows, CredentialUIBroker, consent/UAC, LogonUI, secure desktop, other sessions, stale/reused handles, and whole-screen capture.

Introduce a local-only `CaptureLease` for repeated observation. It is exact-window scoped, time bounded, revocable, cannot be minted through MCP/model input, and is invalidated by window/process replacement, lock/logoff/session transition, policy change, or Qdral restart.

### SG-000081 — Structured desktop actuation

Implement UI Automation actuation before coordinate input:

- InvokePattern;
- ValuePattern;
- SelectionItemPattern;
- TogglePattern;
- ScrollPattern.

Every action binds the exact process instance, window generation, element identity, expected role/state/pattern availability, workspace/policy revision, approval class, and postcondition. Password/secret elements and protected surfaces are denied.

### SG-000082 — Governed coordinate fallback

Enable the existing lower-ceiling coordinate design only after structured UIA and exact-window capture are live.

Every coordinate proposal binds one exact capture generation and target window. Every execution binds one exact coordinate/action, one fresh single-use input lease, one approval, and one execution. Human physical input revokes/cancels the active input lease. No silent carry-over between captures or resized/moved windows.

Initial verbs: click, double-click, right-click, bounded scroll, bounded Unicode text input. Clipboard-based typing, unrestricted hotkeys, arbitrary drag, and free-form input streams remain absent until separately authorized.

### SG-000083 — Canonical ComputerAction IR and model adapters

Define a provider-neutral `ComputerActionProposal` that carries only untrusted proposal data. Implement separately testable parsers/adapters for UI-TARS and other authorized providers as needed. The adapter may normalize action syntax but cannot create or override Qdral target IDs, approvals, leases, trust, policy, risk class, or execution outcome.

Malformed, ambiguous, unsupported, oversized, stale, or authority-bearing adapter fields fail closed.

### SG-000084 — Remote computer-use leases

Remote computer use remains disabled until an explicit locally authorized lease model is proven. A `RemoteComputerUseLease` binds the verified remote principal/provider, exact device, route/session epoch, workspace, locally selected windows/capabilities, expiry, and policy revision. It is created/revoked locally with the required presence class and cannot be self-granted remotely.

No background screen stream, surprise queued execution, remote approval delegation, or cross-provider/session lease reuse. Browser and desktop remote reach are separately scoped and default denied.

### SG-000085 — Security, failure, privacy, and performance qualification

Blocking qualification must include:

- DNS rebinding, redirect widening, popup/new-window escape, iframe/worker/service-worker traffic, WebSocket traffic, QUIC/DoH/WebRTC bypass, unsafe schemes, external protocol handlers, and browser permissions;
- browser/host crash, navigation/document/node drift, stale HWND/PID reuse, resize/move/DPI changes, multi-monitor behavior, RDP/session transition, lock/logoff, secure desktop and Windows Hello/UAC surfaces;
- human-input interruption, approval/lease replay, policy/workspace/trust/revoke drift, remote revoke/disconnect, oversized images/trees/results, malicious filenames, symlink/junction/reparse escape, password/secret fields, and model-supplied forged identities;
- bounded CPU/memory/process counts, screenshot dimensions/bytes/rate, DOM nodes/depth/bytes, browser tabs, pending transfers, event/result sizes, and timeouts;
- data-retention and log-redaction tests proving screenshot/browser/secret payloads do not enter logs or telemetry beyond the explicit bounded evidence contract.

Mutating operations are never auto-retried. Crash/restart invalidates affected volatile identities and leases.

### SG-000086 — P18 exit and v0.3 readiness

P18 exits only when all exposed browser/desktop/computer-use shapes are live-qualified on their target OS/runtime, every denied authority is regression-tested, exact tool/profile contracts match shipped discovery, and release artifacts pass the normal Qdral exact-head CI, security review, TypeSafe Jev, Alibaba Open Code Review, manual excluded-file review, SBOM/provenance, and post-merge verification requirements.

## 6. Tool-surface model after P18

Target profiles are explicit unions, not implicit widening:

- `core`: existing bounded cross-provider core;
- `desktop_observe`: structured local window/tree observation plus separately qualified capture;
- `browser_structured`: live qualified browser shapes only;
- `desktop_control`: structured UIA actuation only;
- `coordinate_fallback`: separately enabled lower-ceiling coordinate actions;
- `computer_use`: an explicit local union selected by local policy, never a remote default.

Unknown profiles fail closed. Adding a live provider implementation does not automatically expose its tools.

## 7. Browser engine packaging

Production must not run `npx ...@latest` or fetch runtime code dynamically. The selected browser-host code and dependencies are pinned and included in the release/SBOM. Prefer a supported locally installed Edge/Chromium-family executable on Windows with executable/path/publisher verification and a Qdral-owned profile. Unsupported/missing engines return a typed unavailable result; they never fall back to a personal profile.

`qdral doctor` should report browser executable identity/version, profile location/class, host version, protocol compatibility, and whether live browser capability is qualified and locally enabled.

## 8. Event/audit model

A presentation event stream may report bounded lifecycle events such as proposal creation, policy decision, approval request/result, lease issuance/consumption/revocation, observation generation, action start/result, postcondition result, interruption, and failure. Canonical security evidence remains Qdral's audit/evidence system. Event UI cannot grant authority or become the only source of security truth.

## 9. P18 definition of done

P18 is not complete merely when a click or screenshot works. Closure requires all authorized live browser, transfer, exact-window capture, structured UIA, coordinate fallback, model-adapter, and remote-lease contracts to be implemented and qualified; all explicitly denied authority to remain unreachable; Windows real-machine behavior to be proven where Windows-specific; exact-head CI and review gates to pass; and post-merge verification to confirm the canonical revision.
