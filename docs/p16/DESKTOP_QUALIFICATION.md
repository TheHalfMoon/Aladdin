# Structured Desktop Exposure Qualification (SG-000063)

Status: CANONICAL DECISION FOR QDRAL-P16

## Question

SG-000027 through SG-000038 closed a structured desktop layer: typed
process, window, tree, and element identities; structured invoke,
set_value, select, toggle, and scroll; window-scoped screenshot capture;
visual and coordinate proposals; and bounded input execution under a
single-use lease with human interruption. Until SG-000063 the native adapter
reported every live desktop read as unavailable, so none of it was exposed.
Before any shape becomes an MCP tool it must be live on the native adapter.
An MCP tool must not claim desktop capability the adapter does not have.

## What became live

`crates/qdral-provider-uia/src/native_desktop.rs` observes the caller's
interactive desktop read-only through Win32 and UI Automation:

- Window listing enumerates top-level windows with `EnumWindows` and keeps
  only windows that are visible, not DWM-cloaked, titled, and owned by
  another process in the caller's session (`ProcessIdToSessionId`).
- Process identity is the Win32 image path (`QueryFullProcessImageNameW`)
  and the creation time (`GetProcessTimes`). A process whose facts cannot be
  read is omitted, never given a fabricated identity.
- Tree reads bind one window with `ElementFromHandle` and walk the UIA
  control view with the registry's depth (8) and node (256) bounds plus a
  response byte ceiling, under UIA connection and transaction timeouts.
- Only the interactive window station `WinSta0` is observed. Anywhere else
  (a service session, a non-interactive runner) every read fails closed as
  unavailable.

It never sends input, window messages, or pattern invocations, never
focuses or activates a window, and never captures pixels.

## Exclusions

- Qdral's own windows: windows of the observing process (which hosts the
  SOFT approval prompts) and of `qdral.exe`, `qdrald.exe`, and
  `qdral-mcp-host.exe` are never reported, and a tree read of the observing
  process's window is denied.
