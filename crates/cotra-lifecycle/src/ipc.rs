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

/// OS execution variables in the exact casing the closed `cotra-tunnel`
/// allowlist expects.
const OS_VARIABLES: &[&str] = &[
    "Path",
    "PATHEXT",
    "SystemRoot",
    "WINDIR",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "LOCALAPPDATA",
    "APPDATA",
    "PROGRAMDATA",
    "ProgramFiles",
    "ProgramFiles(x86)",
];

/// The current environment with OS execution variable names normalized to
/// their canonical casing. Windows variable names are case-insensitive, but
/// some launchers (for example MSYS shells) export `SYSTEMROOT`; without
/// normalization the case-sensitive allowlist would drop `SystemRoot` and
/// children could not initialize Winsock.
pub fn host_environment() -> BTreeMap<String, String> {
    normalize_environment(std::env::vars())
}

pub fn normalize_environment(
    vars: impl IntoIterator<Item = (String, String)>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (name, value) in vars {
        let canonical = OS_VARIABLES
            .iter()
            .find(|known| known.eq_ignore_ascii_case(&name))
            .map(|known| (*known).to_string())
            .unwrap_or(name);
        out.entry(canonical).or_insert(value);
    }
    out
}

/// Environment for Cotra child processes: the closed `cotra-tunnel`
/// allowlist (OS execution variables and safe Cotra variables, with every
/// secret-like name removed) plus the configured workspaces.
pub fn child_environment(config: &Config) -> BTreeMap<String, String> {
    let mut env = cotra_tunnel::sanitized_env(&host_environment());
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
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LifecycleError::state(format!("start cotrad: {error}")))?;
    // Keep a bounded tail of cotrad's diagnostics for failure messages.
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| LifecycleError::internal("cotrad stderr unavailable"))?;
    // Drain stderr in fixed-size chunks, keeping only a bounded tail, and hand
    // it back over a channel so a descendant holding the pipe can never block
    // this call.
    let (diagnostics_sender, diagnostics) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut stderr = stderr;
        let mut tail = Vec::<u8>::new();
        let mut chunk = [0u8; 1024];
        while let Ok(read) = std::io::Read::read(&mut stderr, &mut chunk) {
            if read == 0 {
                break;
            }
            tail.extend_from_slice(&chunk[..read]);
            if tail.len() > 2048 {
                tail.drain(..tail.len() - 2048);
            }
        }
        let text = String::from_utf8_lossy(&tail);
        let last = text
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("");
        let _ = diagnostics_sender.send(crate::logs::redact_line(last).chars().take(400).collect());
    });
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
    let exited = loop {
        match child.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                break child.wait().is_ok();
            }
        }
    };
    let tail = if exited {
        diagnostics
            .recv_timeout(Duration::from_millis(500))
            .unwrap_or_default()
    } else {
        String::new()
    };
    outcome.map_err(|error| {
        if tail.is_empty() {
            error
        } else {
            LifecycleError::state(format!("{} (cotrad: {tail})", error.message))
        }
    })
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
    fn os_variable_casing_is_normalized_before_the_allowlist() {
        let env = normalize_environment([
            ("SYSTEMROOT".to_string(), r"C:\Windows".to_string()),
            ("PATH".to_string(), r"C:\bin".to_string()),
            ("temp".to_string(), r"C:\t".to_string()),
            ("OPENAI_API_KEY".to_string(), "leak".to_string()),
        ]);
        let sanitized = cotra_tunnel::sanitized_env(&env);
        assert_eq!(sanitized.get("SystemRoot").unwrap(), r"C:\Windows");
        assert_eq!(sanitized.get("Path").unwrap(), r"C:\bin");
        assert_eq!(sanitized.get("TEMP").unwrap(), r"C:\t");
        assert!(!sanitized.contains_key("OPENAI_API_KEY"));
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
