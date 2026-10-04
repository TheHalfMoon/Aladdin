# Qdral Computer-Use Resource Bounds

Status: PLANNED NORMATIVE BOUNDS FOR QDRAL-P18
Date: 2026-10-03

Exact numeric defaults are set and tested in the implementation grain that owns each resource. This document fixes the requirement that every resource has an explicit hard ceiling before exposure.

## Mandatory bounded resources

- browser-host processes and browser child processes;
- browser tabs/pages per workspace/session;
- navigation redirects and total navigation lifetime;
- in-flight browser requests and WebSockets;
- response/header metadata retained for policy;
- DOM/accessibility max nodes, max depth, max bytes, and observation time;
- pending browser action proposals and approvals;
- staged download count, per-download bytes, total staged bytes, and staging lifetime;
- pending upload sources, per-upload bytes, and source lifetime;
- screenshot/capture width, height, pixel count, encoded bytes, frame rate, and frames per lease;
- capture leases per process/window/session and maximum lease lifetime;
- UIA tree nodes/depth/bytes/time and concurrent element actions;
- input leases per session, action payload size, text length, scroll magnitude, and lease lifetime;
- model-adapter input bytes, action count, string lengths, coordinate/box dimensions, and parse time;
- remote computer-use leases per principal/device/workspace, concurrent requests, result bytes, and hard lease duration;
- event-stream/audit presentation payload size and queue depth;
- host/provider stdout/stderr/log size and retention;
- cancellation/cleanup deadlines and orphan-state retention.

## Fail-closed rule

Unknown, missing, non-finite, negative, overflowed, or above-ceiling resource requests fail before authority is exercised. Limits are server-owned and cannot be raised by model fields or remote request metadata.

## Backpressure rule

On resource exhaustion Deskal returns a typed bounded-resource failure. It does not spawn an unbounded helper, silently increase a ceiling, switch to a less-governed backend, or queue a mutation for surprise later execution.
