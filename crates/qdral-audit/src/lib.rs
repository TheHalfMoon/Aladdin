use qdral_contracts::RequestEnvelope;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct AuditLogger {
    path: PathBuf,
}

#[derive(Debug, Serialize)]
struct AuditEvent<'a> {
    timestamp_ms: u128,
    request_id: &'a str,
    client_session_id: &'a str,
    workspace_id: &'a str,
    capability: &'a str,
    operation: &'a str,
    target: Option<&'a str>,
    policy_revision: &'a str,
    outcome: &'a str,
}

impl AuditLogger {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn record(
        &self,
        request: &RequestEnvelope,
        policy_revision: &str,
        outcome: &str,
    ) -> io::Result<()> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let event = AuditEvent {
            timestamp_ms,
            request_id: &request.request_id,
            client_session_id: &request.client_session_id,
            workspace_id: &request.workspace_id,
            capability: &request.capability,
            operation: &request.operation,
            target: request.target.as_deref(),
            policy_revision,
            outcome,
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        serde_json::to_writer(&mut file, &event)?;
        file.write_all(b"\n")?;
        file.flush()
    }
}

pub fn default_audit_path() -> PathBuf {
    if let Some(path) = std::env::var_os("QDRAL_AUDIT_PATH") {
        return PathBuf::from(path);
    }

    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join("Qdral")
            .join("audit.jsonl");
    }

    std::env::temp_dir().join("qdral").join("audit.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use qdral_contracts::RequestEnvelope;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_structured_event_without_arguments() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("qdral-audit-{suffix}"))
            .join("audit.jsonl");
        let logger = AuditLogger::new(&path).expect("audit");
        let req = RequestEnvelope {
            version: 1,
            request_id: "r".into(),
            client_session_id: "s".into(),
            workspace_id: "w".into(),
            capability: "fs.search".into(),
            operation: "search".into(),
            target: Some(".".into()),
            arguments: json!({"query":"TOP-SECRET-VALUE"}),
        };
        logger.record(&req, "p1", "SUCCESS").expect("record");
        let text = std::fs::read_to_string(&path).expect("read audit");
        assert!(!text.contains("TOP-SECRET-VALUE"));
        assert!(text.contains("\"capability\":\"fs.search\""));
        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));
    }
}
