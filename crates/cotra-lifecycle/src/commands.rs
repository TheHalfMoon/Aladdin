//! Runtime, configuration, and trust commands of the `cotra` CLI.

use crate::{Args, Output};
use cotra_lifecycle::config::{self, Config};
use cotra_lifecycle::doctor::{self, CheckStatus};
use cotra_lifecycle::install::Installer;
use cotra_lifecycle::layout::Layout;
use cotra_lifecycle::lifecycle::{self, RunState, StopOutcome};
use cotra_lifecycle::{host_platform, ipc, LifecycleError};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

const RESTART_HINT: &str =
    "Restart Cotra (`cotra stop` then `cotra start`) for running sessions to pick up the change.\n";

fn active_version(layout: &Layout) -> Result<String, LifecycleError> {
    let platform = host_platform();
    Ok(Installer::new(layout.clone(), platform.as_ref())
        .verify()?
        .active
        .version)
}

pub fn workspace(args: &mut Args) -> Result<Output, LifecycleError> {
    let layout = Layout::for_current_user()?;
    let action = args.positional("workspace action (add, remove, list, trust, untrust)")?;
    match action.as_str() {
        "add" => {
            let id = args.positional("workspace id")?;
            let dir = PathBuf::from(args.positional("workspace directory")?);
            args.finish()?;
            let mut config = Config::load(&layout)?;
            let entry = config.add_workspace(&id, &dir)?;
            config.save(&layout)?;
            Ok(Output {
                exit_code: 0,
                human: format!(
                    "Workspace {} added: {}\nIt is not trusted for STRONG-gated operations until `cotra workspace trust {}`.\n{RESTART_HINT}",
                    entry.id,
                    entry.root.display(),
                    entry.id
                ),
                json: json!({"ok": true, "workspace": entry}),
            })
        }
        "remove" => {
            let id = args.positional("workspace id")?;
            args.finish()?;
            let mut config = Config::load(&layout)?;
            let entry = config.remove_workspace(&id)?;
            config.save(&layout)?;
            Ok(Output {
                exit_code: 0,
                human: format!(
                    "Workspace {} removed from configuration. Files in {} were not touched.\n{RESTART_HINT}",
                    entry.id,
                    entry.root.display()
                ),
                json: json!({"ok": true, "removed": entry}),
            })
        }
        "list" => {
            args.finish()?;
            let config = Config::load(&layout)?;
            let version = active_version(&layout).ok();
            let mut rows = Vec::new();
            let mut human = String::new();
            for entry in &config.workspaces {
                let trust = version.as_ref().and_then(|version| {
                    let cotrad = layout.version_dir(version).join("cotrad.exe");
                    let request = ipc::request(&entry.id, "workspace.trust.get", "get", json!({}));
                    ipc::call(&cotrad, &config, &request, Duration::from_secs(20))
                        .ok()
                        .and_then(|response| ipc::expect_ok(&response).ok().cloned())
                });
                let trusted = trust
                    .as_ref()
                    .and_then(|trust| trust.get("trusted"))
                    .and_then(Value::as_bool);
                human.push_str(&format!(
                    "{:<20} {:<9} {}\n",
                    entry.id,
                    match trusted {
                        Some(true) => "trusted",
                        Some(false) => "untrusted",
                        None => "unknown",
                    },
                    entry.root.display()
                ));
                rows.push(json!({"id": entry.id, "root": entry.root, "trust": trust}));
            }
            if rows.is_empty() {
                human.push_str("No workspaces configured. Add one with `cotra workspace add <id> <directory>`.\n");
            }
            Ok(Output {
                exit_code: 0,
                human,
                json: json!({"ok": true, "workspaces": rows}),
            })
        }
        "trust" | "untrust" => {
            let id = args.positional("workspace id")?;
            args.finish()?;
            let config = Config::load(&layout)?;
            config.check_workspaces_with_policy()?;
            if !config
                .workspaces
                .iter()
                .any(|entry| entry.id.eq_ignore_ascii_case(&id))
            {
                return Err(LifecycleError::usage(format!(
                    "workspace {id} is not configured"
                )));
            }
            let version = active_version(&layout)?;
            let cotrad = layout.version_dir(&version).join("cotrad.exe");
            let (capability, operation) = if action == "trust" {
                ("workspace.trust.grant", "grant")
            } else {
                ("workspace.trust.revoke", "revoke")
            };
            eprintln!("Confirm with Windows Hello when prompted. This is a STRONG approval and cannot be granted by an agent.");
            let configured = config
                .workspaces
                .iter()
                .find(|entry| entry.id.eq_ignore_ascii_case(&id))
                .map(|entry| entry.id.clone())
                .unwrap_or(id);
            let request = ipc::request(&configured, capability, operation, json!({}));
            let response = ipc::call(&cotrad, &config, &request, Duration::from_secs(180))?;
            let result = ipc::expect_ok(&response)?.clone();
            Ok(Output {
                exit_code: 0,
                human: format!(
                    "Workspace {configured} is now {} (trust revision {}).\n",
                    if result.get("trusted").and_then(Value::as_bool) == Some(true) {
                        "trusted"
                    } else {
                        "untrusted"
                    },
                    result
                        .get("revision")
                        .and_then(Value::as_u64)
                        .unwrap_or_default()
                ),
                json: json!({"ok": true, "trust": result}),
            })
        }
        other => Err(LifecycleError::usage(format!(
            "unknown workspace action {other:?}; run `cotra help`"
        ))),
    }
}

