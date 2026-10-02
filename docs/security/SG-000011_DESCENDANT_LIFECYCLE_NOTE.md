# SG-000011 — Descendant-aware post-exit lifecycle hardening

SG-000011 closes the post-primary-exit pipe-drain gap identified during SG-000010 review without widening public process authority.

## Qualified behavior

After the primary contained process exits, stdout and stderr are drained together under an explicit bounded deadline. If both pipes reach EOF, normal Job quiescence verification continues. If the drain deadline expires while Job members remain active, Quntal terminates the Job, requires verified zero active Job processes, and then performs one final bounded drain so inherited pipe handles must reach a terminal state before success.

If Job termination, zero-active-process verification, or final pipe closure cannot be proven, the executor returns `TerminationUnverified`. Output-limit events retain stream-typed `OutputLimit` semantics and are never promoted without verified termination.

## Native qualification fixture

The SG-000011 fixture is provider-private. The executor creates a primary test process and a second AppContainer-contained Job member that shares the same inherited stdout/stderr handles. The primary exits immediately while the second Job member remains alive, reproducing the retained-handle condition deterministically. The fixture does **not** claim that the second process was spawned by the primary process; it qualifies the Job/pipe lifecycle condition that caused the unbounded drain.

Both processes are created suspended, token-verified as AppContainer processes, assigned to the same kill-on-close Job before resume, use the existing sanitized environment and three inherited standard handles, and receive no network capability.

Exact-head CI, Jev, OCR, and post-merge evidence are recorded in the governed PR/SpecGrain closeout rather than self-referenced in this security note.

## Authority boundary

No public authority is added. Windows public `process.spawn` remains restricted to the exact SG-000010-qualified `%SystemRoot%\System32\whoami.exe` target. Generic executable-registry widening, shell/cmd, PowerShell, caller environment overrides, stdin payloads, network, detached/background execution, public kill, Git mutation, browser/UI automation, elevation, and approval bypass/reuse remain absent.
