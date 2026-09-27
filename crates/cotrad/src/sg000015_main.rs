use cotra_approval::{ApprovalBroker, LocalApprovalBroker};
use cotra_audit::{default_audit_path, AuditLogger};
use cotra_contracts::{FailureCode, RequestEnvelope, ResponseEnvelope, INTERNAL_PROTOCOL_VERSION};
use cotra_policy::{PolicyEngine, Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use serde_json::Value;
use std::io::{self, BufRead, Write};

mod git_mutation;

#[allow(dead_code)]
mod legacy {
    include!("main.rs");

    pub(super) fn load_policy_bridge() -> Result<cotra_policy::PolicyEngine, String> {
        load_policy()
    }

    pub(super) fn dispatch_bridge(
        policy: &cotra_policy::PolicyEngine,
        workspace: &cotra_policy::Workspace,
        approval: &impl cotra_approval::ApprovalBroker,
        request: &cotra_contracts::RequestEnvelope,
    ) -> Result<serde_json::Value, cotra_provider_fs::ProviderError> {
        dispatch(policy, workspace, approval, request)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cotrad fatal: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let policy = legacy::load_policy_bridge()?;
    let audit = AuditLogger::new(default_audit_path())
        .map_err(|error| format!("initialize audit log: {error}"))?;
    let approval = LocalApprovalBroker;

    eprintln!(
        "cotrad ready: protocol={} audit={} mode=SG-000015_APPROVED_LOCAL_GIT_MUTATION",
        INTERNAL_PROTOCOL_VERSION,
        audit.path().display()
    );

    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("cotrad stdin error: {error}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<RequestEnvelope>(&line) {
            Ok(request) => handle_request(&policy, &audit, &approval, request),
            Err(error) => ResponseEnvelope::failure(
                "unknown",
                FailureCode::InvalidRequest,
                format!("invalid request JSON: {error}"),
            ),
        };

        serde_json::to_writer(&mut stdout, &response)
            .map_err(|error| format!("serialize response: {error}"))?;
        stdout
            .write_all(b"\n")
            .map_err(|error| format!("write response: {error}"))?;
        stdout
            .flush()
            .map_err(|error| format!("flush response: {error}"))?;
    }

    Ok(())
}

fn handle_request(
    policy: &PolicyEngine,
    audit: &AuditLogger,
    approval: &impl ApprovalBroker,
    request: RequestEnvelope,
) -> ResponseEnvelope {
    let decision = match policy.authorize(&request) {
        Ok(decision) => decision,
        Err(error) => {
            let _ = audit.record(&request, POLICY_REVISION, "DENIED");
            return ResponseEnvelope::failure(request.request_id, error.code, error.message);
        }
    };

    match dispatch(policy, &decision.workspace, approval, &request) {
        Ok(value) => {
            if let Err(error) = audit.record(&request, decision.policy_revision, "SUCCESS") {
                return ResponseEnvelope::failure(
                    request.request_id,
                    FailureCode::InternalError,
                    format!("audit write failed: {error}"),
                );
            }
            ResponseEnvelope::success(&request, value, decision.policy_revision)
        }
        Err(error) => {
            let state = match error.code {
                FailureCode::ApprovalDenied => "APPROVAL_DENIED",
                FailureCode::ApprovalUnavailable => "APPROVAL_UNAVAILABLE",
                FailureCode::TargetStale => "TARGET_STALE",
                FailureCode::ProcessTimeout => "PROCESS_TIMEOUT",
                FailureCode::ProcessTerminationUnverified => "PROCESS_TERMINATION_UNVERIFIED",
                FailureCode::OutputLimit => "OUTPUT_LIMIT",
                _ => "FAILED",
            };
            let _ = audit.record(&request, decision.policy_revision, state);
            ResponseEnvelope::failure(request.request_id, error.code, error.message)
        }
    }
}

fn dispatch(
    policy: &PolicyEngine,
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if let Some(result) = git_mutation::dispatch(workspace, approval, request)? {
        return Ok(result);
    }
    legacy::dispatch_bridge(policy, workspace, approval, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cotra_approval::test_support::FixedApprovalBroker;
    use cotra_approval::ApprovalDecision;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cotra-sg15-main-{suffix}"));
        fs::create_dir_all(&root).expect("root");
        root
    }

    fn git(cwd: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_device())
            .output()
            .expect("git");
        assert!(output.status.success(), "git failed: {}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    #[test]
    fn sg000015_dispatch_reaches_typed_git_mutation_before_legacy_fallback() {
        let root = temp_root();
        git(&root, &["init"]);
        git(&root, &["config", "user.name", "Cotra Test"]);
        git(&root, &["config", "user.email", "cotra@example.invalid"]);
        fs::write(root.join("a.txt"), "one\n").expect("write");
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-m", "initial"]);
        fs::write(root.join("a.txt"), "two\n").expect("modify");

        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = PolicyEngine::new(vec![workspace.clone()]).expect("policy");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_owned();
        let request = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg15-main".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "git.stage".into(),
            operation: "stage".into(),
            target: Some(".".into()),
            arguments: json!({"expected_head": head, "paths": ["a.txt"]}),
        };
        policy.authorize(&request).expect("policy");
        let result = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request,
        )
        .expect("dispatch");
        assert_eq!(result["paths"][0], "a.txt");
        assert_eq!(git(&root, &["diff", "--cached", "--name-only"]).trim(), "a.txt");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    fn null_device() -> &'static str {
        "NUL"
    }

    #[cfg(not(windows))]
    fn null_device() -> &'static str {
        "/dev/null"
    }
}