pub fn tunnel(args: &mut Args) -> Result<Output, LifecycleError> {
    let layout = Layout::for_current_user()?;
    let action = args.positional("tunnel action (setup, show)")?;
    match action.as_str() {
        "setup" => {
            let client = PathBuf::from(args.value("--client")?.ok_or_else(|| {
                LifecycleError::usage("--client <tunnel-client.exe> is required")
            })?);
            let tunnel_id = args
                .value("--tunnel-id")?
                .ok_or_else(|| LifecycleError::usage("--tunnel-id <tunnel_...> is required"))?;
            let key_file = args.value("--key-file")?.map(PathBuf::from);
            args.finish()?;
            config::validate_tunnel_id(&tunnel_id)?;
            if !client.is_absolute() || !client.is_file() {
                return Err(LifecycleError::usage(
                    "--client must be the absolute path of the official tunnel-client executable",
                ));
            }
            let version = active_version(&layout)?;
            let mut config = Config::load(&layout)?;
            config.check_workspaces_with_policy()?;
            let key = match &key_file {
                Some(path) => std::fs::read_to_string(path)
                    .map_err(|error| LifecycleError::io("read --key-file", error))?
                    .trim()
                    .to_string(),
                None => cotra_lifecycle::console::read_secret_line(
                    "Tunnel runtime key (input hidden): ",
                )?,
            };
            config::validate_runtime_key(&key)?;
            let previous = config.clone();
            config.tunnel = Some(config::TunnelSettings {
                client: client.clone(),
                tunnel_id: tunnel_id.clone(),
            });
            // Key and configuration change together: any failure after the key
            // is written restores both the previous key and configuration.
            let previous_key = std::fs::read(layout.runtime_key_file()).ok();
            config::store_runtime_key(&layout, &key)?;
            drop(key);
            let platform = host_platform();
            let committed = config::tunnel_config(&layout, &config, &version)
                .and_then(|_| config.save(&layout))
                .and_then(|()| platform.verify_tree_acl(&layout.root));
            if let Err(error) = committed {
                let restored_key = match previous_key {
                    Some(bytes) => cotra_lifecycle::layout::write_bytes_atomic(
                        &layout.runtime_key_file(),
                        &bytes,
                    ),
                    None => std::fs::remove_file(layout.runtime_key_file())
                        .or_else(|error| match error.kind() {
                            std::io::ErrorKind::NotFound => Ok(()),
                            _ => Err(error),
                        })
                        .map_err(|error| LifecycleError::io("remove new runtime key", error)),
                };
                let restored_config = previous.save(&layout);
                return Err(match (restored_key, restored_config) {
                    (Ok(()), Ok(())) => error,
                    _ => LifecycleError::state(format!(
                        "tunnel setup failed ({}) and the previous key or configuration could not be fully restored; rerun `cotra tunnel setup`",
                        error.message
                    )),
                });
            }
            let mut human = format!(
                "Tunnel {tunnel_id} configured with client {}.\nThe runtime key is stored in {} (owner-only) and is passed to the tunnel client by file reference only.\n",
                client.display(),
                layout.runtime_key_file().display()
            );
            if let Some(path) = &key_file {
                human.push_str(&format!(
                    "You may now delete the key file you supplied: {}\n",
                    path.display()
                ));
            }
            human.push_str("Next: `cotra start`, then `cotra status` or `cotra doctor`.\n");
            Ok(Output {
                exit_code: 0,
                human,
                json: json!({"ok": true, "tunnel_id": tunnel_id, "client": client, "key_file": layout.runtime_key_file()}),
            })
        }
        "show" => {
            args.finish()?;
            let config = Config::load(&layout)?;
            let key_present = std::fs::metadata(layout.runtime_key_file())
                .map(|metadata| metadata.len() > 0)
                .unwrap_or(false);
            let human = match &config.tunnel {
                Some(tunnel) => format!(
                    "tunnel id: {}\nclient: {}\nruntime key: {} (value never displayed)\n",
                    tunnel.tunnel_id,
                    tunnel.client.display(),
                    if key_present { "present" } else { "missing" }
                ),
                None => "The tunnel is not configured. Run `cotra tunnel setup`.\n".into(),
            };
            Ok(Output {
                exit_code: 0,
                human,
                json: json!({"ok": true, "tunnel": config.tunnel, "runtime_key_present": key_present}),
            })
        }
        other => Err(LifecycleError::usage(format!(
            "unknown tunnel action {other:?}; run `cotra help`"
        ))),
    }
}

