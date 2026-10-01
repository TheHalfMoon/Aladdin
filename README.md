# Cotra

**Computer Orchestration & Trusted Runtime Access**

Cotra is an open-source, local-first MCP gateway that lets ChatGPT work with an explicitly trusted folder on your Windows computer through a small set of bounded tools, with local approval for anything that changes your computer. It connects through the OpenAI Secure MCP Tunnel, so no inbound port is opened on your machine.

Cotra is not a remote shell, a remote desktop, or a "run anything" agent.

## What ChatGPT can do through Cotra

Exactly these 20 tools are exposed over MCP:

| Area | Tools | Approval |
|---|---|---|
| Status | `system_status`, `workspace_get` | none (read-only) |
| Files in a trusted workspace | `fs_stat`, `fs_list`, `fs_read`, `fs_search` | none (read-only, bounded) |
| File writes | `fs_write_preview`, `fs_write` | local approval for each write, bound to the exact content and current file hash |
| Git (local) | `git_status`, `git_diff`, `git_log` | none (read-only) |
| Git (local changes) | `git_branch_create`, `git_stage`, `git_unstage`, `git_commit` | local approval, bound to the exact repository state |
| Git (network) | `git_fetch_preview`, `git_fetch`, `git_push_preview`, `git_push` | local approval; see limitations |
| Processes | `process_spawn` | local approval; see limitations |

Everything else is denied. Cotra also contains capabilities that are deliberately **not** exposed to ChatGPT in this release (browser automation, Windows UI Automation, screenshots and coordinate input, clipboard, and destination-scoped HTTPS fetch); they cannot be reached over MCP.

### Current limitations

