use cotra_approval::{now_ms, ApprovalBroker, ApprovalPrompt, ConsumeExpectation};
use cotra_contracts::{FailureCode, RequestEnvelope};
use cotra_policy::{Workspace, POLICY_REVISION};
use cotra_provider_browser::DnsResolver;
use cotra_provider_fs::ProviderError;
use serde_json::Value;
use std::path::Path;

/// Dispatch the SG-000021 browser shapes plus SG-000022 page lifecycle and
/// origin-bound navigation. Profile status and destination validation remain
/// strictly local. Page open allocates server-side identity with no network
/// activity. Preview validates with re-resolution but mutates nothing.
/// Navigate requires fresh SOFT approval with digest binding and hop-by-hop
/// redirect validation. Every other browser shape returns `Ok(None)` so the
/// caller fails closed through the STRONG gate or the legacy denial.
pub fn dispatch(
    workspace: &Workspace,
    request: &RequestEnvelope,
    resolver: &impl DnsResolver,
    profile_root: &Path,
) -> Result<Option<Value>, ProviderError> {
    match (request.capability.as_str(), request.operation.as_str()) {
        ("browser.profile", "status") => {
            if request.target.is_some() {
                return Err(ProviderError::new(
                    FailureCode::InvalidRequest,
                    "browser.profile/status does not accept a target field",
                ));
            }
            reject_browser_arguments(request, &[])?;
            let profile = cotra_provider_browser::ensure_isolated_profile(profile_root)
                .map_err(|error| ProviderError::new(error.code, error.message))?;
            Ok(Some(profile.status_json(&workspace.id, POLICY_REVISION)))
        }
        ("browser.destination", "validate") => {
            if request.target.is_some() {
                return Err(ProviderError::new(
                    FailureCode::InvalidRequest,
                    "browser.destination/validate does not accept a target field",
                ));
            }
            reject_browser_arguments(request, &["url", "expected_origin"])?;
            let url = required_string(request, "url")?;
            let validated = match request.arguments.get("expected_origin") {
                None => cotra_provider_browser::validate_destination(url, resolver),
                Some(value) => {
                    let expected = value.as_str().ok_or_else(|| {
                        ProviderError::new(
                            FailureCode::InvalidRequest,
                            "browser.destination/validate expected_origin must be a string",
                        )
                    })?;
                    cotra_provider_browser::validate_redirect(expected, url, resolver)
                }
            }
            .map_err(|error| ProviderError::new(error.code, error.message))?;
            Ok(Some(validated.to_json()))
        }
        _ => Ok(None),
    }
}

/// Dispatch SG-000022 page lifecycle and navigation shapes. Returns
/// `Ok(None)` for non-navigation shapes so the caller falls through to the
/// SG-000021 dispatch above. Page open and preview require no approval;
/// navigate requires a fresh SOFT approval with one-shot consumption.
pub fn dispatch_navigation(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
    resolver: &impl DnsResolver,
    profile_root: &Path,
) -> Result<Option<Value>, ProviderError> {
    match (request.capability.as_str(), request.operation.as_str()) {
        ("browser.page", "open") => {
            if request.target.is_some() {
                return Err(ProviderError::new(
                    FailureCode::InvalidRequest,
                    "browser.page/open does not accept a target field",
                ));
            }
            reject_navigation_arguments(request, &[])?;
            let profile = cotra_provider_browser::ensure_isolated_profile(profile_root)
                .map_err(|error| ProviderError::new(error.code, error.message))?;
            let registry_path = cotra_provider_browser::default_page_registry_path(&profile.root);
            let mut store = cotra_provider_browser::PageStore::load_or_create(registry_path);
            let page = store
                .open_page(&workspace.id, &profile.identity, POLICY_REVISION)
                .map_err(|error| ProviderError::new(error.code, error.message))?;
            Ok(Some(page.to_json()))
        }
        ("browser.navigation", "preview") => {
            if request.target.is_some() {
                return Err(ProviderError::new(
                    FailureCode::InvalidRequest,
                    "browser.navigation/preview does not accept a target field",
                ));
            }
            reject_navigation_arguments(request, &["page_id", "url", "redirect_chain"])?;
            let page_id = required_navigation_string(request, "page_id")?;
            let url = required_navigation_string(request, "url")?;
            let chain = optional_redirect_chain(request)?;
            let profile = cotra_provider_browser::ensure_isolated_profile(profile_root)
                .map_err(|error| ProviderError::new(error.code, error.message))?;
            let registry_path = cotra_provider_browser::default_page_registry_path(&profile.root);
            let store = cotra_provider_browser::PageStore::load_or_create(registry_path);
            let page = store.get(page_id).cloned().ok_or_else(|| {
                ProviderError::new(
                    FailureCode::TargetStale,
                    "browser page handle is unknown; stale page handles fail closed",
                )
            })?;
            check_page_binding(workspace, &profile.identity, &page)?;
            let preview = cotra_provider_browser::preview_navigation(&page, url, &chain, resolver)
                .map_err(|error| ProviderError::new(error.code, error.message))?;
            Ok(Some(preview.to_json()))
        }
        ("browser.navigation", "navigate") => {
            navigate_with_approval(workspace, approval, request, resolver, profile_root).map(Some)
        }
        _ => Ok(None),
    }
}

