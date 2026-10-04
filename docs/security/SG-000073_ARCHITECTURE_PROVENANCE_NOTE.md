# SG-000073 Computer-Use Architecture and Provenance Freeze

Status: SG-000073 NORMATIVE FREEZE (QDRAL-P18)
Date: 2026-10-04
Authority delta: none. This record freezes vocabulary, boundaries, and
provenance. It authorizes no browser launch, donor dependency, tool,
profile, lease, or execution.

## 1. Pinned donor input

The only P18 donor input is the pinned UI-TARS revision below, studied
read-only. No donor file is imported, vendored, linked, or fetched at
runtime by SG-000073. Desktop Commander and Kernux remain reference-only
pins under SG-000066 exit evidence.

| Item | Frozen value |
| --- | --- |
| Donor repository | `bytedance/UI-TARS-desktop` |
| Pinned revision | `2ff41a9e515828c5bd5b276e493d73aa0bdf4a3a` |
| Root license at pin | Apache-2.0 (`LICENSE` verified present at the pinned revision) |
| Action parser package | `packages/ui-tars/action-parser` version `1.2.3`, license `Apache-2.0` (declared in its `package.json` at the pin) |
| Parser entry examined | `packages/ui-tars/action-parser/src/actionParser.ts` (blob `6f411c3a0f846c3e21f5a576a4b93315ae7517b8`), SPDX `Apache-2.0` header, exports `actionParser` and `parseActionVlm` |
| Shared types examined | `packages/ui-tars/shared/src/types/agent.ts` (blob `7b9864b8aef6c78149eab014412896e9b0439c7d`) and `types/data.ts` (blob `22dad6703f8cf17f7457039b285b110607619db3`), both SPDX `Apache-2.0` headers |
| Parser output shape | `{ reflection, thought, action_type, action_inputs }` with `action_inputs` drawn from `content`, `start_box`, `end_box`, `key`, `hotkey`, `direction`, `start_coords`, `end_coords` |
| Parser transitive needs | `@ui-tars/shared` (workspace, same repository) and `lodash.isnumber@3.0.3`; both licenses must be re-verified against the exact reused files before any import grain |

Exact reused file paths are recorded per import grain, not here: SG-000073
imports zero donor files, so the reused set is empty and the
`imports-no-runtime` regression proves it stays empty. Every future import
grain records donor repository and exact commit, source path, file and
package license with notices, transitive dependency obligations,
modification notices where Apache-2.0 section 4(b) requires them, reuse
classification, authority delta, and tests, or the code is not
implementation-ready.

## 2. Reuse classifications (frozen)

Browser MCP/Puppeteer implementation: adapt or port the engine mechanics
only, never as an independent MCP authority edge. Action parser:
syntax-to-proposal only; it cannot authorize execution. Operator
abstraction: port the concept by splitting
observation, proposal, validation, and execution. NutJS desktop operator:
reference only; the raw model-to-input path is never imported.
Screenshot and DPI handling: port selected exact-window normalization logic
only. NutJS clipboard typing: reject. RemoteComputerOperator: reference only
for lifecycle and RPC design. Event stream: adapt the presentation concept;
canonical evidence stays Deskal audit. Electron IPC and UI patterns:
reference only; the renderer stays sandboxed and low authority. Donor agent
loop and model runtime: reject for core authority. Donor filesystem tools,
`run_command`, and `run_script`: reject. Arbitrary browser JS and raw CDP:
reject. Personal browser profiles: reject. Raw model-to-input execution:
reject.

## 3. Canonical proposal vocabulary (frozen)

Recorded normatively in `apps/qdral-mcp/src/computer_action_proposal.ts`
and pinned by `computer-action-proposal.test.ts`. Proposals carry untrusted
data only.

Proposal verbs (sixteen, proposal-only): `browser_navigate`,
`browser_click`, `browser_fill`, `browser_select`, `browser_snapshot`,
`uia_invoke`, `uia_set_value`, `uia_select`, `uia_toggle`, `uia_scroll`,
`uia_screenshot`, `coordinate_click`, `coordinate_double_click`,
`coordinate_right_click`, `coordinate_scroll`, `coordinate_type`.

