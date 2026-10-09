# Aladdin Desktop — read-only Windows foundation

**Status:** experimental native user-interface grain. This is **not a production release** and does not implement the unadopted Computer Use master plan or any newly privileged action.

This is a real Windows Presentation Foundation desktop executable built entirely with the Windows .NET Framework compiler. The preview itself needs no Electron, Chromium download, NuGet packages, Node runtime, model weights, cloud AI, hosted API, administrator privileges, or network access.

## Working features

- Native dark Windows desktop window with a conversation-oriented local command area, machine identity and runtime status.
- Detects the existing per-user Aladdin/Qdral compatibility executable at %LOCALAPPDATA%\Qdral\bin\qdral.exe.
- Provides fixed, read-only status and version CLI queries with a five-second timeout. Never passes the chat text to an interpreter, operating-system command or AI service.
- Shows the local computer and makes it clear when the Aladdin runtime is not installed. There are no fabricated remote devices.
- User-selected folder browser with an explicitly read-only list of at most 12 entry names; no data leaves the machine.
- Read-only commands: help, status, version, devices, and files. Unrecognized requests are rejected without action.
- Clearly marks Aladdin AI, Hala One, Reliance, voice and remote pairing as **not configured**.

## Build and test

Run these in a regular, non-elevated Windows PowerShell session from the repository root:

    powershell.exe -NoProfile -File .\apps\aladdin-desktop-preview\tests\verify.ps1
    & "$env:TEMP\AladdinDesktopPreviewBuild\Aladdin.Desktop.exe"

The test compiles the application and console smoke-test binary, validates a closed allowlist of read-only commands, checks that shell-injection-shaped input is rejected, and enforces a preview executable size budget. It does **not** automatically start the desktop GUI.

All generated binaries go to the user's TEMP directory. A future signed installer and installed-size qualification have not been performed.

## Security boundary

The preview does not create a network listener, model connection, browser automation service, input actuation, shell execution API, file mutation, remote pairing endpoint, or delegated approval. It never attempts to elevate privileges.

Only fixed read-only compatibility CLI operations may be invoked from the known per-user installation path. The existing Aladdin/Qdral core remains the policy authority; the preview does not duplicate its grants or bypass local approvals.

Security finding #278 remains open and blocks promotion of ledger-backed production actions until independent qualification.

## Follow-up engineering gates

1. Evaluate long-term native UI stack against actual memory, accessibility, update-signing and cold-start measurements; this WPF prototype does not dictate the final cross-platform stack.
2. Integrate a reviewed read-only status IPC instead of repeatedly spawning the compatibility CLI.
3. Adopt the existing authorization broker before any action preview/approval/execution UI.
4. Build secure multi-device enrollment/status and revocation, with no default raw remote control.
5. Design cloud-only Aladdin AI API integration with authenticated task IDs, user-granted screen/data sharing and explicit model unavailability, then add voice.
6. Prove native E2E, signed delivery, security review, independent tests, product packaging and installer/download size before release.

See master planning issue #279, identity #281 and security blocker #278 in the Aladdin GitHub repository.
