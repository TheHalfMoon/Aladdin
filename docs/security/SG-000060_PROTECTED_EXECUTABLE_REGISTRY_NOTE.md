# SG-000060 Protected Executable Registry Note

Status: IMPLEMENTATION FOR COTRA-P16
SpecGrain: SG-000060
Base: `79ba16a3fbe167edb7ddb846f7c7556144bc6abf`
Date: 2026-10-02
Code: `crates/cotra-policy/src/executable_registry.rs`,
`crates/cotra-policy/src/lib.rs` (`enforce_sg000010_executable_policy`),
`crates/cotrad/src/executable_admin.rs`, `crates/cotrad/src/main.rs`
(`verify_registered_identity`), `crates/cotra-lifecycle/src/commands.rs`
(`cotra exec`).

## 1. What changes

The public `process_spawn` ceiling was one executable (`whoami.exe`).
SG-000060 adds a local registry so everyday native build and test tools can
run through the same process path. Nothing else about that path changes:
argv-only execution, workspace-relative cwd, AppContainer containment with
job-object descendant control, the `NONE` network class, timeouts, output
bounds, a null stdin, environment and secret filtering, and a fresh per-run
approval bound to the exact executable, argv, cwd, limits, and environment.
The MCP tool and its schema are unchanged.

## 2. Registration

- Only `cotra exec add <id> <path.exe>` on the local machine registers an
  executable. cotrad requires STRONG platform presence bound to a digest of
  the exact entry (id, canonical path, SHA-256, size, subcommands, denied
  arguments, argument bound) and re-hashes the file after approval; any change
  during approval fails closed.
- `executable.*` capabilities are local-only: remote contexts get
  `CAPABILITY_DENIED` and no MCP tool exists. Removal is authority-reducing
  and needs no approval.
- Registrable: native `.exe` images with an absolute, non-UNC, non-device
  path, at most 512 MiB, outside every configured workspace (workspaces are
  agent-writable).
- Never registrable, whatever the path: shells, script hosts, interpreters,
  runtime launchers, remote shells, and system administration tools (`cmd`,
  `powershell`, `pwsh`, `bash`, `wsl`, `python`, `node`, `deno`, `ruby`,
  `perl`, `java`, `wscript`, `cscript`, `mshta`, `rundll32`, `regsvr32`,
  `msiexec`, `certutil`, `curl`, `ssh`, `psexec`, `wmic`, `msbuild`, and the
  rest of `DENIED_EXECUTABLE_STEMS`, including versioned names such as
  `python3.12`), and every non-`.exe` file (`.cmd`, `.bat`, `.ps1`, `.vbs`,
  `.js`, `.lnk`, `.com`, `.msi`, and extension-less files). Package-runner
  scripts such as `npm.cmd` are therefore denied.
- The registry is an integrity-checked JSON document in protected state
  (`%LOCALAPPDATA%\Cotra\executable_registry.json`, override
  `COTRA_EXECUTABLE_REGISTRY_PATH`, now a protected-state override). A
  missing, corrupt, tampered, or oversized registry yields no entries.

## 3. Launch

1. Policy: the canonical executable must be the baseline or a registered
   entry, re-checked against the denylist, with argv matching the entry
   grammar: `argv[0]` must be a registered subcommand (or argv empty when
   none are registered), argv length within the entry bound, and no denied
   argument (exact or `name=` form).
2. Approval: the existing per-run SOFT approval.
3. Immediately before launch: the entry is looked up again and the file's
   canonical path, size, and SHA-256 are re-verified. Any drift is
   `TARGET_STALE` and nothing runs.

Identity is never path-only. A registered tool updated in place (for example
by its own updater) must be re-registered locally.

## 4. Residual risk

- A same-user process that can already write the executable between the
  final hash check and process creation is outside the strict model, as
  documented for protected state; the hash check closes every window the
  agent controls.
- Registered build and test tools run project code by design (build
  scripts, tests). That code runs inside the existing AppContainer with no
  network and workspace-bounded file access, under a timeout and output
  bounds, with per-run approval.

## 5. Evidence

- `executable_registry` unit tests: denylisted names and versioned variants,
  script and shortcut types, UNC and relative paths, workspace executables,
  strict ids and tokens, the argv grammar (subcommands, denied arguments,
  `name=` forms, bounds, empty grammars), modification and removal failing
  closed, and a tamper-evident store.
- `executable_admin` tests: STRONG presence required (denied, unavailable,
  and SOFT-only brokers create nothing), hash-pinned entries, removal without
  approval, interpreter, workspace, and extra-field rejection.
- `sg000060_registered_executables_run_contained_and_drift_fails_closed`:
  on real Windows (run under PowerShell on Windows 11 Home build 26200, and
  in CI), an unregistered `hostname.exe` is denied, a registered one runs
  inside the verified AppContainer and job object with exit code 0 and the
  `NONE` network class, an argument outside its grammar and `cmd.exe` are
  denied, and a drifted hash is refused with `TARGET_STALE` before launch.
- The parity inventory now records `run_build_and_test_tools` as exposed
  through registered executables and the registry management shapes as
  intentionally denied.