Target reference fields (twelve, server-issued values only):
`target_kind`, `workspace_id`, `browser_profile_id`, `page_id`,
`window_id`, `node_id`, `element_id`, `page_generation`,
`document_generation`, `window_generation`, `capture_generation`, `origin`.

Envelope fields (four): `action`, `target`, `parameters`, `provider_hint`.

Forbidden authority-bearing fields (ten, never in proposal data):
`approval`, `approval_class`, `trust`, `workspace_authority`,
`policy_revision`, `capture_lease`, `input_lease`, `remote_lease`,
`execution_result`, `postcondition_result`. Adapters may normalize syntax;
they cannot create or override Deskal target IDs, approvals, trust,
workspace authority, policy revision, leases, execution results, or
postcondition results.

## 4. Boundaries (frozen)

Process boundary: the future browser host runs as a Deskal-launched
low-authority child process (`qdral-browser-host`), never as an independent
principal. IPC boundary: a private local framed channel only; no public
MCP endpoint, no HTTP or LAN control surface, no remote-debugging TCP
listener. Trust boundary: workspace trust mutates only through Deskal
STRONG-gated APIs. Approval boundary: approvals are minted only by the
local broker with one-shot digest binding; the model, provider, adapter,
browser host, and remote caller mint none. Lease boundary: capture, input,
and remote leases are issued and consumed only by Deskal policy; expiry and
revocation stay dead with no silent renewal or resurrection. Shift boundary:
no stage (donor, parser, host, adapter, transport) collapses propose,
validate, authorize, and execute.

## 5. Denied authority (frozen)

Arbitrary JavaScript, generic CDP and DevTools, unrestricted shell and
script execution, generic sockets, proxies, and tunnels, personal browser
profiles, raw model-to-input execution, whole-screen and background capture
streams, credential and security-dialog automation, remote approval, model
self-approval, silent elevation, silent coordinate fallback, arbitrary
file-transfer paths, mutation auto-retry after dispatch, and runtime
dynamic donor-code fetch remain unreachable unless a separately governed
program changes them.

## 6. Browser-host packaging strategy (frozen)

Pinned reviewed browser-host code and dependencies ship inside the Deskal
release and SBOM. No runtime `@latest` fetch and no dynamic donor-code
fetch. The host prefers a supported locally installed Chromium-family
executable on Windows with executable, path, and publisher verification; a
Deskal-owned automation profile only; ephemeral by default; sandbox
enabled; unsupported or missing engines return typed unavailable and never
fall back to a personal browser. `qdral doctor` reports the installed
browser engine identity and version, automation profile class and location,
browser-host version and protocol compatibility, and whether live browser
capability is qualified and enabled.

## 7. Windows provider boundaries (frozen)

Desktop observation and actuation stay inside the Win32 and UI Automation
provider boundary with process-lifetime typed identities. Exact-window
capture stays inside the native per-window capture boundary with
server-issued capture generations. Coordinate fallback stays inside the
bounded native-input boundary with one-shot input leases, fresh approval,
and immediate human-input preemption. Deskal protected surfaces (approval,
trust, and revoke UI, CredentialUIBroker, consent, UAC, LogonUI, and the
secure desktop) stay denied in every provider.

## 8. Unreachability proof

Donor code is not independently reachable from MCP or remote interfaces:

- `SG-000066 imports no Desktop Commander, Kernux, or UI-TARS runtime`
  (marker scan over `apps/qdral-mcp/src`, `Cargo.lock`, manifests, and
  `crates`, failing on any donor runtime marker);
- `no MCP tool claims browser capability`, `no browser tool is registered
  on the MCP server surface`, and `every structured browser shape is
  recorded as not exposed and denied in the inventory`;
- `the structured browser layer launches, attaches, and connects to
  nothing outside tests`;
- the SG-000073 freeze tests pinning the sixteen verbs, twelve target
  fields, four envelope fields, and ten forbidden fields with unknown
  verbs and smuggled authority failing closed.

No donor parser or backend receives authority-bearing fields: there is no
parser or backend in the tree, and the frozen vocabulary gives adapters no
field in which to carry authority.
