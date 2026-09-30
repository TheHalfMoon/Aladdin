//! A local client for one request to the installed `cotrad`, used by
//! `doctor` and by the human-invoked trust and approval commands. It speaks
//! the existing internal JSON-line protocol; every request goes through the
//! policy kernel and approval broker exactly as MCP-originated requests do.

use crate::config::Config;
use crate::LifecycleError;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Environment for Cotra child processes: the closed `cotra-tunnel`
/// allowlist (OS execution variables and safe Cotra variables, with every
/// secret-like name removed) plus the configured workspaces.
pub fn child_environment(config: &Config) -> BTreeMap<String, String> {
    let mut env = cotra_tunnel::sanitized_env(&cotra_tunnel::current_env());
    env.remove("COTRA_WORKSPACE_ROOT");
    env.remove("COTRA_WORKSPACE_ID");
    env.insert("COTRA_WORKSPACES_JSON".into(), config.workspaces_json());
    if let Some(default) = config.default_workspace() {
        env.insert("COTRA_DEFAULT_WORKSPACE".into(), default.into());
    }
    env
}

pub fn request(workspace_id: &str, capability: &str, operation: &str, arguments: Value) -> Value {
    json!({
        "version": 1,
        "request_id": format!("cotra-cli-{}", crate::nonce()),
        "client_session_id": "cotra-cli",
        "workspace_id": workspace_id,
        "capability": capability,
        "operation": operation,
        "target": null,
        "arguments": arguments,
    })
}

/// Sends one request to a fresh `cotrad` and returns the response envelope.
pub fn call(
    cotrad: &Path,
    config: &Config,
    request: &Value,
    timeout: Duration,
) -> Result<Value, LifecycleError> {
    let mut child = Command::new(cotrad)
        .env_clear()
        .envs(child_environment(config))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| LifecycleError::state(format!("start cotrad: {error}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| LifecycleError::internal("cotrad stdin unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| LifecycleError::internal("cotrad stdout unavailable"))?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = sender.send(result);
    });
    let written = writeln!(stdin, "{request}").and_then(|()| stdin.flush());
    let response = match written {
        Ok(()) => receiver.recv_timeout(timeout),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(LifecycleError::state(format!("write to cotrad: {error}")));
        }
    };
    drop(stdin);
    let outcome = match response {
        Ok(Ok(line)) if !line.trim().is_empty() => serde_json::from_str::<Value>(line.trim())
            .map_err(|error| {
                LifecycleError::state(format!("cotrad response is not JSON: {error}"))
            }),
        Ok(Ok(_)) => Err(LifecycleError::state("cotrad exited without a response")),
        Ok(Err(error)) => Err(LifecycleError::state(format!(
            "read cotrad response: {error}"
        ))),
        Err(_) => Err(LifecycleError::state(
            "cotrad did not respond within the time limit",
        )),
    };
    // cotrad exits on stdin EOF; make sure it does not outlive this call.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }
    outcome
}

/// Returns the `result` of a successful response or the typed error.
pub fn expect_ok(response: &Value) -> Result<&Value, LifecycleError> {
    if response.get("ok").and_then(Value::as_bool) == Some(true) {
        return response
            .get("result")
            .ok_or_else(|| LifecycleError::state("cotrad response has no result"));
    }
    let code = response
        .pointer("/error/code")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN");
    let message = response
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("unspecified failure");
    Err(LifecycleError::state(format!("cotrad {code}: {message}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkspaceEntry;

    #[test]
    fn child_environment_carries_workspaces_and_no_secrets() {
        let config = Config {
            workspaces: vec![WorkspaceEntry {
                id: "default".into(),
                root: std::env::temp_dir(),
            }],
            ..Config::default()
        };
        let env = child_environment(&config);
        assert!(env.contains_key("COTRA_WORKSPACES_JSON"));
        assert_eq!(env.get("COTRA_DEFAULT_WORKSPACE").unwrap(), "default");
        for name in env.keys() {
            let upper = name.to_ascii_uppercase();
            for needle in [
                "SECRET",
                "TOKEN",
                "PASSWORD",
                "API_KEY",
                "TUNNEL_KEY",
                "CREDENTIAL",
            ] {
                assert!(!upper.contains(needle), "{name}");
            }
        }
    }

    #[test]
    fn error_envelopes_are_typed() {
        let error = expect_ok(&json!({
            "ok": false,
            "error": {"code": "APPROVAL_DENIED", "message": "denied"}
        }))
        .unwrap_err();
        assert!(error.message.contains("APPROVAL_DENIED"));
        let ok = json!({"ok": true, "result": {"name": "cotrad"}});
        assert_eq!(expect_ok(&ok).unwrap()["name"], "cotrad");
    }
}
