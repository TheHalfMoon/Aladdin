use cotra_approval::{ApprovalBroker, LocalApprovalBroker};
use cotra_audit::{default_audit_path, AuditLogger};
use cotra_contracts::{FailureCode, RequestEnvelope, ResponseEnvelope, INTERNAL_PROTOCOL_VERSION};
use cotra_policy::{PolicyEngine, Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use serde_json::Value;
use std::io::{self, BufRead, Write};

mod git_fetch;
mod git_mutation;
mod git_push;

#[allow(dead_code)]
mod legacy {
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

    include!("main.rs");
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
    let approval = LocalApprovalBroker::new();

    eprintln!(
        "cotrad ready: protocol={} audit={} mode=SG-000019_STRONG_PRESENCE",
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
    if cotra_policy::approval_class_for(&request.capability, &request.operation)
        == cotra_approval::ApprovalClass::Strong
    {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "STRONG-class operations require platform-mediated presence and no STRONG execution authority is authorized in this grain; failing closed without SOFT downgrade",
        ));
    }
    let push_lookup = git_push::PolicyPushLookup { policy };
    let push_resolver = git_push::system_resolver();
    if let Some(result) =
        git_push::dispatch(workspace, approval, request, &push_lookup, &push_resolver)?
    {
        return Ok(result);
    }
    let fetch_lookup = git_fetch::PolicyFetchLookup { policy };
    let fetch_resolver = git_fetch::system_resolver();
    if let Some(result) =
        git_fetch::dispatch(workspace, approval, request, &fetch_lookup, &fetch_resolver)?
    {
        return Ok(result);
    }
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
        let root = std::env::temp_dir().join(format!("cotra-sg19-main-{suffix}"));
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
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    #[test]
    fn sg000019_push_preview_dispatches_with_single_use_soft_approval() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        git(&root, &["config", "user.name", "Cotra Test"]);
        git(&root, &["config", "user.email", "cotra@example.invalid"]);
        fs::write(root.join("a.txt"), "one\n").expect("write");
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-m", "initial"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = PolicyEngine::with_destinations(
            vec![workspace.clone()],
            Vec::new(),
            vec![cotra_policy::PushDestination {
                workspace_id: "default".into(),
                id: "test-origin".into(),
                canonical_url: "https://example.com/repo.git".into(),
                hostname: "example.com".into(),
                port: 443,
                credential_reference: "anonymous".into(),
            }],
        )
        .expect("policy");
        let request = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg17-preview".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "git.push.preview".into(),
            operation: "preview".into(),
            target: Some(".".into()),
            arguments: json!({"policy_id": "test-origin", "source_branch": "main", "dest_branch": "main", "credential_reference": "anonymous"}),
        };
        policy.authorize(&request).expect("policy");
        let result = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request,
        )
        .expect("dispatch");
        assert_eq!(result["policy_id"], "test-origin");
        assert_eq!(result["source_ref"], "refs/heads/main");
        assert_eq!(result["destination_ref"], "refs/heads/main");
        assert_eq!(result["credential_reference"], "anonymous");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sg000019_force_and_unknown_push_shapes_are_denied() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy =
            PolicyEngine::with_destinations(vec![workspace.clone()], Vec::new(), Vec::new())
                .expect("policy");
        for (capability, operation) in [
            ("git.push", "force-push"),
            ("git.push", "delete"),
            ("git.fetch", "push"),
        ] {
            let request = RequestEnvelope {
                version: INTERNAL_PROTOCOL_VERSION,
                request_id: "sg19-deny".into(),
                client_session_id: "session".into(),
                workspace_id: "default".into(),
                capability: capability.into(),
                operation: operation.into(),
                target: Some(".".into()),
                arguments: json!({}),
            };
            let error = policy.authorize(&request).expect_err("push-like denied");
            assert_eq!(error.code, FailureCode::CapabilityDenied);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sg000019_strong_class_dispatch_fails_closed_without_soft_downgrade() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy =
            PolicyEngine::with_destinations(vec![workspace.clone()], Vec::new(), Vec::new())
                .expect("policy");
        for (capability, operation) in [
            ("policy.change", "change"),
            ("workspace.trust_change", "trust_change"),
            ("fs.delete", "delete"),
            ("git.reset", "reset_hard"),
            ("process.kill", "kill_tree"),
        ] {
            assert_eq!(
                cotra_policy::approval_class_for(capability, operation),
                cotra_approval::ApprovalClass::Strong
            );
            let request = RequestEnvelope {
                version: INTERNAL_PROTOCOL_VERSION,
                request_id: "sg19-strong".into(),
                client_session_id: "session".into(),
                workspace_id: "default".into(),
                capability: capability.into(),
                operation: operation.into(),
                target: Some(".".into()),
                arguments: json!({}),
            };
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Approved),
                &request,
            )
            .expect_err("STRONG dispatch must fail closed");
            assert_eq!(error.code, FailureCode::CapabilityDenied);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sg000019_soft_flows_retain_single_use_approval() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        git(&root, &["config", "user.name", "Cotra Test"]);
        git(&root, &["config", "user.email", "cotra@example.invalid"]);
        fs::write(root.join("a.txt"), "one\n").expect("write");
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-m", "initial"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = PolicyEngine::with_destinations(
            vec![workspace.clone()],
            Vec::new(),
            vec![cotra_policy::PushDestination {
                workspace_id: "default".into(),
                id: "test-origin".into(),
                canonical_url: "https://example.com/repo.git".into(),
                hostname: "example.com".into(),
                port: 443,
                credential_reference: "anonymous".into(),
            }],
        )
        .expect("policy");
        assert_eq!(
            cotra_policy::approval_class_for("git.push.preview", "preview"),
            cotra_approval::ApprovalClass::Soft
        );
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
