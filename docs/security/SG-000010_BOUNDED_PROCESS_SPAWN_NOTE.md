# SG-000010 — Approved bounded argv `process.spawn`

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P05

## Authority delta

SG-000010 adds exactly one public EXECUTE capability: `process.spawn` / `spawn`.

The interface is argv-first. It accepts an absolute executable path and a separate argv array, but this first public grain deliberately authorizes only the native Windows `whoami.exe` executable that is used by the qualification fixture. It does not accept a raw command string, shell text, PowerShell text, caller environment overrides, stdin payloads, background execution, process network capability, public kill authority, elevation, browser/UI automation, or approval reuse.

This narrow positive executable policy is intentional. A generic absolute-executable policy would allow `cmd.exe`, PowerShell, another interpreter, or an equivalent renamed interpreter to smuggle shell/script authority through argv and would contradict the SG-000010 authority boundary. Executable-registry widening is therefore a successor authority grain with its own policy and qualification.

## Request boundary

The MCP edge and policy kernel independently constrain:
- executable: non-empty absolute local path, and on Windows it must resolve exactly to the qualified `%SystemRoot%\\System32\\whoami.exe` executable for SG-000010;
- argv: bounded string array with no NUL bytes;
- cwd: relative path resolving inside the selected trusted workspace;
- timeout: 1 second through 30 minutes;
- stdout: 1 byte through 16 MiB;
- stderr: 1 byte through 4 MiB;
- stdin policy: `null` only;
- network class: `NONE` only.

Unknown process argument fields fail closed. In particular, `command`, `env`, stdin payloads, and detached/background controls are not part of the contract.

## Normalization and approval

`quntald` builds an `ExecutionPlan` before approval. The plan canonicalizes the executable and cwd, preserves argv order, applies the fixed inherited-environment allowlist, removes Quntal/tunnel and secret-like variables, and validates execution limits.

Every execution requires a fresh independent local approval. The approval digest is SHA-256 over length-prefixed normalized fields with domain separation. It binds:
- workspace identity;
- policy revision;
- canonical executable;
- argv in order;
- canonical cwd;
- timeout/stdout/stderr bounds;
- `stdin=null`;
- `network=NONE`;
- the exact sanitized inherited environment.

Material plan drift therefore changes the approval digest. Denied or unavailable approval returns before the contained executor is called.

## MCP request lifecycle

`process_spawn` does not share the long-lived read/write/Git daemon. Each EXECUTE request creates a request-scoped `KernelClient` / `quntald` process, performs the approval and contained execution through that daemon, and closes it in a `finally` block.

The MCP-side deadline is the requested provider runtime bound plus a five-minute local-approval window. If that deadline expires, the promise rejects and the `finally` block closes the request-scoped daemon. Closing `quntald` drops its contained Job handle, so an execution cannot later start or continue silently after the MCP caller has already observed timeout. Other Quntal tools keep their independent long-lived daemon and are not interrupted by this fail-closed EXECUTE lifecycle.

This is the SG-000010 cancellation boundary. A future durable-operation protocol may provide richer cancellation/reconnect semantics, but this grain must not leave an approval or contained process running after its request-scoped client has ended.

## Windows containment

The public path reuses the already-qualified SG-000009/SG-000009A contained executor rather than introducing `std::process::Command` or a shell bypass.

The child is:
1. created suspended in a zero-capability AppContainer;
2. given only NUL stdin plus explicit stdout/stderr pipe handles;
3. assigned to a kill-on-close Job Object;
4. verified as a Job member before resume;
5. verified to have an AppContainer token before resume;
6. resumed only after those checks;
7. monitored with independent timeout/stdout/stderr bounds.

Timeout or output overflow terminates the Job and is promoted to its typed failure only after zero active Job processes are verified. Failure to prove quiescence returns `PROCESS_TERMINATION_UNVERIFIED`.

The existing generic executor contains a post-primary-exit pipe-drain path that is not yet qualified for arbitrary executables that create descendants retaining stdio handles. SG-000010 avoids claiming that broader authority by limiting the public executable policy to the fixed `whoami.exe` qualification class. Descendant-aware post-exit drain/termination hardening is a prerequisite for any successor executable-registry widening.

## Output and evidence

Successful execution returns bounded stdout/stderr, exit code, and containment evidence:
- AppContainer token verified;
- assigned to Job before resume;
- Job quiescent at completion;
- `network_class=NONE`;
- `stdin_policy=null`.

No background process handle or public process identifier is returned.

## Native qualification

The required Windows end-to-end test traverses policy -> approval -> daemon -> process provider using the fixed system inbox `whoami.exe` fixture. It must prove exit/output evidence, AppContainer identity, pre-resume Job membership, and final Job quiescence.

The Windows policy suite must also prove that direct `cmd.exe` and PowerShell executable requests are rejected as `CAPABILITY_DENIED` in SG-000010.

SG-000009 and SG-000009A timeout/output-limit/termination regressions remain mandatory. Exact-head Windows/Ubuntu Rust, Node, Governance, genuine Jev, and Alibaba Open Code Review gates are required before merge.
