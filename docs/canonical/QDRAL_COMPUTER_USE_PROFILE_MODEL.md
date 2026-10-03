# Qdral Computer-Use Profile Model

Status: PLANNED PROFILE CONTRACT FOR QDRAL-P18
Date: 2026-10-03

Profiles are explicit allowlists derived from the authoritative Qdral tool contract. Provider support or implementation presence never implies profile membership.

## Target local profiles

- `core`: existing bounded cross-provider core.
- `desktop_observe`: exact structured desktop observation plus exact-window capture only after capture qualification.
- `browser_structured`: only live-qualified governed browser shapes.
- `desktop_control`: live-qualified structured UIA actuation only.
- `coordinate_fallback`: separately enabled lower-ceiling coordinate actions only.
- `computer_use`: explicit local union selected by local policy; not a remote default.

## Rules

1. Unknown profile names fail closed.
2. Adding a provider implementation cannot add a tool to any profile automatically.
3. A tool not present in the authoritative tool contract cannot be registered.
4. A live provider shape may remain hidden/local-only even after qualification.
5. Remote provider OAuth/scope authorization is only an outer ceiling and cannot enable a local profile.
6. Remote computer-use requires SG-000084 local lease authority in addition to any transport scope/profile.
7. `coordinate_fallback` is never implicitly included because `desktop_control` or `computer_use` is enabled; any union must list it explicitly in the authoritative contract and local policy.
8. `browser_evaluate`, raw CDP/DevTools, unrestricted shell/script, personal-profile operations, raw model input, security-dialog automation, remote approval, and model self-approval belong to no profile.
9. Discovery, invocation, parity inventory, generated docs, OAuth/remote tables, and provider ceilings are regression-tested for exact agreement.
