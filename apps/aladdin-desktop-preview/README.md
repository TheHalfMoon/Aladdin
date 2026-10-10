# Aladdin Desktop — read-only Windows foundation

**Status:** experimental native user-interface grain. This is **not a production release** and does not implement the unadopted Computer Use master plan or any newly privileged action.

This is a real Windows Presentation Foundation desktop executable built entirely with the Windows .NET Framework compiler. The preview itself needs no Electron, Chromium download, NuGet packages, Node runtime, model weights, cloud AI, hosted API, administrator privileges, or network access.

## Working features

- Native Windows desktop window with a compact conversation area, this computer's identity and the live state of the local Aladdin runtime.
- Finds the per-user runtime the same way the runtime itself does (`%LOCALAPPDATA%\Qdral\bin\qdral.exe`); never searches PATH or runs a user- or model-provided path.
- Truthful runtime states, parsed from the runtime's own JSON (`qdral status|version|doctor --json`): **not installed**, **not responding**, **incompatible response**, or **installed** with its version, run state (running / degraded / not running), detail and workspace count. An error or unparsable answer is never shown as healthy.
- Read-only commands: `help`, `status`, `version`, `doctor` (health-check summary with the failing and warning checks), `devices`, `files`. Anything else is refused without action; chat text is never passed to an interpreter, operating-system command or AI service.
- Runtime queries run off the UI thread with both output streams drained concurrently (no full-pipe deadlock), output bounded to 256 KB, and the process tree killed on timeout (8 s; 30 s for `doctor`, which starts the daemon for an IPC probe).
- Folder preview: a user-selected folder is enumerated off the UI thread without materializing the directory, limited to 12 names, cancellable, and stopped after 10 s; nothing is modified or uploaded.
- Accessible controls: UI Automation ids and names on the window, command box, Send/Refresh buttons, transcript, runtime and device state; keyboard focus starts in the command box.
- Aladdin AI, voice and pairing are shown as **not connected**; no remote devices are fabricated.

## Build and test

Run these in a regular, non-elevated Windows PowerShell session from the repository root:

    powershell.exe -NoProfile -File .\apps\aladdin-desktop-preview\tests\verify.ps1
    powershell.exe -NoProfile -File .\apps\aladdin-desktop-preview\tests\e2e.ps1
    powershell.exe -NoProfile -File .\apps\aladdin-desktop-preview\tests\e2e.ps1 -RuntimeRoot <isolated LOCALAPPDATA with an installed runtime>

`verify.ps1` compiles the application with the in-box .NET Framework compiler and runs the self-tests: command allowlist and refusal of shell-shaped input, interpretation of real runtime JSON (installed, timeout, garbage, error, doctor failures, version), a stalled child stopped at its timeout, a child flooding both streams drained without deadlock and bounded, and the bounded, cancellable folder preview.

`e2e.ps1` is a native Windows end-to-end test driven through Windows UI Automation (no third-party framework). It launches the real executable, checks the runtime panel, sends `status`, a shell-like string (must be refused and start no process), and with `-RuntimeRoot` also `doctor` and `version` against a real installed runtime; it checks accessible names and keyboard focusability, measures startup and memory, and closes the window. `-RuntimeRoot` sets `LOCALAPPDATA` for the app process only, so a test install never touches the user's real one.

Measured on Windows 11 build 26300 (Intel UHD Graphics, 120 DPI, 1536×960): executable 34,816 bytes; window in 810 ms; live runtime status in 858 ms; doctor against a real install about 8 s. Memory is **higher than expected for this UI**: about 149 MB working set / 145 MB private idle, 228 MB / 214 MB after runtime queries. This is an open performance item, not a qualified result.

All generated binaries go to the user's TEMP directory. A signed installer and installed-size qualification have not been performed.

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
