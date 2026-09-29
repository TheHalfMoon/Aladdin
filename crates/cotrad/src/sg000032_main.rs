use cotra_approval::{ApprovalBroker, ApprovalClass, LocalApprovalBroker};
use cotra_audit::{default_audit_path, AuditLogger};
use cotra_contracts::{FailureCode, RequestEnvelope, ResponseEnvelope, INTERNAL_PROTOCOL_VERSION};
use cotra_policy::{PolicyEngine, Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, Write};
use std::path::Path;

mod browser;
mod git_fetch;
mod git_mutation;
mod git_push;
mod trust;
mod uia;

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
        "cotrad ready: protocol={} audit={} mode=SG-000032_UIA_SCROLL",
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
    let browser_resolver = browser::system_resolver();
    let browser_root = cotra_provider_browser::default_profile_root();
    match dispatch(
        policy,
        &decision.workspace,
        approval,
        &mut trust_store,
        &request,
        &browser_resolver,
        &browser_root,
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
    browser_resolver: &impl cotra_provider_browser::DnsResolver,
    browser_root: &Path,
) -> Result<Value, ProviderError> {
    if let Some(result) = dispatch_trust(workspace, approval, trust_store, request)? {
        return Ok(result);
    }
    if let Some(result) = uia::dispatch_uia(workspace, approval, request)? {
        return Ok(result);
    }
    if let Some(result) =
        browser::dispatch_navigation(workspace, approval, request, browser_resolver, browser_root)?
    {
        return Ok(result);
    }
    if let Some(result) = browser::dispatch_observation(workspace, request, browser_root)? {
        return Ok(result);
    }
    if let Some(result) = browser::dispatch_actuation(workspace, approval, request, browser_root)? {
        return Ok(result);
    }
    if let Some(result) =
        browser::dispatch_download(workspace, approval, request, browser_resolver, browser_root)?
    {
        return Ok(result);
    }
    let trust = trust_store.get(&workspace.id);
    if let Some(result) = browser::dispatch_upload(
        workspace,
        approval,
        request,
        trust.trusted,
        trust.revision,
        browser_root,
    )? {
        return Ok(result);
    }
    if let Some(result) = browser::dispatch(workspace, request, browser_resolver, browser_root)? {
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
    use cotra_provider_browser::ProviderError as BrowserError;
    use serde_json::json;
    use std::fs;
    use std::net::IpAddr;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct StaticBrowserResolver {
        addresses: Vec<IpAddr>,
    }

    impl cotra_provider_browser::DnsResolver for StaticBrowserResolver {
        fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>, BrowserError> {
            Ok(self.addresses.clone())
        }
    }

    fn public_browser_resolver() -> StaticBrowserResolver {
        StaticBrowserResolver {
            addresses: vec!["93.184.216.34".parse().unwrap()],
        }
    }

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cotra-sg21-main-{suffix}"));
        fs::create_dir_all(&root).expect("root");
        root
    }

    fn temp_trust_path() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cotra-sg21-trust-{}-{suffix}.jsonl",
            std::process::id()
        ))
    }

    fn temp_browser_root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cotra-sg21-browser-{label}-{}-{suffix}",
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

    fn policy_for(workspace: &Workspace) -> PolicyEngine {
        PolicyEngine::with_destinations(
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
        .expect("policy")
    }

    fn browser_request(capability: &str, operation: &str, arguments: Value) -> RequestEnvelope {
        RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg21-browser".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: capability.into(),
            operation: operation.into(),
            target: None,
            arguments,
        }
    }

    #[test]
    fn sg000021_profile_status_dispatches_isolated_identity() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let request = browser_request("browser.profile", "status", json!({}));
        policy.authorize(&request).expect("policy");
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("status");
        let result = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("dispatch");
        assert_eq!(result["isolated"], true);
        assert_eq!(result["personal_data"], false);
        assert_eq!(result["workspace_id"], "default");
        assert_eq!(result["policy_revision"], POLICY_REVISION);
        let marker = browser_root.join("COTRA_AUTOMATION_PROFILE");
        assert!(marker.is_file(), "profile marker must exist");
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000021_destination_validate_accepts_public_and_denies_ssrf_and_widening() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("validate");

        let accepted = browser_request(
            "browser.destination",
            "validate",
            json!({"url": "https://example.com/docs"}),
        );
        policy.authorize(&accepted).expect("policy");
        let result = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &accepted,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("dispatch");
        assert_eq!(result["origin"], "https://example.com:443");

        let loopback = StaticBrowserResolver {
            addresses: vec!["127.0.0.1".parse().unwrap()],
        };
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &accepted,
            &loopback,
            &browser_root,
        )
        .expect_err("loopback must fail closed");
        assert_eq!(error.code, FailureCode::CapabilityDenied);

        let widened = browser_request(
            "browser.destination",
            "validate",
            json!({"url": "https://evil.example.com/", "expected_origin": "https://example.com/"}),
        );
        policy.authorize(&widened).expect("shape authorized");
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &widened,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("redirect widening must fail closed");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000021_actuation_dispatch_fails_closed_without_soft_downgrade() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("actuation");
        for (capability, operation) in [
            ("browser.navigate", "navigate"),
            // NOTE (SG-000024 successor): structured browser.dom/click and
            // browser.dom/fill are authorized successor shapes covered by
            // the SG-000024 actuation tests; they no longer fail closed here.
            // NOTE (SG-000025 successor): scoped browser.download/preview and
            // browser.download/download are authorized successor shapes
            // covered by the SG-000025 download tests; they no longer fail
            // closed here. Downloaded-file execution, opening, extraction,
            // and upload remain denied in every successor grain.
            ("browser.download", "execute"),
            ("browser.download", "open"),
            ("browser.download", "extract"),
            ("browser.file", "execute"),
            ("browser.archive", "extract"),
            ("browser.upload", "upload"),
            ("browser.profile", "use_personal"),
            ("browser.script", "evaluate"),
            ("policy.change", "change"),
            ("fs.delete", "delete"),
        ] {
            assert_eq!(
                cotra_policy::approval_class_for(capability, operation),
                cotra_approval::ApprovalClass::Strong
            );
            let request = browser_request(capability, operation, json!({}));
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Approved),
                &mut trust_store,
                &request,
                &public_browser_resolver(),
                &browser_root,
            )
            .expect_err("actuation dispatch must fail closed");
            assert_eq!(
                error.code,
                FailureCode::CapabilityDenied,
                "{capability}/{operation}"
            );
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000021_personal_profile_root_never_reaches_disk() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let request = browser_request(
            "browser.profile",
            "status",
            json!({"profile_root": "C:\\Users\\Owner\\AppData\\Local\\Google\\Chrome\\User Data"}),
        );
        let error = policy
            .authorize(&request)
            .expect_err("personal profile root must be rejected");
        assert_eq!(error.code, FailureCode::InvalidRequest);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sg000021_trust_grant_still_requires_strong_presence() {
        use cotra_approval::test_support::{broker_with_presence, TestPresenceVerifier};
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let grant = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg21-grant".into(),
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
        let browser_root = temp_browser_root("trust");
        let result = dispatch(
            &policy,
            &workspace,
            &broker,
            &mut trust_store,
            &grant,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("dispatch");
        assert_eq!(result["trusted"], true);
        assert_eq!(result["revision"], 1);
        let weak = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &grant,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("weak grant must fail");
        assert!(matches!(
            weak.code,
            FailureCode::ApprovalDenied | FailureCode::ApprovalUnavailable
        ));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_file(approval_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000021_soft_push_preview_retained() {
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
        let policy = policy_for(&workspace);
        assert_eq!(
            cotra_policy::approval_class_for("git.push.preview", "preview"),
            cotra_approval::ApprovalClass::Soft
        );
        let preview_request = RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "sg21-soft".into(),
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
    fn sg000022_page_open_preview_navigate_round_trip_with_soft_approval() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg22-roundtrip");
        let open_request = browser_request("browser.page", "open", json!({}));
        policy
            .authorize(&open_request)
            .expect("page open authorized");
        let opened = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &open_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("open dispatches");
        let page_id = opened["page_id"].as_str().expect("page id").to_owned();
        assert!(page_id.starts_with("pg-"));

        let preview_request = browser_request(
            "browser.navigation",
            "preview",
            json!({"page_id": page_id, "url": "https://example.com/docs"}),
        );
        policy
            .authorize(&preview_request)
            .expect("preview authorized");
        let preview = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &preview_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("preview dispatches");
        assert_eq!(preview["target_origin"], "https://example.com:443");

        let navigate_request = browser_request(
            "browser.navigation",
            "navigate",
            json!({
                "page_id": page_id,
                "url": "https://example.com/docs",
                "expected_origin": "",
                "expected_generation": 0,
                "expected_pinned_address": "93.184.216.34",
            }),
        );
        policy
            .authorize(&navigate_request)
            .expect("navigate shape authorized");
        let evidence = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &navigate_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("navigate dispatches");
        assert_eq!(evidence["final_origin"], "https://example.com:443");
        assert_eq!(evidence["new_generation"], 1);
        assert_eq!(evidence["cookies"], false);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000022_navigation_denies_widening_ssrf_download_and_stale_handles() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg22-denies");
        let opened = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &browser_request("browser.page", "open", json!({})),
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("open")
        .clone();
        let page_id = opened["page_id"].as_str().expect("page id").to_owned();
        let navigate = |url: &str,
                        expected_origin: &str,
                        expected_generation: u64,
                        expected_pin: &str|
         -> RequestEnvelope {
            browser_request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": url,
                    "expected_origin": expected_origin,
                    "expected_generation": expected_generation,
                    "expected_pinned_address": expected_pin,
                }),
            )
        };
        for (url, code) in [
            (
                "https://example.com/tool.exe",
                FailureCode::CapabilityDenied,
            ),
            ("https://example.com/", FailureCode::TargetStale),
        ] {
            let (origin, generation, pin) = if url == "https://example.com/" {
                ("https://wrong.example:443", 0, "93.184.216.34")
            } else {
                ("", 0, "93.184.216.34")
            };
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Approved),
                &mut trust_store,
                &navigate(url, origin, generation, pin),
                &public_browser_resolver(),
                &browser_root,
            )
            .expect_err("navigation must fail closed");
            assert_eq!(error.code, code, "{url}");
        }
        let widened = browser_request(
            "browser.navigation",
            "navigate",
            json!({
                "page_id": page_id,
                "url": "https://example.com/",
                "expected_origin": "",
                "expected_generation": 0,
                "expected_pinned_address": "93.184.216.34",
                "redirect_chain": ["https://evil.example.com/"],
            }),
        );
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &widened,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("redirect widening must fail closed");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
        let loopback = StaticBrowserResolver {
            addresses: vec!["127.0.0.1".parse().unwrap()],
        };
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &navigate("https://example.com/", "", 0, "93.184.216.34"),
            &loopback,
            &browser_root,
        )
        .expect_err("loopback must fail closed");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
        let forged = browser_request(
            "browser.navigation",
            "navigate",
            json!({
                "page_id": "pg-forged-handle",
                "url": "https://example.com/",
                "expected_origin": "",
                "expected_generation": 0,
                "expected_pinned_address": "93.184.216.34",
            }),
        );
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &forged,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("forged handle must fail");
        assert_eq!(error.code, FailureCode::TargetStale);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000022_dom_download_upload_scripting_debug_and_personal_remain_denied() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg22-denied");
        for (capability, operation) in [
            // NOTE (SG-000024 successor): structured browser.dom/click and
            // browser.dom/fill are lawfully authorized by the SG-000024
            // successor grain and are therefore no longer in this denied
            // set; they are covered by the SG-000024 actuation tests below.
            ("browser.dom", "snapshot"),
            ("browser.snapshot", "capture"),
            // NOTE (SG-000025 successor): scoped browser.download/preview and
            // browser.download/download are authorized successor shapes
            // covered by the SG-000025 download tests; they no longer fail
            // closed here. Downloaded-file execution, opening, extraction,
            // and upload remain denied in every successor grain.
            ("browser.download", "execute"),
            ("browser.download", "open"),
            ("browser.download", "extract"),
            ("browser.file", "execute"),
            ("browser.archive", "extract"),
            ("browser.upload", "upload"),
            ("browser.script", "evaluate"),
            ("browser.cdp", "command"),
            ("browser.profile", "use_personal"),
            ("browser.page", "close"),
            ("browser.navigation", "back"),
        ] {
            assert_eq!(
                cotra_policy::approval_class_for(capability, operation),
                cotra_approval::ApprovalClass::Strong
            );
            let request = browser_request(capability, operation, json!({}));
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Approved),
                &mut trust_store,
                &request,
                &public_browser_resolver(),
                &browser_root,
            )
            .expect_err("denied shape must fail closed");
            assert_eq!(
                error.code,
                FailureCode::CapabilityDenied,
                "{capability}/{operation}"
            );
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000023_snapshot_observe_round_trip_with_typed_node_identity() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg23-roundtrip");
        let opened = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &browser_request("browser.page", "open", json!({})),
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("open")
        .clone();
        let page_id = opened["page_id"].as_str().expect("page id").to_owned();

        let denied_before_navigate = browser_request(
            "browser.snapshot",
            "observe",
            json!({
                "page_id": page_id,
                "expected_origin": "",
                "expected_generation": 0,
            }),
        );
        policy
            .authorize(&denied_before_navigate)
            .expect("snapshot shape authorized");
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &denied_before_navigate,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("open pages have no document and must fail closed");
        assert_eq!(error.code, FailureCode::TargetStale);

        let navigate_request = browser_request(
            "browser.navigation",
            "navigate",
            json!({
                "page_id": page_id,
                "url": "https://example.com/docs",
                "expected_origin": "",
                "expected_generation": 0,
                "expected_pinned_address": "93.184.216.34",
            }),
        );
        policy
            .authorize(&navigate_request)
            .expect("navigate authorized");
        let evidence = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &navigate_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("navigate dispatches");
        assert_eq!(evidence["final_origin"], "https://example.com:443");

        let snapshot_request = browser_request(
            "browser.snapshot",
            "observe",
            json!({
                "page_id": page_id,
                "expected_origin": "https://example.com:443",
                "expected_generation": 1,
            }),
        );
        policy
            .authorize(&snapshot_request)
            .expect("snapshot authorized");
        let snapshot = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &snapshot_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("snapshot dispatches");
        assert_eq!(snapshot["page_id"], page_id.as_str());
        assert_eq!(snapshot["origin"], "https://example.com:443");
        assert_eq!(snapshot["page_generation"], 1);
        assert_eq!(snapshot["cookies"], false);
        assert_eq!(snapshot["credentials"], false);
        let nodes = snapshot["nodes"].as_array().expect("nodes");
        assert!(!nodes.is_empty());
        for node in nodes {
            assert!(node["node_id"]
                .as_str()
                .is_some_and(|value| value.starts_with("nd-")));
            assert_eq!(node["page_id"], page_id.as_str());
            assert_eq!(node["page_generation"], 1);
            assert_eq!(node["origin"], "https://example.com:443");
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000023_snapshot_denies_stale_foreign_oversized_and_actuation() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg23-denies");
        let opened = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &browser_request("browser.page", "open", json!({})),
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("open")
        .clone();
        let page_id = opened["page_id"].as_str().expect("page id").to_owned();
        let navigate_request = browser_request(
            "browser.navigation",
            "navigate",
            json!({
                "page_id": page_id,
                "url": "https://example.com/docs",
                "expected_origin": "",
                "expected_generation": 0,
                "expected_pinned_address": "93.184.216.34",
            }),
        );
        dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &navigate_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("navigate");

        let observe = |arguments: Value| -> RequestEnvelope {
            browser_request("browser.snapshot", "observe", arguments)
        };
        for (arguments, code) in [
            (
                json!({
                    "page_id": page_id,
                    "expected_origin": "https://wrong.example:443",
                    "expected_generation": 1,
                }),
                FailureCode::TargetStale,
            ),
            (
                json!({
                    "page_id": page_id,
                    "expected_origin": "https://example.com:443",
                    "expected_generation": 9,
                }),
                FailureCode::TargetStale,
            ),
            (
                json!({
                    "page_id": "pg-forged-handle",
                    "expected_origin": "https://example.com:443",
                    "expected_generation": 1,
                }),
                FailureCode::TargetStale,
            ),
        ] {
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Denied),
                &mut trust_store,
                &observe(arguments),
                &public_browser_resolver(),
                &browser_root,
            )
            .expect_err("stale snapshot must fail closed");
            assert_eq!(error.code, code);
        }

        let oversized = observe(json!({
            "page_id": page_id,
            "expected_origin": "https://example.com:443",
            "expected_generation": 1,
            "max_nodes": 201,
        }));
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &oversized,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("oversized snapshot must fail closed");
        assert!(matches!(
            error.code,
            FailureCode::InvalidRequest | FailureCode::OutputLimit
        ));

        for (capability, operation) in [
            // NOTE (SG-000024 successor): browser.dom/click and
            // browser.dom/fill are authorized successor shapes covered by
            // the SG-000024 actuation tests below.
            ("browser.snapshot", "capture"),
            ("browser.script", "evaluate"),
            ("browser.cdp", "command"),
        ] {
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Approved),
                &mut trust_store,
                &browser_request(capability, operation, json!({})),
                &public_browser_resolver(),
                &browser_root,
            )
            .expect_err("actuation must fail closed");
            assert_eq!(
                error.code,
                FailureCode::CapabilityDenied,
                "{capability}/{operation}"
            );
        }
        let widened = observe(json!({
            "page_id": page_id,
            "expected_origin": "https://example.com:443",
            "expected_generation": 1,
            "script": "alert(1)",
        }));
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &widened,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("script widening must fail closed");
        assert!(matches!(
            error.code,
            FailureCode::InvalidRequest | FailureCode::CapabilityDenied
        ));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    fn open_navigate_snapshot_for_actuation(
        policy: &PolicyEngine,
        workspace: &Workspace,
        trust_store: &mut trust::TrustStore,
        browser_root: &Path,
    ) -> (String, Value) {
        let opened = dispatch(
            policy,
            workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            trust_store,
            &browser_request("browser.page", "open", json!({})),
            &public_browser_resolver(),
            browser_root,
        )
        .expect("open")
        .clone();
        let page_id = opened["page_id"].as_str().expect("page id").to_owned();
        let navigate_request = browser_request(
            "browser.navigation",
            "navigate",
            json!({
                "page_id": page_id,
                "url": "https://example.com/docs",
                "expected_origin": "",
                "expected_generation": 0,
                "expected_pinned_address": "93.184.216.34",
            }),
        );
        policy
            .authorize(&navigate_request)
            .expect("navigate authorized");
        dispatch(
            policy,
            workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            trust_store,
            &navigate_request,
            &public_browser_resolver(),
            browser_root,
        )
        .expect("navigate");
        let snapshot_request = browser_request(
            "browser.snapshot",
            "observe",
            json!({
                "page_id": page_id,
                "expected_origin": "https://example.com:443",
                "expected_generation": 1,
            }),
        );
        policy
            .authorize(&snapshot_request)
            .expect("snapshot authorized");
        let snapshot = dispatch(
            policy,
            workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            trust_store,
            &snapshot_request,
            &public_browser_resolver(),
            browser_root,
        )
        .expect("snapshot");
        (page_id, snapshot)
    }

    #[test]
    fn sg000024_click_round_trip_with_soft_approval_and_generation_bump() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg24-click");
        let (page_id, snapshot) = open_navigate_snapshot_for_actuation(
            &policy,
            &workspace,
            &mut trust_store,
            &browser_root,
        );
        let link = snapshot["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|node| node["role"] == "link")
            .expect("link node")
            .clone();
        let node_id = link["node_id"].as_str().expect("node id").to_owned();
        let click_request = browser_request(
            "browser.dom",
            "click",
            json!({
                "page_id": page_id,
                "expected_origin": "https://example.com:443",
                "expected_generation": 1,
                "expected_document_generation": 1,
                "node_id": node_id,
                "expected_role": "link",
                "expected_state": "enabled",
            }),
        );
        policy.authorize(&click_request).expect("click authorized");
        let evidence = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &click_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("click dispatches");
        assert!(evidence["actuation_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("ac-")));
        assert_eq!(evidence["node_id"], node_id.as_str());
        assert_eq!(evidence["action"], "click");
        assert_eq!(evidence["prior_generation"], 1);
        assert_eq!(evidence["new_generation"], 2);
        assert_eq!(evidence["cookies"], false);
        assert!(evidence.get("value").is_none());
        let replay = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &click_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("replayed generation must fail");
        assert_eq!(replay.code, FailureCode::TargetStale);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000024_fill_round_trip_binds_value_digest_without_value_leakage() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg24-fill");
        let (page_id, snapshot) = open_navigate_snapshot_for_actuation(
            &policy,
            &workspace,
            &mut trust_store,
            &browser_root,
        );
        let textbox = snapshot["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|node| node["role"] == "textbox")
            .expect("textbox node")
            .clone();
        let node_id = textbox["node_id"].as_str().expect("node id").to_owned();
        let fill_request = browser_request(
            "browser.dom",
            "fill",
            json!({
                "page_id": page_id,
                "expected_origin": "https://example.com:443",
                "expected_generation": 1,
                "expected_document_generation": 1,
                "node_id": node_id,
                "expected_role": "textbox",
                "expected_state": "enabled",
                "value": "hello cotra",
            }),
        );
        policy.authorize(&fill_request).expect("fill authorized");
        let evidence = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &fill_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect("fill dispatches");
        assert_eq!(evidence["action"], "fill");
        assert_eq!(evidence["new_generation"], 2);
        assert!(!evidence["value_digest"].as_str().unwrap_or("").is_empty());
        assert!(evidence.get("value").is_none());
        assert!(evidence.get("password").is_none());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
    }

    #[test]
    fn sg000024_actuation_denies_stale_mismatch_oversized_and_extended_verbs() {
        let root = temp_root();
        git(&root, &["init", "-b", "main"]);
        let workspace = Workspace {
            id: "default".into(),
            root: fs::canonicalize(&root).expect("canonical"),
        };
        let policy = policy_for(&workspace);
        let trust_path = temp_trust_path();
        let mut trust_store = trust_store_at(&trust_path);
        let browser_root = temp_browser_root("sg24-denies");
        let (page_id, snapshot) = open_navigate_snapshot_for_actuation(
            &policy,
            &workspace,
            &mut trust_store,
            &browser_root,
        );
        let link = snapshot["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|node| node["role"] == "link")
            .expect("link node")
            .clone();
        let node_id = link["node_id"].as_str().expect("node id").to_owned();
        let click = |arguments: Value| -> RequestEnvelope {
            browser_request("browser.dom", "click", arguments)
        };
        let base = || {
            json!({
                "page_id": page_id,
                "expected_origin": "https://example.com:443",
                "expected_generation": 1,
                "expected_document_generation": 1,
                "node_id": node_id,
                "expected_role": "link",
                "expected_state": "enabled",
            })
        };
        let mut stale_generation = base();
        stale_generation["expected_generation"] = json!(9);
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &click(stale_generation),
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("stale generation must fail");
        assert_eq!(error.code, FailureCode::TargetStale);

        let mut wrong_role = base();
        wrong_role["expected_role"] = json!("button");
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &click(wrong_role),
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("role mismatch must fail");
        assert_eq!(error.code, FailureCode::TargetStale);

        let fill_request = browser_request(
            "browser.dom",
            "fill",
            json!({
                "page_id": page_id,
                "expected_origin": "https://example.com:443",
                "expected_generation": 1,
                "expected_document_generation": 1,
                "node_id": node_id,
                "expected_role": "link",
                "expected_state": "enabled",
                "value": "x",
            }),
        );
        let error = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &mut trust_store,
            &fill_request,
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("fill on link must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);

        let denied_approval = dispatch(
            &policy,
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &mut trust_store,
            &click(base()),
            &public_browser_resolver(),
            &browser_root,
        )
        .expect_err("denied approval must fail");
        assert_eq!(denied_approval.code, FailureCode::ApprovalDenied);

        for (capability, operation) in [
            ("browser.dom", "select"),
            ("browser.dom", "type"),
            ("browser.snapshot", "capture"),
            ("browser.script", "evaluate"),
            ("browser.cdp", "command"),
            // NOTE (SG-000025 successor): scoped browser.download/preview and
            // browser.download/download are authorized successor shapes
            // covered by the SG-000025 download tests; they no longer fail
            // closed here. Downloaded-file execution, opening, extraction,
            // and upload remain denied in every successor grain.
            ("browser.download", "execute"),
            ("browser.download", "open"),
            ("browser.download", "extract"),
            ("browser.file", "execute"),
            ("browser.archive", "extract"),
            ("browser.shell", "run"),
            ("browser.upload", "upload"),
        ] {
            let error = dispatch(
                &policy,
                &workspace,
                &FixedApprovalBroker(ApprovalDecision::Approved),
                &mut trust_store,
                &browser_request(capability, operation, json!({})),
                &public_browser_resolver(),
                &browser_root,
            )
            .expect_err("extended verb must fail closed");
            assert_eq!(
                error.code,
                FailureCode::CapabilityDenied,
                "{capability}/{operation}"
            );
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(trust_path);
        let _ = fs::remove_dir_all(browser_root);
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
