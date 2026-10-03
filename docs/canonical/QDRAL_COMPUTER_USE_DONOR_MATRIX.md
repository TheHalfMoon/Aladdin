# Qdral UI-TARS Donor Matrix

Status: PLANNED PINNED DONOR INPUT
Pinned donor: `bytedance/UI-TARS-desktop@2ff41a9e515828c5bd5b276e493d73aa0bdf4a3a`
Date: 2026-10-03

This matrix is planning input for SG-000066 donor audit and QDRAL-P18. Exact reused file paths and dependency/license obligations must be expanded during SG-000066/SG-000073 before code import.

| Upstream area | Initial classification | Intended Qdral use | Authority rule |
| --- | --- | --- | --- |
| `packages/agent-infra/mcp-servers/browser` | ADAPT / PORT | live browser/Puppeteer engineering | never run as an independent MCP authority edge |
| browser accessibility/structured data logic | ADAPT / PORT | live DOM/AX provider implementation | Qdral issues page/node identities and bounds results |
| browser navigation/click/fill implementations | PORT SELECTIVELY | provider backend mechanics | existing Qdral destination/node/approval contracts remain authoritative |
| browser screenshot logic | REFERENCE / PORT SELECTIVELY | browser-page visual support if separately authorized | no generic desktop capture authority |
| browser `browser_evaluate` | REJECT | none | arbitrary caller JavaScript remains denied |
| raw CDP/debug features | REJECT AS TOOL | internal implementation details only if narrowly required | never exposed as caller authority |
| `packages/ui-tars/action-parser` | ADAPT | UI-TARS syntax -> ComputerActionProposal | proposal only; cannot mint target/approval/lease/execution authority |
| `packages/ui-tars/operators/nut-js` | REFERENCE_ONLY | DPI/coordinate/input engineering study | direct model-to-NutJS execution not imported |
| NutJS screenshot/DPI normalization | PORT_SELECTED_LOGIC | inform exact-window coordinate normalization | Qdral native exact-window capture/protected rules control authority |
| NutJS clipboard typing | REJECT | none | Qdral bounded text input must not modify clipboard as typing transport |
| RemoteComputerOperator | REFERENCE_ONLY | lifecycle/RPC design study | Qdral remote principal/device/session/lease model stays authoritative |
| Tarko/Agent event stream | ADAPT CONCEPT | UI/telemetry event presentation | canonical Qdral audit/evidence remains source of truth |
| Electron IPC/UI patterns | REFERENCE_ONLY | narrow UI implementation ideas | renderer remains sandboxed/low authority; no donor permission boundary |
| donor agent loop / model runtime | REJECT_FOR_CORE | none in Qdral authority layer | provider-neutral Qdral kernel remains in control |
| donor filesystem tools | REJECT_FOR_IMPORT | none needed | existing Qdral filesystem contracts remain authoritative |
| donor `run_command` / `run_script` | REJECT | none | bounded Qdral executable registry only |
| personal browser profile / caller user-data-dir | REJECT | none | dedicated Qdral automation profile only |
| donor remote/browser HTTP/SSE listener patterns | REFERENCE_ONLY | protocol lessons only | browser host has no independent public control listener |

## License notes to verify at exact reused paths

The upstream repository root is Apache-2.0. Individual packages may declare different licenses; for example the browser MCP package declares MIT while the UI-TARS action parser and NutJS operator declare Apache-2.0. The exact reused file/package/dependency boundary must therefore be recorded before import rather than assuming one repository-wide rule for every copied component.

## Required source-audit output before implementation

For every selected file/subsystem, record: donor repository and exact commit, source path, file/package license and notices, transitive dependency obligations, Qdral destination, reuse classification, whether modified, authority delta, tests required, and the SpecGrain that authorizes import/execution. If any of these fields is unknown, the code is not implementation-ready for import.
