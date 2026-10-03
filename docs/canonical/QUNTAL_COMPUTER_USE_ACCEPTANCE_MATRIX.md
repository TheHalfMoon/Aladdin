# Quntal Governed Computer-Use Acceptance Matrix

Status: PLANNED QUALIFICATION MATRIX FOR QUNTAL-P18
Date: 2026-10-03

This matrix is normative planning input. An implementation grain may narrow a capability but must not silently broaden any row.

| Domain | Required proof before exposure |
| --- | --- |
| Browser host | isolated Quntal profile; supported executable identity; sandbox enabled; no public control listener; deterministic supervision/cleanup |
| Browser network | public-destination policy; DNS re-resolution; redirect checks; subresource mediation; no private/metadata/bypass route |
| Browser identity | exact profile/page/origin/page generation/document generation/node identity; stale/forged/cross-scope failure |
| Browser observation | bounded DOM/AX; password/secret redaction; no cookie/storage/credential/raw-JS/CDP exposure |
| Browser actuation | exact current node/role/state; fresh one-shot approval; race-safe precheck; bounded postcondition; no coordinate fallback |
| Downloads | protected staging; origin/redirect/type/size/digest verification; create-only approved destination; no open/execute/extract |
| Uploads | admissible server-recorded source only; exact file-input node; digest/size/media/trust revalidation; one-shot approval |
| Desktop observation | existing exact process/window/UIA identity and protected-surface exclusions remain intact |
| Window capture | exact window only; capture generation; DPI/geometry/session binding; bounded CaptureLease; no whole-screen/background stream |
| Structured desktop actuation | exact UIA element and supported pattern; current state; password/protected denial; fresh approval; postcondition |
| Coordinate fallback | exact capture/window/action; one-shot InputLease; human interruption; no clipboard typing/free-form stream/silent fallback |
| Model adapters | parser only; bounded canonical proposal IR; no authority fields; no direct backend calls; malformed/unknown fail closed |
| Remote reach | locally created exact RemoteComputerUseLease; provider/principal/device/session/route isolation; local approvals remain local |
| Failure semantics | crashes/restarts/drift invalidate volatile identities/leases; no mutation auto-retry or surprise offline queue |
| Privacy | screenshot/browser/secret bytes excluded from logs/metrics/crash reports; evidence bounded and intentionally designed |
| Resource safety | hard ceilings for processes/tabs/requests/transfers/DOM/images/results/events/leases/timeouts and deterministic cleanup |
| Denied authority | arbitrary JS/CDP, unrestricted shell/script, generic socket/proxy, personal profile, raw model input, remote approval/self-grant, elevation/security dialogs remain unreachable |
| Packaging | pinned dependencies; licenses/notices; SBOM/provenance; reproducibility; no runtime latest/dynamic code fetch; doctor reports supported engine state |
| Qualification | exact-head CI, real-Windows tests where applicable, TypeSafe Jev, Alibaba Open Code Review, manual excluded-file review, security/privacy review, zero unresolved blockers, post-merge verification |

P18 cannot exit with a row marked unproven for an exposed capability.
