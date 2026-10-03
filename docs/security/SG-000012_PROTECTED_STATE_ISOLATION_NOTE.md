# SG-000012 — Restricted-child protected-state isolation qualification

SG-000012 qualifies a negative-access boundary for the existing contained Windows process path before Qdral widens executable or PowerShell authority.

## What is qualified

The native Windows integration fixture creates a Qdral-owned sentinel beneath the parent user's LocalAppData `Qdral` state boundary. The parent verifies that the sentinel exists, builds the execution plan through the production sanitizer, and launches the integration-test child through the existing zero-capability AppContainer + kill-on-close Job executor.

The contained child succeeds only when all of the following are true:

- `QDRAL_TUNNEL_KEY_FILE` is absent;
- `OPENAI_API_KEY` is absent;
- `QDRAL_DAEMON` is absent;
- stdin is NUL/EOF rather than a readable inherited authority channel;
- reading the Qdral-owned protected-state sentinel fails.

The parent then requires:

- child exit code `0`;
- AppContainer token verification;
- Job assignment before resume;
- verified Job quiescence.

The sentinel bytes are never emitted to stdout, stderr, logs, or model-visible evidence.

## Handle and authority-channel boundary

SG-000012 does not add a new launch implementation. It deliberately traverses the already-qualified contained executor. That executor constructs a dedicated NUL stdin handle plus dedicated stdout/stderr pipe writers and passes only those explicit standard handles through `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`.

The current request-scoped `qdral-mcp -> qdrald` authority path uses the daemon's own stdio. It is not one of the child handles. The native fixture additionally proves the contained child's stdin is EOF/NUL, so the daemon request channel was not inherited as readable child stdin.

This grain does not introduce service-mode named-pipe IPC and does not claim qualification of a future IPC architecture.

## Filesystem boundary

The qualification target is Qdral-owned state under LocalAppData, not a trusted workspace and not a user-selected file. SG-000012 does not change workspace ACLs or arbitrary user-file permissions.

If native Windows evidence shows that the default Qdral-owned state ACL permits the AppContainer read, the grain must fail closed. Any repair must be limited to Qdral-owned protected state and must be requalified natively before the grain can close.

## Authority boundary

No public authority is added by SG-000012.

Windows public `process.spawn` remains restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` executable. The following remain absent or denied:

- generic executable-registry widening;
- additional public process targets;
- raw shell / `cmd.exe` authority;
- `powershell.run` or direct PowerShell executable authority;
- caller environment overrides;
- stdin payload injection;
- process network authority;
- detached/background execution;
- public process kill;
- Git mutation;
- browser/UI automation;
- elevation;
- approval bypass or persistent approval reuse.

Exact-head Windows/Ubuntu CI, genuine TypeSafe Jev, Alibaba Open Code Review, security review, and post-merge verification remain required before SG-000012 can become canonical.