fn wait_seconds(args: &mut Args, default: u64) -> Result<Duration, LifecycleError> {
    match args.value("--wait")? {
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|seconds| (1..=600).contains(seconds))
            .map(Duration::from_secs)
            .ok_or_else(|| LifecycleError::usage("--wait takes 1-600 seconds")),
        None => Ok(Duration::from_secs(default)),
    }
}

fn describe(status: &lifecycle::RuntimeStatus) -> String {
    let state = match status.state {
        RunState::Running => "running",
        RunState::Degraded => "degraded",
        RunState::NotRunning => "not running",
    };
    let mut text = format!("Cotra is {state}: {}\n", status.detail);
    if let Some(pid) = status.supervisor_pid {
        text.push_str(&format!("  supervisor pid: {pid}\n"));
    }
    if let Some(pid) = status.tunnel_client_pid {
        text.push_str(&format!("  tunnel client pid: {pid}\n"));
    }
    if let Some(url) = &status.health_url {
        text.push_str(&format!("  health URL: {url}\n"));
    }
    if let Some(last) = &status.last_exit {
        text.push_str(&format!(
            "  last exit: {} (exit code {:?})\n",
            last.reason, last.exit_code
        ));
    }
    text
}

pub fn start(args: &mut Args) -> Result<Output, LifecycleError> {
    let wait = wait_seconds(args, 20)?;
    args.finish()?;
    let layout = Layout::for_current_user()?;
    let platform = host_platform();
    let status = lifecycle::start(&layout, platform.as_ref(), wait)?;
    Ok(Output {
        exit_code: 0,
        human: describe(&status),
        json: json!({"ok": status.state != RunState::NotRunning, "status": status}),
    })
}

pub fn stop(args: &mut Args) -> Result<Output, LifecycleError> {
    let wait = wait_seconds(args, 20)?;
    args.finish()?;
    let layout = Layout::for_current_user()?;
    let report = lifecycle::stop(&layout, wait)?;
    let human = match report.outcome {
        StopOutcome::NotRunning => "Cotra is not running.\n".to_string(),
        StopOutcome::StaleRecordRemoved => format!("Cotra was not running; {}.\n", report.detail),
        StopOutcome::StoppedVerified => format!("Cotra stopped (verified): {}.\n", report.detail),
        StopOutcome::SupervisorTerminated => format!("Cotra stopped: {}.\n", report.detail),
    };
    Ok(Output {
        exit_code: 0,
        human,
        json: json!({"ok": true, "stop": report}),
    })
}

