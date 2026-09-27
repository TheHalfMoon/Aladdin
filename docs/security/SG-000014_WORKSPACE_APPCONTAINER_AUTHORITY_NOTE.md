# SG-000014 — Workspace-scoped AppContainer filesystem authority qualification

## Scope

This grain qualifies only a provider-private, test-owned Windows workspace ACL boundary for a zero-capability AppContainer identity. It does not add public filesystem, process, PowerShell, network, Git, browser, UI, elevation, or approval authority.

## Qualification design

The native Windows integration test derives the deterministic AppContainer SID from the exact temporary profile name before launch. It then adds one explicit ACE only to a temporary test-owned workspace directory, preserving the original DACL for restoration.

The ACE grants the AppContainer identity only the filesystem read/write/execute rights required to traverse the workspace, read the deterministic input fixture, and create the deterministic output fixture. The ACE is inherited only by children of that workspace. No user-profile root, Cotra protected-state directory, sibling resource, or caller-selected path is modified.

The existing contained executor then creates the same AppContainer profile and launches the integration-test fixture through the already-qualified suspended-process, zero-capability, explicit-handle, Job Object boundary.

## Positive evidence required

Native Windows exact-head qualification must prove that the contained child:

- reads `input.txt` inside the explicitly granted workspace;
- creates `output.txt` inside that workspace;
- runs with AppContainer identity verified;
- is assigned to the Cotra Job Object before resume;
- reaches verified Job quiescence;
- observes NUL/EOF stdin;
- receives none of the tested Cotra/tunnel/API secret-like environment values.

## Negative evidence required

The same contained child must fail to read a deterministic sibling resource outside the granted workspace. The full Windows suite must also keep the SG-000012 protected-state/authority-channel isolation regression and SG-000013 bounded PowerShell containment regression green.

## Cleanup

The test captures the workspace's original security descriptor and DACL before mutation. After contained execution, it restores the original DACL explicitly and requires that restoration to succeed before deleting the test-owned workspace and sibling fixtures. A best-effort Drop path attempts restoration if the test exits early.

## Authority boundary

No public authority changes in SG-000014.

Still absent or unchanged:

- public `powershell.run`;
- additional public `process.spawn` executable targets;
- generic executable-registry widening;
- broad filesystem or user-profile grants;
- Cotra protected-state grants;
- caller-selected ACL targets or security descriptors;
- caller environment overrides or stdin payloads;
- process network authority or PowerShell remoting;
- detached/background execution;
- public process kill;
- Git mutation;
- browser/UI automation;
- elevation;
- approval bypass or persistent approval reuse.

Public Windows `process.spawn` remains restricted to the exact SG-000010-qualified System32 `whoami.exe` target.

## Qualification policy

The implementation is not proven until the exact final head has native Windows/Ubuntu Rust, Node, Governance, genuine TypeSafe Jev, Alibaba Open Code Review, exact-diff security review, and zero unresolved blocking review threads all green. Cubic, Qodo, CodeRabbit, and similar bots are not qualification evidence.
