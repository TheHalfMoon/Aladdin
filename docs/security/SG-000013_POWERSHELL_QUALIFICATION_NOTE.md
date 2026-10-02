# SG-000013 — Private bounded PowerShell containment qualification

Status: IMPLEMENTATION CANDIDATE
Program: QUNTAL-P05

SG-000013 qualifies Windows PowerShell inside Quntal's already-proven contained process engine without exposing a public PowerShell capability.

## Qualification boundary

The native Windows integration qualification invokes the exact inbox Windows PowerShell executable under `System32\WindowsPowerShell\v1.0\powershell.exe` directly through `quntal-provider-process`.

Every fixture uses a fixed test-owned script and the exact prefix:

- `-NoLogo`
- `-NoProfile`
- `-NonInteractive`
- `-Command`

The qualification path uses a trusted-workspace cwd, the existing sanitized environment, NUL stdin, bounded timeout/stdout/stderr, a zero-capability AppContainer, suspended launch with token verification, Job assignment before resume, and verified Job quiescence.

## Native cases

The Windows suite proves:

1. deterministic bounded success with expected stdout and EOF on NUL stdin;
2. typed `ProcessTimeout` only after verified Job termination/quiescence;
3. stream-typed stdout `OutputLimit` only after verified termination;
4. stream-typed stderr `OutputLimit` only after verified termination;
5. Quntal/tunnel/API secret-like source environment values are removed from the child plan;
6. existing SG-000011 descendant-aware lifecycle and SG-000012 protected-state/authority-channel tests remain part of the full Windows regression suite.

## Authority boundary

This grain changes tests and security documentation only.

It does not register `powershell.run` in MCP, does not authorize PowerShell in `quntald` policy, and does not add PowerShell to the public `process.spawn` executable policy.

Windows public `process.spawn` therefore remains restricted to the SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target.

Still absent or denied:

- caller-provided PowerShell script text;
- caller-selected PowerShell executable authority;
- PowerShell through public `process.spawn`;
- raw `cmd.exe` / shell authority;
- profile loading;
- remoting or process network capability;
- caller environment overrides;
- stdin payload injection;
- detached/background execution;
- public kill authority;
- Git mutation;
- browser/UI automation;
- elevation;
- approval bypass or persistent approval reuse.

Exact-head CI, genuine TypeSafe Jev, Alibaba Open Code Review, security review, merge, and post-merge evidence must be recorded through the governed closeout before SG-000013 may be called `CLOSED`.