- `process_spawn` on Windows is restricted to `whoami.exe`, run in an isolated AppContainer.
- `git_fetch` and `git_push` require destination policies that `cotra` does not configure yet, so they fail closed in an installed Cotra.
- Release binaries are not code-signed (there is no paid certificate under the project's zero-cost rule). Windows SmartScreen may warn; verify the archive with `SHA256SUMS.txt` and the GitHub build-provenance attestation.
- Cotra runs only while you are signed in, because approvals appear on your desktop.

## How approvals work

- **SOFT approval** — a Cotra dialog appears on your desktop describing the exact action (for example, the file and content hash). Approve or deny it yourself. Each approval is used once, expires quickly, and is bound to that exact action, workspace, and policy.
- **STRONG approval** — Windows Hello (PIN, fingerprint, or face) for trust changes and emergency revoke.
- ChatGPT cannot approve its own actions: approvals come only from the local broker, and Cotra's own windows are excluded from automation.
- `cotra approvals` lists recent decisions; `cotra emergency-revoke` invalidates every pending approval.

## Requirements

- Windows 10 version 1809 (build 17763) or later, or Windows 11, x64.
- Node.js 20 or later.
- The official OpenAI tunnel client executable, and a Secure MCP Tunnel id (`tunnel_…`) and runtime key from OpenAI for your ChatGPT workspace. Follow OpenAI's documentation to create them; Cotra does not create, download, or redistribute them.
- No administrator rights. Do not use "Run as administrator".

## Install

1. Download `cotra-<version>-windows-x64.zip` and `SHA256SUMS.txt` from the project's GitHub Releases page (release automation only prepares drafts; a release appears there once the maintainer publishes it) and verify the hash:
   ```powershell
   $zip = "cotra-<version>-windows-x64.zip"
   $expected = ((Get-Content .\SHA256SUMS.txt) | Where-Object { $_ -like "*  $zip" } | ForEach-Object { $_.Split(" ")[0] })
   $actual = (Get-FileHash ".\$zip" -Algorithm SHA256).Hash.ToLower()
   if (-not $expected -or $actual -ne $expected) { throw "checksum mismatch: do not extract $zip" } else { "checksum OK" }
   ```
   On Linux or WSL, `sha256sum -c --ignore-missing SHA256SUMS.txt` does the same.
   Optionally verify provenance: `gh attestation verify .\cotra-<version>-windows-x64.zip --repo TheHalfMoon/Cotra`.
2. Extract the archive and run, from the extracted folder:
   `.\cotra.exe install`
   Every file is checked against `manifest.json` before anything is copied. Cotra installs to `%LOCALAPPDATA%\Cotra`, readable only by you (and Windows itself), and adds `%LOCALAPPDATA%\Cotra\bin` to your user PATH (`--no-path` to skip). Open a new terminal afterwards.

## Configure

1. Add a workspace (a folder ChatGPT may use):
   `cotra workspace add myproject C:\path\to\project`
   Folders that contain Cotra's own data (for example your whole user profile) are refused.
2. Optionally grant workspace trust with Windows Hello: `cotra workspace trust myproject`.
3. Configure the tunnel:
   `cotra tunnel setup --client C:\path\to\tunnel-client.exe --tunnel-id tunnel_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
   You are prompted for the runtime key without echo (or pass `--key-file <path>`). The key is stored only in Cotra's protected folder, passed to the tunnel client by file reference, and never printed or logged.

## Connect ChatGPT

1. `cotra start` — starts the official tunnel client under Cotra's supervisor. It reports `running` only when the tunnel client is alive and its local health endpoint answers.
2. In ChatGPT, connect to your Secure MCP Tunnel as described in OpenAI's documentation. ChatGPT then sees the 20 tools above.

## Everyday commands

| Command | Purpose |
|---|---|
| `cotra status` | Installed version, configuration, and whether the runtime is running |
| `cotra doctor` | Full diagnostics; exits non-zero if any check fails (`--json` for details) |
| `cotra stop` | Stops the tunnel client and everything it started, and confirms they exited |
| `cotra workspace list` | Workspaces and their trust state |
| `cotra tunnel show` | Tunnel configuration (never the key) |
| `cotra help` | All commands |

## Diagnose problems

Run `cotra doctor`. Each check reports `PASS`, `WARN`, `FAIL`, or `????` (could not be checked) with one line of evidence — Cotra never reports health it did not observe. Logs are in `%LOCALAPPDATA%\Cotra\logs` (`tunnel.log`, `supervisor.log`, `lifecycle.log`); tunnel output is redacted before it is written.

## Update

Cotra never checks for or downloads updates by itself.

1. Download and verify the new release as in Install, then extract it.
2. `cotra update --source C:\path\to\extracted-release --check` shows the version change.
3. `cotra update --source C:\path\to\extracted-release` stops Cotra, switches versions, has the new version verify itself, and restarts it. If the new version fails its own check, the previous version is restored automatically and `cotra doctor` reports why.
4. `cotra rollback` returns to the previous version. Older versions require `--allow-downgrade`.

## Uninstall

- `cotra uninstall` stops Cotra and removes its programs and the PATH entry. Your configuration, tunnel key, logs, and audit/approval/trust history are **kept** and listed.
- `cotra uninstall --purge-data --yes` also deletes that data.
- Your workspaces are never touched.

## Security model and governance

- [Architecture and delivery plan](docs/canonical/ARCHITECTURE_AND_DELIVERY_PLAN.md)
- [Current canonical state](docs/canonical/CURRENT.md)
- [Threat model](docs/security/THREAT_MODEL.md) and [regression matrix](docs/security/THREAT_MODEL_REGRESSION.md)
- [Installer and lifecycle design](docs/canonical/P12_INSTALLER_LIFECYCLE_DESIGN.md)
- [Dependency and license review](docs/research/DEPENDENCY_LICENSE_REVIEW.md)
- [Diffcipline](docs/governance/DIFFCIPLINE.md)

## Building from source

`npm ci --ignore-scripts && npm run build`, then `cargo build --release --locked -p cotrad -p cotra-lifecycle --bins`, then `node scripts/package-release.mjs --out <dir>` to assemble a release directory and `node scripts/archive-release.mjs --release <dir> --out <dir>.zip` to archive it. CI builds with the deterministic flags in `.github/actions/build-package-qualify/action.yml` and checks that an independent rebuild is byte-identical.

## License

Apache License 2.0. Third-party components and their licenses are listed in `THIRD_PARTY_NOTICES.txt` in each release.
