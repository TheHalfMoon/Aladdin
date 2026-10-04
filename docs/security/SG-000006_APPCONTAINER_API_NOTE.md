# SG-000006 AppContainer API note

Date: 2026-09-24

Qdral uses the documented Windows desktop AppContainer profile APIs directly for a narrow qualification probe:

- CreateAppContainerProfile — Userenv.dll / Userenv.lib
- DeriveAppContainerSidFromAppContainerName — Userenv.dll / Userenv.lib
- DeleteAppContainerProfile — Userenv.dll / Userenv.lib
- FreeSid — used for SIDs returned by the AppContainer APIs

The probe grants zero AppContainer capabilities and launches no child process.

The Windows test creates a unique per-run profile, derives its SID, frees returned SID allocations, and deletes the profile. Failure after creation performs a cleanup attempt.

Official references:
- https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile
- https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deriveappcontainersidfromappcontainername
- https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deleteappcontainerprofile

No Microsoft source code is copied. The FFI declarations mirror the documented ABI only.
