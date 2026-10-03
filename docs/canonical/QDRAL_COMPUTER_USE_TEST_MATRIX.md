# Qdral Computer-Use Test Matrix

Status: PLANNED NORMATIVE TEST CATALOG FOR QDRAL-P18
Date: 2026-10-03

The implementation grains may split or extend these tests, but an exposed capability cannot omit its applicable category.

## Browser/network

- public HTTPS happy path;
- DNS rebinding and re-resolution drift;
- redirect widening, downgrade, loops, excessive hops;
- private, loopback, link-local, multicast/metadata and userinfo denial;
- popup/new-window and iframe navigation;
- worker/service-worker and fetch/XHR;
- WebSocket and direct-network bypass attempts;
- QUIC/DoH/WebRTC policy;
- unsafe/browser-internal/file/javascript/custom schemes;
- browser permission prompts and external protocol handlers;
- automatic download trigger.

## Browser identity and actuation

- page/document/node generation drift;
- forged/cross-page/cross-origin/cross-workspace node IDs;
- dynamic DOM replacement;
- disabled/wrong-role/wrong-pattern targets;
- approval replay/expiry/digest mismatch;
- cancellation/timeout/host crash before and after dispatch;
- postcondition match/mismatch;
- password/secret redaction;
- arbitrary JS/CDP denied.

## Transfers

- safe download and upload happy paths;
- path traversal, UNC/device/NT/ADS/reserved names;
- symlink/junction/reparse escape;
- dangerous/unknown/mismatched type;
- oversize/truncated/size mismatch;
- changed upload source;
- arbitrary caller path denied;
- duplicate write/submission and replay;
- crash/cancel cleanup;
- no auto-open/execute/extract.

## Desktop/capture

- same-session live window and exact-window capture;
- Qdral/protected/security window exclusion;
- CredentialUIBroker/UAC/consent/LogonUI/secure desktop;
- PID/HWND reuse and class drift;
- DPI scaling, resize/move, multi-monitor;
- minimized/occluded behavior;
- RDP/session transition and lock/logoff;
- capture lease issue/use/revoke/expiry;
- screenshot size/rate/byte ceilings;
- screenshot bytes absent from logs/metrics.

## Structured UIA

- Invoke/Value/SelectionItem/Toggle/Scroll happy paths;
- wrong role/state/pattern/read-only/disabled;
- element replacement and stale observation;
- password/credential target denial;
- focus/state race and postcondition mismatch;
- arbitrary UIA pattern denied.

## Coordinate input

- click/double/right/scroll/text happy paths;
- stale capture/window/process/geometry/DPI;
- out-of-bounds/overflow/invalid coordinates;
- human physical input interruption;
- input lease issue/use/replay/expiry/revoke;
- lock/logoff/RDP transition;
- no clipboard typing;
- unrestricted hotkey/drag/free-form stream denied;
- no silent structured-to-coordinate fallback.

## Model adapters

- valid canonical normalization;
- malformed/truncated/oversized input;
- ambiguous action;
- unknown action/fields;
- NaN/infinite/overflow coordinates;
- authority-smuggling fields such as approval/trust/lease/target IDs;
- direct-backend invocation impossible;
- fuzz/property/differential corpus.

## Remote isolation

- correct provider/principal/device/session/route lease;
- cross-provider/principal/device/session/route replay;
- expired/revoked/widened lease;
- local approval requirement retained;
- remote self-grant/self-approval denied;
- disconnect/reconnect/token refresh/route epoch drift;
- emergency revoke and device revoke;
- no background stream or offline mutation queue;
- compromised-relay route confusion fails locally.

## Release/static denial

- no caller-facing arbitrary JS/CDP;
- no unrestricted shell/PowerShell/cmd/script path;
- no generic socket/proxy/tunnel;
- no personal-profile fallback;
- no raw model-to-input path;
- no remote approval/model self-approval;
- no silent elevation/security-dialog automation;
- no runtime `latest`/dynamic donor-code fetch;
- tool/profile/parity/OAuth/remote tables equal shipped discovery.
