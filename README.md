# Quntal

**Computer Orchestration & Trusted Runtime Access**

Quntal is an open-source, local-first MCP gateway that lets ChatGPT work with an explicitly trusted folder on your Windows computer through a small set of bounded tools, with local approval for anything that changes your computer. It connects through the OpenAI Secure MCP Tunnel, so no inbound port is opened on your machine.

Quntal is not a remote shell, a remote desktop, or a "run anything" agent.

## What ChatGPT can do through Quntal

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

Everything else is denied. Quntal also contains capabilities that are deliberately **not** exposed to ChatGPT in this release (browser automation, Windows UI Automation, screenshots and coordinate input, clipboard, and destination-scoped HTTPS fetch); they cannot be reached over MCP.

### Current limitations

- `process_spawn` on Windows is restricted to `whoami.exe`, run in an isolated AppContainer.
- `git_fetch` and `git_push` require destination policies that `quntal` does not configure yet, so they fail closed in an installed Quntal.
- Release binaries are not code-signed (there is no paid certificate under the project's zero-cost rule). Windows SmartScreen may warn; verify the archive with `SHA256SUMS.txt` and the GitHub build-provenance attestation.
- Quntal runs only while you are signed in, because approvals appear on your desktop.

## How approvals work

- **SOFT approval** — a Quntal dialog appears on your desktop describing the exact action (for example, the file and content hash). Approve or deny it yourself. Each approval is used once, expires quickly, and is bound to that exact action, workspace, and policy.
- **STRONG approval** — Windows Hello (PIN, fingerprint, or face) for trust changes and emergency revoke.
- ChatGPT cannot approve its own actions: approvals come only from the local broker, and Quntal's own windows are excluded from automation.
- `quntal approvals` lists recent decisions; `quntal emergency-revoke` invalidates every pending approval.

## Requirements

- Windows 10 version 1809 (build 17763) or later, or Windows 11, x64.
- Node.js 20 or later.
- The official OpenAI tunnel client executable, and a Secure MCP Tunnel id (`tunnel_…`) and runtime key from OpenAI for your ChatGPT workspace. Follow OpenAI's documentation to create them; Quntal does not create, download, or redistribute them.
- No administrator rights. Do not use "Run as administrator".

## Install

1. Download `quntal-<version>-windows-x64.zip` and `SHA256SUMS.txt` from the project's GitHub Releases page (release automation only prepares drafts; a release appears there once the maintainer publishes it) and verify the hash:
   ```powershell
   $zip = "quntal-<version>-windows-x64.zip"
   $expected = ((Get-Content .\SHA256SUMS.txt) | Where-Object { $_ -like "*  $zip" } | ForEach-Object { $_.Split(" ")[0] })
   $actual = (Get-FileHash ".\$zip" -Algorithm SHA256).Hash.ToLower()
   if (-not $expected -or $actual -ne $expected) { throw "checksum mismatch: do not extract $zip" } else { "checksum OK" }
   ```
   On Linux or WSL, `sha256sum -c --ignore-missing SHA256SUMS.txt` does the same.
   Optionally verify provenance: `gh attestation verify .\quntal-<version>-windows-x64.zip --repo TheHalfMoon/Quntal`.
2. Extract the archive and run, from the extracted folder:
   `.\quntal.exe install`
   Every file is checked against `manifest.json` before anything is copied. Quntal installs to `%LOCALAPPDATA%\Quntal`, readable only by you (and Windows itself), and adds `%LOCALAPPDATA%\Quntal\bin` to your user PATH (`--no-path` to skip). Open a new terminal afterwards.

## Configure

1. Add a workspace (a folder ChatGPT may use):
   `quntal workspace add myproject C:\path\to\project`
   Folders that contain Quntal's own data (for example your whole user profile) are refused.
2. Optionally grant workspace trust with Windows Hello: `quntal workspace trust myproject`.
3. Configure the tunnel:
   `quntal tunnel setup --client C:\path\to\tunnel-client.exe --tunnel-id tunnel_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
   You are prompted for the runtime key without echo (or pass `--key-file <path>`). The key is stored only in Quntal's protected folder, passed to the tunnel client by file reference, and never printed or logged.

## Connect ChatGPT

1. `quntal start` — starts the official tunnel client under Quntal's supervisor. It reports `running` only when the tunnel client is alive and its local health endpoint answers.
2. In ChatGPT, connect to your Secure MCP Tunnel as described in OpenAI's documentation. ChatGPT then sees the 20 tools above.

## Connect local MCP clients (no tunnel required)

Local clients that can launch a command use `quntal mcp stdio`. It starts the same authoritative MCP server over standard input and output, with the same 20 tools and the same local approvals. No tunnel setup is needed. Standard output stays the MCP channel, so run it only through the client configuration below, not by hand.

Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "quntal": {
      "command": "quntal",
      "args": ["mcp", "stdio"]
    }
  }
}
```

Codex and other command-based clients use the same command and arguments:

```json
{
  "command": "quntal",
  "args": ["mcp", "stdio"]
}
```

Mistral Vibe Code accepts a stdio entry in its MCP server list with command `quntal` and arguments `["mcp", "stdio"]`. Generic MCP clients and the MCP Inspector connect with transport `stdio`, command `quntal`, and arguments `["mcp", "stdio"]`.

`quntal` here is `%LOCALAPPDATA%\Quntal\bin\quntal.exe` on your user PATH after install. It resolves the active verified release on every launch, refuses tampered or inactive payloads, and fails closed when no workspace is configured. Tested per-client files live in [`examples/mcp-clients`](examples/mcp-clients) (Claude Desktop JSON and Desktop Extension manifest, Codex TOML, Vibe Code TOML, generic shapes, and the Inspector qualification script).

## Connect over loopback HTTP (no tunnel required)

`quntal mcp serve` exposes the same 20 tools over Streamable HTTP on `127.0.0.1` only. It never binds a LAN address. Every request needs a per-user bearer credential:

```powershell
$env:QUNTAL_LOOPBACK_TOKEN = "<at-least-32-characters-you-choose>"
quntal mcp serve
```

The listener prints its URL, for example `http://127.0.0.1:54321/mcp`, on standard error. Pass `--port <1-65535>` to choose the port instead of an ephemeral one. The credential is required on every request as `Authorization: Bearer <token>`; missing or wrong credentials fail closed, as do forged `Host` or browser `Origin` values. Browser pages on other origins cannot drive the listener. Keep the token on your machine and never commit it.

Mistral Vibe Code HTTP mode and generic Streamable HTTP clients connect to the printed URL with the bearer header. Clients that can launch a command should prefer `quntal mcp stdio` above.

## Everyday commands

| Command | Purpose |
|---|---|
| `quntal status` | Installed version, configuration, and whether the runtime is running |
| `quntal doctor` | Full diagnostics; exits non-zero if any check fails (`--json` for details) |
| `quntal stop` | Stops the tunnel client and everything it started, and confirms they exited |
| `quntal workspace list` | Workspaces and their trust state |
| `quntal tunnel show` | Tunnel configuration (never the key) |
| `quntal help` | All commands |

## Diagnose problems

Run `quntal doctor`. Each check reports `PASS`, `WARN`, `FAIL`, or `????` (could not be checked) with one line of evidence — Quntal never reports health it did not observe. Logs are in `%LOCALAPPDATA%\Quntal\logs` (`tunnel.log`, `supervisor.log`, `lifecycle.log`); tunnel output is redacted before it is written.

## Update

Quntal never checks for or downloads updates by itself.

1. Download and verify the new release as in Install, then extract it.
2. `quntal update --source C:\path\to\extracted-release --check` shows the version change.
3. `quntal update --source C:\path\to\extracted-release` stops Quntal, switches versions, has the new version verify itself, and restarts it. If the new version fails its own check, the previous version is restored automatically and `quntal doctor` reports why.
4. `quntal rollback` returns to the previous version. Older versions require `--allow-downgrade`.

## Uninstall

- `quntal uninstall` stops Quntal and removes its programs and the PATH entry. Your configuration, tunnel key, logs, and audit/approval/trust history are **kept** and listed.
- `quntal uninstall --purge-data --yes` also deletes that data.
- Your workspaces are never touched.

## Security model and governance

- [Architecture and delivery plan](docs/canonical/ARCHITECTURE_AND_DELIVERY_PLAN.md)
- [Current canonical state](docs/canonical/CURRENT.md)
- [Threat model](docs/security/THREAT_MODEL.md) and [regression matrix](docs/security/THREAT_MODEL_REGRESSION.md)
- [Installer and lifecycle design](docs/canonical/P12_INSTALLER_LIFECYCLE_DESIGN.md)
- [Dependency and license review](docs/research/DEPENDENCY_LICENSE_REVIEW.md)
- [Diffcipline](docs/governance/DIFFCIPLINE.md)

## Building from source

`npm ci --ignore-scripts && npm run build`, then `cargo build --release --locked -p quntald -p quntal-lifecycle --bins`, then `node scripts/package-release.mjs --out <dir>` to assemble a release directory and `node scripts/archive-release.mjs --release <dir> --out <dir>.zip` to archive it. CI builds with the deterministic flags in `.github/actions/build-package-qualify/action.yml` and checks that an independent rebuild is byte-identical.

## License

Apache License 2.0. Third-party components and their licenses are listed in `THIRD_PARTY_NOTICES.txt` in each release.