pub fn system_resolver() -> cotra_provider_browser::SystemResolver {
    cotra_provider_browser::SystemResolver
}

fn reject_browser_arguments(
    request: &RequestEnvelope,
    allowed: &[&str],
) -> Result<(), ProviderError> {
    let arguments = request.arguments.as_object().ok_or_else(|| {
        ProviderError::new(
            FailureCode::InvalidRequest,
            "browser arguments must be an object",
        )
    })?;
    if let Some(key) = arguments
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        if matches!(
            key.as_str(),
            "profile_root"
                | "root"
                | "path"
                | "argv"
                | "executable"
                | "script"
                | "javascript"
                | "command"
                | "personal"
                | "credentials"
                | "cookies"
                | "passwords"
                | "extensions"
                | "devtools"
                | "cdp"
        ) {
            return Err(ProviderError::new(
                FailureCode::CapabilityDenied,
                format!(
                    "browser request must not carry authority-widening field: {key}; caller-selected profiles, browser argv, scripting, and credential material are denied"
                ),
            ));
        }
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            format!("browser request does not accept argument field: {key}"),
        ));
    }
    Ok(())
}

fn required_string<'a>(request: &'a RequestEnvelope, name: &str) -> Result<&'a str, ProviderError> {
    request
        .arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("browser.destination/validate requires arguments.{name}"),
            )
        })
}

fn reject_navigation_arguments(
    request: &RequestEnvelope,
    allowed: &[&str],
) -> Result<(), ProviderError> {
    let arguments = request.arguments.as_object().ok_or_else(|| {
        ProviderError::new(
            FailureCode::InvalidRequest,
            "browser arguments must be an object",
        )
    })?;
    if let Some(key) = arguments
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        if matches!(
            key.as_str(),
            "profile_root"
                | "root"
                | "path"
                | "argv"
                | "executable"
                | "script"
                | "javascript"
                | "command"
                | "personal"
                | "credentials"
                | "cookies"
                | "passwords"
                | "extensions"
                | "devtools"
                | "cdp"
                | "approval"
                | "token"
                | "nonce"
                | "digest"
        ) {
            return Err(ProviderError::new(
                FailureCode::CapabilityDenied,
                format!(
                    "browser request must not carry authority-widening field: {key}; caller-selected profiles, browser argv, scripting, credential material, and caller-supplied approval material are denied"
                ),
            ));
        }
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            format!("browser navigation request does not accept argument field: {key}"),
        ));
    }
    Ok(())
}

fn required_navigation_string<'a>(
    request: &'a RequestEnvelope,
    name: &str,
) -> Result<&'a str, ProviderError> {
    request
        .arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("browser.navigation requires arguments.{name}"),
            )
        })
}

