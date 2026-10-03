# Quntal Computer-Use Failure Semantics

Status: PLANNED NORMATIVE FAILURE CONTRACT FOR QUNTAL-P18
Date: 2026-10-03

## Core rule

Failure never widens authority. A failure in a structured path does not authorize coordinate fallback, a browser failure does not authorize personal-profile fallback, and a transport failure does not authorize retrying a mutation after dispatch.

## Required invalidation behavior

| Event | Required result |
| --- | --- |
| browser-host crash | invalidate all browser engine/page/document/node volatile identities; no mutation auto-retry |
| browser process crash/restart | invalidate engine/page/document/node identities and pending transfer sources |
| navigation/document replacement | prior document/node identities become stale |
| page origin/generation drift | affected browser request fails closed as stale |
| process restart/PID reuse | prior process/window/element/capture/input identities become stale |
| HWND destruction/reuse/class drift | prior window/element/capture/input identities become stale |
| window resize/move/DPI drift | capture/coordinate proposal becomes stale according to the bound geometry contract |
| UIA element replacement/state/pattern drift | structured desktop action fails closed and must be reproposed from a fresh observation |
| human physical input | active coordinate execution authority is cancelled/revoked; no automatic resume |
| CaptureLease expiry/revoke | further frames denied; no silent renewal |
| InputLease expiry/revoke/dispatch | further use denied; dispatched leases are consumed even if postcondition fails |
| RemoteComputerUseLease expiry/revoke | all further remote computer-use dispatch denied; no silent refresh |
| workspace trust/policy revision change | affected pending identities/approvals/leases fail closed |
| emergency revoke | affected local/remote computer-use authority is invalidated immediately |
| lock/logoff/RDP/session transition | affected desktop/capture/input/remote authority invalidated; protected surfaces remain inaccessible |
| Quntal restart | volatile browser/capture/input/remote leases and volatile target identities invalidated unless a future grain explicitly proves safe persistence |
| remote disconnect | no surprise offline mutation queue; dispatched mutation is not auto-retried |
| timeout/cancellation | outcome is typed unknown/cancelled unless a verified provider postcondition proves a result; no fabricated success |

## Retry rules

- Mutating browser, transfer, structured desktop, and coordinate actions are never automatically retried after dispatch.
- A caller may submit a new operation only after obtaining fresh current-state evidence and any required fresh approval/lease.
- Idempotent reads may be retried only where the governing transport/provider contract explicitly allows it and all original identity/lease/expiry conditions still hold.
- A reconnect never resurrects expired or revoked authority.

## Evidence rule

Quntal distinguishes `not_started`, `dispatched`, `completed`, `cancelled`, and `outcome_unknown` where the provider can support that distinction. It never claims a mutation was reverted, terminated, or completed without provider evidence.
