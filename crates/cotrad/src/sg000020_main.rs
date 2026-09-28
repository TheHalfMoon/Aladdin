use cotra_approval::{ApprovalBroker, ApprovalClass, LocalApprovalBroker};
use cotra_audit::{default_audit_path, AuditLogger};
use cotra_contracts::{FailureCode, RequestEnvelope, ResponseEnvelope, INTERNAL_PROTOCOL_VERSION};
use cotra_policy::{PolicyEngine, Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, Write};

mod git_fetch;
mod git_mutation;
mod git_push;
mod trust;

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
        "cotrad ready: protocol={} audit={} mode=SG-000020_TRUST_REVOKE",
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

    let mut trust_store = trust::TrustStore::load_or_create(trust::default_trust_path());
    match dispatch(
        policy,
        &decision.workspace,
        approval,
        &mut trust_store,
        &request,
    ) {
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
    trust_store: &mut trust::TrustStore,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if let Some(result) = dispatch_trust(workspace, approval, trust_store, request)? {
        return Ok(result);
    }
    if cotra_policy::approval_class_for(&request.capability, &request.operation)
        == ApprovalClass::Strong
    {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "STRONG-class operations require platform-mediated presence and no STRONG execution authority is authorized for this shape; failing closed without SOFT downgrade",
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

fn dispatch_trust(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    trust_store: &mut trust::TrustStore,
    request: &RequestEnvelope,
) -> Result<Option<Value>, ProviderError> {
    match (request.capability.as_str(), request.operation.as_str()) {
        ("workspace.trust.get", "get") => {
            let status = trust_store.get(&request.workspace_id);
            Ok(Some(json!({
                "workspace_id": status.workspace_id,
                "trusted": status.trusted,
                "revision": status.revision,
                "provenance_method": status.provenance_method,
                "updated_at_ms": status.updated_at_ms,
                "policy_revision": status.policy_revision,
            })))
        }
        ("workspace.trust.grant", "grant") => {
            change_trust(workspace, approval, trust_store, request, true).map(Some)
        }
        ("workspace.trust.revoke", "revoke") => {
            change_trust(workspace, approval, trust_store, request, false).map(Some)
        }
        ("trust.revoke_emergency", "revoke") => {
            let digest = revoke_digest(workspace, request);
            let prompt = cotra_approval::ApprovalPrompt::new_strong(
                workspace.id.clone(),
                POLICY_REVISION,
                "emergency revoke all pending approvals",
                request.workspace_id.clone(),
                format!("workspace={} policy={POLICY_REVISION}", workspace.id),
                digest.clone(),
            );
            let epoch = approval
                .emergency_revoke(&prompt)
                .map_err(|error| ProviderError::new(error.code, error.message))?;
            Ok(Some(json!({
                "workspace_id": request.workspace_id,
                "revoke_epoch": epoch,
                "policy_revision": POLICY_REVISION,
            })))
        }
        ("approval.history.query", "query") => {
            let limit = history_limit(request)?;
            let entries = approval.history(limit);
            let items: Vec<Value> = entries
                .iter()
                .map(|entry| {
                    json!({
                        "id": entry.id,
                        "digest": entry.digest,
                        "workspace_id": entry.workspace_id,
                        "policy_revision": entry.policy_revision,
                        "decision": format!("{:?}", entry.decision),
                        "approval_class": entry.approval_class.as_str(),
                        "presence_outcome": entry.presence_outcome.as_str(),
                        "presence_method": entry.presence_method,
                        "epoch": entry.epoch,
                        "is_revoke": entry.is_revoke,
                        "requested_at_ms": entry.requested_at_ms,
                        "decided_at_ms": entry.decided_at_ms,
                        "expires_at_ms": entry.expires_at_ms,
                        "reuse_scope": entry.reuse_scope,
                        "consumed": entry.consumed,
                    })
                })
                .collect();
            Ok(Some(json!({"entries": items})))
        }
        ("trust.history.query", "query") => {
            let limit = history_limit(request)?;
            let entries = trust_store.history(limit);
            let items: Vec<Value> = entries
                .iter()
                .map(|entry| {
                    json!({
                        "workspace_id": entry.workspace_id,
                        "trusted": entry.trusted,
                        "revision": entry.revision,
                        "provenance_method": entry.provenance_method,
                        "updated_at_ms": entry.updated_at_ms,
                        "policy_revision": entry.policy_revision,
                    })
                })
                .collect();
            Ok(Some(json!({"entries": items})))
        }
        _ => Ok(None),
    }
}

fn change_trust(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    trust_store: &mut trust::TrustStore,
    request: &RequestEnvelope,
    requested_trusted: bool,
) -> Result<Value, ProviderError> {
    let before = trust_store.get(&request.workspace_id);
    let digest = trust_digest(workspace, request, &before, requested_trusted);
    let action = if requested_trusted {
        "grant workspace trust"
    } else {
        "revoke workspace trust"
    };
    let prompt = cotra_approval::ApprovalPrompt::new_strong(
        workspace.id.clone(),
        POLICY_REVISION,
        action,
        request.workspace_id.clone(),
        format!(
            "workspace={} requested_trusted={requested_trusted} current_trusted={} current_revision={} policy={POLICY_REVISION}",
            request.workspace_id, before.trusted, before.revision
        ),
        digest.clone(),
    );
    let token = approval
        .request_token(&prompt)
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    approval
        .consume(
            &token,
            &cotra_approval::ConsumeExpectation::strong(
                digest,
                workspace.id.clone(),
                POLICY_REVISION,
            ),
            cotra_approval::now_ms(),
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let after = trust_store.get(&request.workspace_id);
    if after.trusted != before.trusted || after.revision != before.revision {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "workspace trust state changed after approval; trust change fails closed",
        ));
    }
    let method = approval
        .history(1)
        .first()
        .map(|entry| entry.presence_method.clone())
        .unwrap_or_else(|| "unknown".to_owned());
    let status = if requested_trusted {
        trust_store.grant(
            &request.workspace_id,
            POLICY_REVISION,
            &method,
            cotra_approval::now_ms(),
        )?
    } else {
        trust_store.revoke_workspace(
            &request.workspace_id,
            POLICY_REVISION,
            &method,
            cotra_approval::now_ms(),
        )?
    };
    Ok(json!({
        "workspace_id": status.workspace_id,
        "trusted": status.trusted,
        "revision": status.revision,
        "provenance_method": status.provenance_method,
        "updated_at_ms": status.updated_at_ms,
        "policy_revision": status.policy_revision,
    }))
}

fn trust_digest(
    workspace: &Workspace,
    request: &RequestEnvelope,
    before: &trust::TrustStatus,
    requested_trusted: bool,
) -> String {
    let mut hasher = Sha256::new();
    digest_field(&mut hasher, b"COTRA_TRUST_APPROVAL_V1");
    digest_field(&mut hasher, workspace.id.as_bytes());
    digest_field(&mut hasher, POLICY_REVISION.as_bytes());
    digest_field(&mut hasher, request.capability.as_bytes());
    digest_field(&mut hasher, request.operation.as_bytes());
    digest_field(&mut hasher, request.workspace_id.as_bytes());
    digest_field(
        &mut hasher,
        (if requested_trusted { "grant" } else { "revoke" }).as_bytes(),
    );
    digest_field(
        &mut hasher,
        (if before.trusted { "1" } else { "0" }).as_bytes(),
    );
    digest_field(&mut hasher, before.revision.to_string().as_bytes());
    hex_lower(&hasher.finalize())
}

fn revoke_digest(workspace: &Workspace, request: &RequestEnvelope) -> String {
    let mut hasher = Sha256::new();
    digest_field(&mut hasher, b"COTRA_REVOKE_APPROVAL_V1");
    digest_field(&mut hasher, workspace.id.as_bytes());
    digest_field(&mut hasher, POLICY_REVISION.as_bytes());
    digest_field(&mut hasher, request.capability.as_bytes());
    digest_field(&mut hasher, request.operation.as_bytes());
    digest_field(&mut hasher, request.workspace_id.as_bytes());
    hex_lower(&hasher.finalize())
}

fn history_limit(request: &RequestEnvelope) -> Result<usize, ProviderError> {
    match request.arguments.get("limit") {
        None => Ok(50),
        Some(value) => {
            let limit = value.as_u64().ok_or_else(|| {
                ProviderError::new(
                    FailureCode::InvalidRequest,
                    "history query limit must be an unsigned integer",
                )
            })? as usize;
            if !(1..=200).contains(&limit) {
                return Err(ProviderError::new(
                    FailureCode::InvalidRequest,
                    "history query limit must be between 1 and 200",
                ));
            }
            Ok(limit)
        }
    }
}

fn digest_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
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
        let root = std::env::temp_dir().join(format!("cotra-sg20-main-{suffix}"));
        fs::create_dir_all(&root).expect("root");
        root
    }

    fn temp_trust_path() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cotra-sg20-trust-{}-{suffix}.jsonl",
            std::process::id()
        ))
    }

    fn trust_store_at(path: &Path) -> trust::TrustStore {
        trust::TrustStore::load_or_create(path.to_path_buf())
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
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let result = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &request,
        )
        .expect("dispatch");
        assert_eq!(result["policy_id"], "test-origin");
        assert_eq!(result["source_ref"], "refs/heads/main");
        assert_eq!(result["destination_ref"], "refs/heads/main");
        assert_eq!(result["credential_reference"], "anonymous");
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
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
            let trust_path = temp_trust_path();
            let mut trust_store = trust_store_at(&trust_path);
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Approved),
                &mut trust_store,
                &request,
            )
            .expect_err("STRONG dispatch must fail closed");
            assert_eq!(error.code, FailureCode::CapabilityDenied);
            let _ = fs::remove_file(trust_path);
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
        let preview_request = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg19-soft".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "git.push.preview".into(),
            operation: "preview".into(),
            target: Some(".".into()),
            arguments: json!({"policy_id": "test-origin", "source_branch": "main", "dest_branch": "main", "credential_reference": "anonymous"}),
        };
        policy.authorize(&preview_request).expect("SOFT retained");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sg000020_trust_get_is_untrusted_fail_closed_for_unknown() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy =
            PolicyEngine::with_destinations(vec![workspace.clone()], Vec::new(), Vec::new())
                .expect("policy");
        let request = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg20-trust-get".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "workspace.trust.get".into(),
            operation: "get".into(),
            target: Some(".".into()),
            arguments: json!({}),
        };
        policy.authorize(&request).expect("trust get authorized");
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let result = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &request,
        )
        .expect("dispatch")
        .expect("handled");
        assert_eq!(result["trusted"], false);
        assert_eq!(result["revision"], 0);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
    }

    #[test]
    fn sg000020_trust_grant_requires_strong_and_history_labels_class() {
        use cotra_approval::test_support::{broker_with_presence, TestPresenceVerifier};
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy =
            PolicyEngine::with_destinations(vec![workspace.clone()], Vec::new(), Vec::new())
                .expect("policy");
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let grant = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg20-grant".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "workspace.trust.grant".into(),
            operation: "grant".into(),
            target: Some(".".into()),
            arguments: json!({}),
        };
        policy.authorize(&grant).expect("grant authorized");
        let approval_path = temp_trust_path();
        let broker = broker_with_presence(approval_path.clone(), TestPresenceVerifier::verified());
        let result = dispatch(&policy, &workspace, &broker, &mut trust_store, &grant)
            .expect("dispatch")
            .expect("handled");
        assert_eq!(result["trusted"], true);
        assert_eq!(result["revision"], 1);
        let history_request = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg20-history".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "trust.history.query".into(),
            operation: "query".into(),
            target: Some(".".into()),
            arguments: json!({}),
        };
        let history = dispatch(
            &policy,
            &workspace,
            &broker,
            &mut trust_store,
            &history_request,
        )
        .expect("history")
        .expect("handled");
        assert_eq!(history["entries"].as_array().expect("entries").len(), 1);
        let weak = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &grant,
        )
        .expect_err("weak grant must fail");
        assert!(matches!(
            weak.code,
            FailureCode::ApprovalDenied | FailureCode::ApprovalUnavailable
        ));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_file(approval_path);
    }

    #[test]
    fn sg000020_emergency_revoke_invalidates_prior_tokens() {
        use cotra_approval::test_support::{broker_with_presence, TestPresenceVerifier};
        use cotra_approval::{ApprovalBroker, ConsumeExpectation};
        let approval_path = temp_trust_path();
        let broker = broker_with_presence(approval_path.clone(), TestPresenceVerifier::verified());
        let prompt = cotra_approval::ApprovalPrompt::new_strong(
            "default",
            POLICY_REVISION,
            "privileged test action",
            "target",
            "summary",
            "digest-revoke-test",
        );
        let token = broker.request_token(&prompt).expect("token");
        let revoke_request = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg20-revoke".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "trust.revoke_emergency".into(),
            operation: "revoke".into(),
            target: Some(".".into()),
            arguments: json!({}),
        };
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy =
            PolicyEngine::with_destinations(vec![workspace.clone()], Vec::new(), Vec::new())
                .expect("policy");
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        dispatch(
            &policy,
            &workspace,
            &broker,
            &mut trust_store,
            &revoke_request,
        )
        .expect("revoke")
        .expect("handled");
        let expected =
            ConsumeExpectation::strong(prompt.digest.clone(), "default", POLICY_REVISION);
        let error = broker
            .consume(&token, &expected, cotra_approval::now_ms())
            .expect_err("revoked");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_file(approval_path);
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
