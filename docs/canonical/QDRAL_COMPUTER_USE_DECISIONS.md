# Qdral Computer-Use Non-Negotiable Decisions

Status: PLANNED GOVERNANCE INPUT FOR QDRAL-P18
Date: 2026-10-03

This record exists so later implementation agents cannot infer authority from UI-TARS or any other donor simply because the donor supports a capability.

## Required decisions

1. Qdral remains the sole authority for policy, trust, approvals, target identity, leases, destination policy, audit, revocation, and tool exposure.
2. Donor code runs only behind Qdral-controlled low-authority provider/host boundaries.
3. Browser control uses a dedicated Qdral automation profile; personal browser profiles are not supported by P18.
4. Caller-facing arbitrary JavaScript, generic CDP/DevTools, unrestricted shell/script, generic socket/proxy, remote approval, model self-approval, silent elevation, credential/security-dialog automation, and raw model-to-input execution are intentionally denied.
5. Structured browser and structured UIA actuation are preferred. Coordinate input is a separately enabled lower-ceiling fallback and never a silent fallback.
6. Desktop capture is exact-window scoped. Whole-screen/background streaming is not part of P18.
7. CaptureLease, InputLease, and RemoteComputerUseLease are server issued, bounded, revocable, identity/policy bound, and cannot be minted or widened by the model/provider/remote caller.
8. Human physical input preempts synthetic coordinate authority.
9. Mutating operations are not automatically retried after dispatch.
10. Browser/network, DOM/node, process/window/element, capture, lease, session/route, policy/trust, lock/logoff, and restart drift invalidate affected authority deterministically.
11. Browser downloads stage under Qdral before bounded verified placement. Upload sources remain explicitly confined; donor browser filesystem flexibility does not widen Qdral filesystem authority.
12. UI-TARS action parsing may normalize syntax only. A parsed action is a proposal, not executable authority.
13. Donor event streams may inform UI/telemetry design but canonical security evidence remains Qdral audit/evidence.
14. Production packaging uses pinned reviewed dependencies; no `npx ...@latest`, runtime donor-code fetch, or automatic personal-browser fallback.
15. No P18 capability is exposed until a live provider implementation passes its own SpecGrain and tool/profile exposure grain.

Any change to these decisions requires an explicit governed authority change, not an implementation shortcut.