fn optional_redirect_chain(request: &RequestEnvelope) -> Result<Vec<String>, ProviderError> {
    match request.arguments.get("redirect_chain") {
        None => Ok(Vec::new()),
        Some(value) => {
            let hops = value.as_array().ok_or_else(|| {
                ProviderError::new(
                    FailureCode::InvalidRequest,
                    "browser.navigation redirect_chain must be an array of strings",
                )
            })?;
            if hops.len() > cotra_provider_browser::MAX_REDIRECT_HOPS {
                return Err(ProviderError::new(
                    FailureCode::InvalidRequest,
                    format!(
                        "browser.navigation redirect_chain exceeds at most {} hops",
                        cotra_provider_browser::MAX_REDIRECT_HOPS
                    ),
                ));
            }
            hops.iter()
                .map(|hop| {
                    hop.as_str().map(str::to_owned).ok_or_else(|| {
                        ProviderError::new(
                            FailureCode::InvalidRequest,
                            "browser.navigation redirect_chain entries must be strings",
                        )
                    })
                })
                .collect()
        }
    }
}

fn check_page_binding(
    workspace: &Workspace,
    profile_identity: &str,
    page: &cotra_provider_browser::PageRecord,
) -> Result<(), ProviderError> {
    if page.workspace_id != workspace.id {
        return Err(ProviderError::new(
            FailureCode::WorkspaceDenied,
            "browser page belongs to a different workspace; foreign page handles fail closed",
        ));
    }
    if page.profile_identity != profile_identity {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "browser page belongs to a different profile; foreign page handles fail closed",
        ));
    }
    if page.policy_revision != POLICY_REVISION {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "browser page policy revision drifted; stale page handles fail closed",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn navigate_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
    resolver: &impl DnsResolver,
    profile_root: &Path,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "browser.navigation/navigate does not accept a target field",
        ));
    }
    reject_navigation_arguments(
        request,
        &[
            "page_id",
            "url",
            "expected_origin",
            "expected_generation",
            "expected_pinned_address",
            "redirect_chain",
        ],
    )?;
    let page_id = required_navigation_string(request, "page_id")?;
    if !page_id.starts_with(cotra_provider_browser::PAGE_ID_PREFIX) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "browser.navigation/navigate page_id is malformed",
        ));
    }
    let url = required_navigation_string(request, "url")?;
    let expected_origin = required_navigation_string(request, "expected_origin")?;
    let expected_pinned = required_navigation_string(request, "expected_pinned_address")?;
    let expected_generation = request
        .arguments
        .get("expected_generation")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                "browser.navigation/navigate requires arguments.expected_generation",
            )
        })?;
    let chain = optional_redirect_chain(request)?;

    let profile = cotra_provider_browser::ensure_isolated_profile(profile_root)
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let registry_path = cotra_provider_browser::default_page_registry_path(&profile.root);
    let mut store = cotra_provider_browser::PageStore::load_or_create(registry_path);
    let page = store.get(page_id).cloned().ok_or_else(|| {
        ProviderError::new(
            FailureCode::TargetStale,
            "browser page handle is unknown; stale page handles fail closed",
        )
    })?;
    check_page_binding(workspace, &profile.identity, &page)?;

    if page.current_origin != expected_origin {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "browser page origin changed since preview; stale page handles fail closed",
        ));
    }
    if page.generation != expected_generation {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "browser page generation changed since preview; stale page handles fail closed",
        ));
    }

    let preview = cotra_provider_browser::preview_navigation(&page, url, &chain, resolver)
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    if preview.pinned_address.to_string() != expected_pinned {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "browser destination address changed since preview; stale navigation fails closed",
        ));
    }

    let digest = cotra_provider_browser::navigation_approval_digest(
        &workspace.id,
        POLICY_REVISION,
        &profile.identity,
        page_id,
        expected_origin,
        expected_generation,
        &preview.target_origin,
        &preview.pinned_address,
        &chain,
        &preview.final_origin,
    );
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "navigate isolated browser page",
        page_id.to_owned(),
        format!(
            "page={} origin={} -> {} pins={} redirects={}",
            page_id,
            if expected_origin.is_empty() {
                "(new)"
            } else {
                expected_origin
            },
            preview.final_origin,
            preview.pinned_address,
            preview.redirect_count
        ),
        digest.clone(),
    );
    let token = approval
        .request_token(&prompt)
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    approval
        .consume(
            &token,
            &ConsumeExpectation::new(digest, workspace.id.clone(), POLICY_REVISION),
            now_ms(),
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;

    let (next, prior_origin, prior_generation) = store
        .apply_navigation(
            page_id,
            expected_origin,
            expected_generation,
            &preview.final_origin,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let evidence = cotra_provider_browser::NavigationEvidence {
        page_id: next.page_id.clone(),
        workspace_id: next.workspace_id.clone(),
        policy_revision: POLICY_REVISION.to_owned(),
        profile_identity: next.profile_identity.clone(),
        prior_origin,
        prior_generation,
        target_origin: preview.target_origin.clone(),
        pinned_address: preview.pinned_address,
        redirect_count: preview.redirect_count,
        final_origin: preview.final_origin.clone(),
        new_generation: next.generation,
    };
    Ok(evidence.to_json(&token.record_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cotra_contracts::INTERNAL_PROTOCOL_VERSION;
    use cotra_provider_browser::ProviderError as BrowserError;
    use serde_json::json;
    use std::net::IpAddr;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct StaticResolver {
        addresses: Vec<IpAddr>,
    }

    impl DnsResolver for StaticResolver {
        fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>, BrowserError> {
            Ok(self.addresses.clone())
        }
    }

    fn public_resolver() -> StaticResolver {
        StaticResolver {
            addresses: vec!["93.184.216.34".parse().unwrap()],
        }
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cotra-browser-dispatch-{label}-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("root");
        root
    }

    fn workspace(root: &std::path::Path) -> Workspace {
        Workspace {
            id: "default".into(),
            root: std::fs::canonicalize(root).expect("canonical"),
        }
    }

    fn profile_dir(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cotra-browser-profile-{label}-{}-{suffix}",
            std::process::id()
        ))
    }

    fn request(capability: &str, operation: &str, arguments: Value) -> RequestEnvelope {
        RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "browser-dispatch".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: capability.into(),
            operation: operation.into(),
            target: None,
            arguments,
        }
    }

    #[test]
    fn profile_status_returns_isolated_identity_bound_to_workspace() {
        let root = temp_root("status");
        let profile_root = profile_dir("status");
        let result = dispatch(
            &workspace(&root),
            &request("browser.profile", "status", json!({})),
            &public_resolver(),
            &profile_root,
        )
        .expect("dispatch")
        .expect("handled");
        assert_eq!(result["isolated"], true);
        assert_eq!(result["personal_data"], false);
        assert_eq!(result["workspace_id"], "default");
        assert_eq!(result["policy_revision"], POLICY_REVISION);
        assert!(result["profile_identity"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }

    #[test]
    fn destination_validate_accepts_public_and_denies_ssrf() {
        let root = temp_root("validate");
        let profile_root = profile_dir("validate");
        let accepted = dispatch(
            &workspace(&root),
            &request(
                "browser.destination",
                "validate",
                json!({"url": "https://example.com/docs"}),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect("dispatch")
        .expect("handled");
        assert_eq!(accepted["origin"], "https://example.com:443");
        assert_eq!(accepted["pinned_address"], "93.184.216.34");

        let loopback = StaticResolver {
            addresses: vec!["127.0.0.1".parse().unwrap()],
        };
        let error = dispatch(
            &workspace(&root),
            &request(
                "browser.destination",
                "validate",
                json!({"url": "https://example.com/"}),
            ),
            &loopback,
            &profile_root,
        )
        .expect_err("loopback must fail closed");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }

    #[test]
    fn redirect_mismatch_and_widening_fields_fail_closed() {
        let root = temp_root("redirect");
        let profile_root = profile_dir("redirect");
        let same = dispatch(
            &workspace(&root),
            &request(
                "browser.destination",
                "validate",
                json!({"url": "https://example.com/next", "expected_origin": "https://example.com/"}),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect("dispatch")
        .expect("handled");
        assert_eq!(same["origin"], "https://example.com:443");

        let widened = dispatch(
            &workspace(&root),
            &request(
                "browser.destination",
                "validate",
                json!({"url": "https://evil.example.com/", "expected_origin": "https://example.com/"}),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("redirect widening must fail closed");
        assert_eq!(widened.code, FailureCode::CapabilityDenied);

        for field in ["profile_root", "argv", "script", "cookies", "cdp"] {
            let error = dispatch(
                &workspace(&root),
                &request(
                    "browser.destination",
                    "validate",
                    json!({"url": "https://example.com/", field: "x"}),
                ),
                &public_resolver(),
                &profile_root,
            )
            .expect_err("widening field must fail");
            assert_eq!(error.code, FailureCode::CapabilityDenied, "{field}");
        }
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }

    #[test]
    fn unknown_browser_shapes_are_unreachable_at_dispatch() {
        let root = temp_root("unreachable");
        let profile_root = profile_dir("unreachable");
        for (capability, operation) in [
            ("browser.navigate", "navigate"),
            ("browser.dom", "click"),
            ("browser.download", "download"),
        ] {
            let result = dispatch(
                &workspace(&root),
                &request(capability, operation, json!({})),
                &public_resolver(),
                &profile_root,
            )
            .expect("dispatch returns");
            assert!(
                result.is_none(),
                "{capability}/{operation} must be unhandled"
            );
        }
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }

    struct DenyBroker;

    impl ApprovalBroker for DenyBroker {
        fn request_token(
            &self,
            _prompt: &ApprovalPrompt,
        ) -> Result<cotra_approval::ApprovedToken, cotra_approval::ApprovalError> {
            Err(cotra_approval::ApprovalError {
                code: FailureCode::ApprovalDenied,
                message: "denied in test".into(),
            })
        }

        fn consume(
            &self,
            _token: &cotra_approval::ApprovedToken,
            _expected: &ConsumeExpectation,
            _now_ms: u64,
        ) -> Result<(), cotra_approval::ApprovalError> {
            Err(cotra_approval::ApprovalError {
                code: FailureCode::ApprovalDenied,
                message: "no approval".into(),
            })
        }
    }

    fn open_page_for_test(
        workspace_root: &std::path::Path,
        profile_root: &std::path::Path,
    ) -> Value {
        let workspace = workspace(workspace_root);
        dispatch_navigation(
            &workspace,
            &DenyBroker,
            &request("browser.page", "open", json!({})),
            &public_resolver(),
            profile_root,
        )
        .expect("open")
        .expect("handled")
    }

    #[test]
    fn page_open_allocates_server_identity_with_workspace_binding() {
        let root = temp_root("nav-open");
        let profile_root = profile_dir("nav-open");
        let page = open_page_for_test(&root, &profile_root);
        assert!(page["page_id"]
            .as_str()
            .is_some_and(|v| v.starts_with("pg-")));
        assert_eq!(page["workspace_id"], "default");
        assert_eq!(page["generation"], 0);
        let second = open_page_for_test(&root, &profile_root);
        assert_ne!(page["page_id"], second["page_id"]);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }

    #[test]
    fn navigation_preview_validates_without_mutation_or_approval() {
        let root = temp_root("nav-preview");
        let profile_root = profile_dir("nav-preview");
        let page = open_page_for_test(&root, &profile_root);
        let page_id = page["page_id"].as_str().expect("page id").to_owned();
        let workspace = workspace(&root);
        let preview = dispatch_navigation(
            &workspace,
            &DenyBroker,
            &request(
                "browser.navigation",
                "preview",
                json!({"page_id": page_id, "url": "https://example.com/docs"}),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect("preview")
        .expect("handled");
        assert_eq!(preview["target_origin"], "https://example.com:443");
        assert_eq!(preview["final_origin"], "https://example.com:443");
        assert_eq!(preview["pinned_address"], "93.184.216.34");
        let loopback = StaticResolver {
            addresses: vec!["127.0.0.1".parse().unwrap()],
        };
        let error = dispatch_navigation(
            &workspace,
            &DenyBroker,
            &request(
                "browser.navigation",
                "preview",
                json!({"page_id": page_id, "url": "https://example.com/"}),
            ),
            &loopback,
            &profile_root,
        )
        .expect_err("SSRF preview must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
        let forged = dispatch_navigation(
            &workspace,
            &DenyBroker,
            &request(
                "browser.navigation",
                "preview",
                json!({"page_id": "pg-forged", "url": "https://example.com/"}),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("forged page must fail");
        assert_eq!(forged.code, FailureCode::TargetStale);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }

    #[test]
    fn navigation_navigate_denies_without_approval_and_validates_stale_and_widening() {
        use cotra_approval::test_support::FixedApprovalBroker;
        use cotra_approval::ApprovalDecision;
        let root = temp_root("nav-navigate");
        let profile_root = profile_dir("nav-navigate");
        let page = open_page_for_test(&root, &profile_root);
        let page_id = page["page_id"].as_str().expect("page id").to_owned();
        let workspace = workspace(&root);

        let denied = dispatch_navigation(
            &workspace,
            &DenyBroker,
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://example.com/",
                    "expected_origin": "",
                    "expected_generation": 0,
                    "expected_pinned_address": "93.184.216.34",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("denied approval must fail");
        assert_eq!(denied.code, FailureCode::ApprovalDenied);

        let stale_origin = dispatch_navigation(
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://example.com/",
                    "expected_origin": "https://wrong.example:443",
                    "expected_generation": 0,
                    "expected_pinned_address": "93.184.216.34",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("stale origin must fail");
        assert_eq!(stale_origin.code, FailureCode::TargetStale);

        let stale_generation = dispatch_navigation(
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://example.com/",
                    "expected_origin": "",
                    "expected_generation": 9,
                    "expected_pinned_address": "93.184.216.34",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("stale generation must fail");
        assert_eq!(stale_generation.code, FailureCode::TargetStale);

        let drifted_pin = dispatch_navigation(
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://example.com/",
                    "expected_origin": "",
                    "expected_generation": 0,
                    "expected_pinned_address": "1.1.1.1",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("drifted pin must fail");
        assert_eq!(drifted_pin.code, FailureCode::TargetStale);

        let widened = dispatch_navigation(
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://evil.example.com/",
                    "expected_origin": "",
                    "expected_generation": 0,
                    "expected_pinned_address": "93.184.216.34",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("widening must fail");
        assert_eq!(widened.code, FailureCode::CapabilityDenied);

        let download = dispatch_navigation(
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://example.com/tool.exe",
                    "expected_origin": "",
                    "expected_generation": 0,
                    "expected_pinned_address": "93.184.216.34",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("download must fail");
        assert_eq!(download.code, FailureCode::CapabilityDenied);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }

    #[test]
    fn navigation_navigate_succeeds_with_approval_and_bumps_generation() {
        use cotra_approval::test_support::FixedApprovalBroker;
        use cotra_approval::ApprovalDecision;
        let root = temp_root("nav-success");
        let profile_root = profile_dir("nav-success");
        let page = open_page_for_test(&root, &profile_root);
        let page_id = page["page_id"].as_str().expect("page id").to_owned();
        let workspace = workspace(&root);
        let evidence = dispatch_navigation(
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://example.com/docs",
                    "expected_origin": "",
                    "expected_generation": 0,
                    "expected_pinned_address": "93.184.216.34",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect("navigate")
        .expect("handled");
        assert_eq!(evidence["final_origin"], "https://example.com:443");
        assert_eq!(evidence["new_generation"], 1);
        assert_eq!(evidence["cookies"], false);
        let replay = dispatch_navigation(
            &workspace,
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(
                "browser.navigation",
                "navigate",
                json!({
                    "page_id": page_id,
                    "url": "https://example.com/other",
                    "expected_origin": "",
                    "expected_generation": 0,
                    "expected_pinned_address": "93.184.216.34",
                }),
            ),
            &public_resolver(),
            &profile_root,
        )
        .expect_err("replayed generation must fail");
        assert_eq!(replay.code, FailureCode::TargetStale);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(profile_root);
    }
}