pub fn status(args: &mut Args) -> Result<Output, LifecycleError> {
    args.finish()?;
    let layout = Layout::for_current_user()?;
    let installed: Option<cotra_lifecycle::layout::CurrentRecord> =
        cotra_lifecycle::layout::read_current(&layout)?;
    let config = Config::load(&layout);
    let runtime = lifecycle::status(&layout)?;
    let mut human = match &installed {
        Some(current) => format!("installed: Cotra {}\n", current.version),
        None => "installed: no\n".to_string(),
    };
    match &config {
        Ok(config) => human.push_str(&format!(
            "configured: {} workspace(s); tunnel {}\n",
            config.workspaces.len(),
            if config.tunnel.is_some() {
                "configured"
            } else {
                "not configured"
            }
        )),
        Err(error) => human.push_str(&format!("configured: error: {}\n", error.message)),
    }
    human.push_str(&describe(&runtime));
    Ok(Output {
        exit_code: 0,
        human,
        json: json!({
            "ok": true,
            "installed": installed.map(|current| current.version),
            "workspaces": config.as_ref().map(|config| config.workspaces.len()).ok(),
            "tunnel_configured": config.as_ref().map(|config| config.tunnel.is_some()).ok(),
            "runtime": runtime,
        }),
    })
}

pub fn doctor(args: &mut Args) -> Result<Output, LifecycleError> {
    args.finish()?;
    let layout = Layout::for_current_user()?;
    let platform = host_platform();
    let report = doctor::run(&layout, platform.as_ref());
    let mut human = String::new();
    for check in &report.checks {
        let mark = match check.status {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Unknown => "????",
        };
        human.push_str(&format!("[{mark}] {:<22} {}\n", check.name, check.detail));
    }
    human.push_str(if report.healthy {
        "No failing checks.\n"
    } else {
        "One or more checks failed.\n"
    });
    Ok(Output {
        exit_code: if report.healthy {
            0
        } else {
            cotra_lifecycle::ErrorKind::State.exit_code()
        },
        human,
        json: json!({"ok": report.healthy, "doctor": report}),
    })
}

pub fn approvals(args: &mut Args) -> Result<Output, LifecycleError> {
    let limit = match args.value("--limit")? {
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|limit| (1..=100).contains(limit))
            .ok_or_else(|| LifecycleError::usage("--limit takes 1-100"))?,
        None => 20,
    };
    args.finish()?;
    let layout = Layout::for_current_user()?;
    let config = Config::load(&layout)?;
    config.check_workspaces_with_policy()?;
    let version = active_version(&layout)?;
    let workspace = config.default_workspace().unwrap_or("default").to_string();
    let request = ipc::request(
        &workspace,
        "approval.history.query",
        "query",
        json!({"limit": limit}),
    );
    let response = ipc::call(
        &layout.version_dir(&version).join("cotrad.exe"),
        &config,
        &request,
        Duration::from_secs(20),
    )?;
    let result = ipc::expect_ok(&response)?.clone();
    let mut human = String::new();
    for entry in result
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        human.push_str(&format!(
            "{} {:<8} {:<7} workspace={} consumed={}\n",
            entry
                .get("decided_at_ms")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            entry.get("decision").and_then(Value::as_str).unwrap_or("?"),
            entry
                .get("approval_class")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            entry
                .get("workspace_id")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            entry
                .get("consumed")
                .map(Value::to_string)
                .unwrap_or_default(),
        ));
    }
    if human.is_empty() {
        human.push_str("No approval history.\n");
    }
    Ok(Output {
        exit_code: 0,
        human,
        json: json!({"ok": true, "approvals": result}),
    })
}

pub fn emergency_revoke(args: &mut Args) -> Result<Output, LifecycleError> {
    args.finish()?;
    let layout = Layout::for_current_user()?;
    let config = Config::load(&layout)?;
    config.check_workspaces_with_policy()?;
    let version = active_version(&layout)?;
    let workspace = config.default_workspace().unwrap_or("default").to_string();
    eprintln!(
        "Confirm with Windows Hello when prompted. Every pending approval will be invalidated."
    );
    let request = ipc::request(&workspace, "trust.revoke_emergency", "revoke", json!({}));
    let response = ipc::call(
        &layout.version_dir(&version).join("cotrad.exe"),
        &config,
        &request,
        Duration::from_secs(180),
    )?;
    let result = ipc::expect_ok(&response)?.clone();
    Ok(Output {
        exit_code: 0,
        human: format!(
            "Emergency revoke recorded (revoke epoch {}). Pending approvals are invalid.\n",
            result
                .get("revoke_epoch")
                .map(Value::to_string)
                .unwrap_or_default()
        ),
        json: json!({"ok": true, "revoke": result}),
    })
}