- Windows security prompts: windows of `CredentialUIBroker.exe` (the
  Windows Hello dialog that shows Qdral's STRONG approval request),
  `consent.exe`, and `LogonUI.exe` are never reported, and a tree read is
  denied if a handle is owned by one of them at read time.
- Protected surfaces: windows whose title or class carries a protected Qdral
  approval, trust, or emergency-revoke marker are omitted from listings and
  counted in `protected_omitted`; tree reads of them are denied.
- Other sessions: windows owned by processes in another Windows session are
  never reported and their trees are denied.
- Password values: elements whose live `IsPassword` property is true (or
  unreadable) never have their value read; the registry additionally
  redacts elements carrying password or secret markers.
- Stale handles: before every tree read the registry re-lists the owning
  process's windows and requires the same handle with the same
  process-instance nonce (PID and creation time) and class; a destroyed,
  reused, or re-classed handle fails closed as `TARGET_STALE`.

## Per-shape decision

| Shape | Native adapter | Decision |
| --- | --- | --- |
| `uia.window/list` | live (read-only) | exposed as `desktop_window_list` |
| `uia.tree/observe` | live (read-only) | exposed as `desktop_window_tree` |
| `uia.process/observe` | live registry read | not a separate tool; facts are in `desktop_window_list` |
| `uia.window/observe` | live registry read | not a separate tool; facts are in `desktop_window_list` |
| `uia.element/observe` | live registry read | not a separate tool; facts are in `desktop_window_tree` |
| `uia.element/invoke` | not implemented | not exposed |
| `uia.element/set_value` | not implemented | not exposed |
| `uia.element/select` | not implemented | not exposed |
| `uia.element/toggle` | not implemented | not exposed |
| `uia.element/scroll` | not implemented | not exposed |
| `uia.screenshot/capture` | not implemented | not exposed |
| `uia.visual/propose` | not implemented (needs a live capture) | not exposed |
| `uia.coordinates/propose` | not implemented (needs a live proposal) | not exposed |
| `uia.input/execute` | not implemented | not exposed |

The decision is pinned by `DESKTOP_SHAPE_QUALIFICATIONS` in
`crates/qdral-provider-uia/src/lib.rs`, by `sg000063_tests.rs` (every
not-live shape fails closed as unavailable on the native adapter), and by
`apps/qdral-mcp/src/uia.test.ts` (only `desktop.ts` forwards UIA shapes, and
only the two live ones).

## Remote reach

Both desktop tools are local-only (`LOCAL_ONLY_TOOL_NAMES`). Window titles
and control trees of every application in the user's session are not
mapped to any remote OAuth scope: the relay edge and the device uplink deny
them as unmapped, and qdrald's remote-session scope table has no `uia.*`
entry, so a remote request fails closed even if it reached the kernel. A
later governed grain may map a locally enabled `desktop_structured` profile.

## Evidence

- `cargo test -p qdral-provider-uia sg000063` on real Windows 11 in an
  interactive session spawns a probe process with a plain text box, a
  system password box, and a second form titled as a Qdral approval
  surface. The live listing returns the probe with `powershell.exe` image
  identity and win32 creation-time generation, omits the protected form
  (`protected_omitted >= 1`), and the live tree returns the plain value,
  marks the password element `value_is_password` and `redacted` with a null
  value, and never contains the password text. A depth-0, one-node read is
  reported as truncated. A window this process creates is never listed and
  its tree read is denied.
- An end-to-end run through `node apps/qdral-mcp/dist/index.js` and a real
  `qdrald.exe` lists 28 tools, finds the probe through
  `desktop_window_list`, reads its tree through `desktop_window_tree` with
  the password value absent, rejects a drifted `window_generation` as
  `TARGET_STALE`, and rejects a malformed `window_id`.

## What remains true

- Structured actuation, screenshots, visual and coordinate proposals, and
  bounded input keep their registry controls (typed identity, approval
  binding, input leases, human interruption, protected-surface denial) as
  the specification a live native actuation adapter must satisfy. A future
  grain must implement and qualify each shape before exposure.
- Raw `SendInput`, keyboard or mouse injection, focus stealing, and process
  termination remain absent.


## SG-000095 successor update

SG-000063 remains the historical read-only qualification above. DESKAL-P20 SG-000095 is the separately governed successor that implements and qualifies the previously deferred local Full User desktop-control surface. The historical SG-000063 and SG-000066 evidence is not rewritten.

Current-tree exposure adds the following **local-only** MCP shapes:

| Shape | Current status | MCP tool |
| --- | --- | --- |
| `desktop.cursor/get` | Full User cursor observation | `desktop_cursor_get` |
| `uia.screenshot/capture` | exact target-window capture | `desktop_window_capture` |
| `uia.element/invoke` | semantic actuation | `desktop_element_invoke` |
| `uia.element/set_value` | semantic actuation | `desktop_element_set_value` |
| `uia.element/select` | semantic actuation | `desktop_element_select` |
| `uia.element/toggle` | semantic actuation | `desktop_element_toggle` |
| `uia.element/scroll` | semantic actuation, at most 20 ticks | `desktop_element_scroll` |
| `desktop.input/execute` | bounded real mouse/keyboard input | `desktop_input_execute` |
| `desktop.window/action` | exact-window focus/minimize/maximize/restore/close | `desktop_window_action` |

The successor does **not** expose `uia.visual/propose`, `uia.coordinates/propose`, or the legacy `uia.input/execute` click-lease chain as MCP tools. It also adds no MCP grant/renew/revoke operation. Full User is granted only through the local lifecycle CLI after STRONG platform presence, and every control call is denied for remote dispatch.

All actuation is additionally gated by an exact FullControlLease, per-action SOFT approval, typed process/window/element generation checks, protected-surface rules, policy revision, runtime-session binding, authority epoch, and immediate pre-dispatch revalidation. Raw input binds the current human-input tick; drift cancels before dispatch. A mutating native worker is attempted once only, so a crash or timeout after possible dispatch reports `outcome_unknown` and is never automatically retried.

The Windows Computer Host remains a private stdin/stdout child with no independent MCP/HTTP/LAN listener and no grant material. Whole-desktop or monitor capture, arbitrary capture regions, recording/ffmpeg, shell/process or filesystem widening, browser/network widening, elevation broker execution, Persistent Admin, Remote Full Control, UAC/Windows Hello/secure-desktop bypass, SYSTEM/TrustedInstaller authority, and release/installer/tag mutation remain outside SG-000095.
