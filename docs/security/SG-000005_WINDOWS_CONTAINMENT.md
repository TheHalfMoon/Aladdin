# SG-000005 Windows containment decision

Date: 2026-09-24

SG-000005 does not expose process execution. It freezes the contract that future execution must satisfy.

Security decision:
- Windows AppContainer is the preferred containment target for arbitrary agent-spawned Win32 processes because its documented purpose is resource isolation.
- Restricted tokens are defense in depth for removing privileges/SID authority but are not treated as the entire sandbox.
- Job Objects are used for process-tree lifecycle/resource control, including kill-on-close where applicable, but are not treated as an authorization sandbox.
- No generic EXECUTE authority is exposed until a Windows-native successor grain proves the selected containment path against Qdral protected state and workspace access.

Primary Microsoft references:
- https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer
- https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile
- https://learn.microsoft.com/en-us/windows/win32/secauthz/restricted-tokens
- https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects

This is a concept/reference use only; no Microsoft source code is copied.
