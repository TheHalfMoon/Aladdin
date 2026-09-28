use cotra_contracts::{FailureCode, RequestEnvelope};
use cotra_policy::{Workspace, POLICY_REVISION};
use cotra_provider_browser::DnsResolver;
use cotra_provider_fs::ProviderError;
use serde_json::Value;
use std::path::Path;

/// Dispatch the two typed SG-000021 browser shapes. Both are strictly local:
/// profile status touches only Cotra protected state, and destination
/// validation performs parsing plus DNS resolution with post-resolution
/// address policy but no fetch, socket, or tunnel egress. Every other browser
/// shape returns `Ok(None)` so the caller fails closed through the STRONG
/// gate or the legacy capability denial.
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
}
