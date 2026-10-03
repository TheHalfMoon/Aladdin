#[path = "git_fetch_legacy.rs"]
mod legacy;

pub use legacy::{system_resolver, FetchDestinationLookup, PolicyFetchLookup};

use qdral_approval::{now_ms, ApprovalBroker, ApprovalPrompt, ConsumeExpectation};
use qdral_contracts::{FailureCode, RequestEnvelope};
use qdral_policy::{Workspace, POLICY_REVISION};
use qdral_provider_fs::ProviderError;
use qdral_provider_network::{DnsResolver, SystemResolver, SystemTransport, Transport};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn dispatch(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
    lookup: &impl FetchDestinationLookup,
    resolver: &impl qdral_provider_git::fetch::DnsResolver,
) -> Result<Option<Value>, ProviderError> {
    if qdral_provider_network::is_network_fetch_shape(&request.capability, &request.operation) {
        let network_resolver = SystemResolver;
        let transport = SystemTransport;
        return dispatch_network(workspace, approval, request, &network_resolver, &transport);
    }
    legacy::dispatch(workspace, approval, request, lookup, resolver)
}

fn dispatch_network(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
    resolver: &impl DnsResolver,
    transport: &impl Transport,
) -> Result<Option<Value>, ProviderError> {
    let url = request
        .arguments
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                "network/fetch requires string arguments.url",
            )
        })?;

    let prepared = qdral_provider_network::prepare(url, resolver).map_err(map_network_error)?;
    let digest = network_digest(workspace, request, &prepared);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "fetch one bounded approved HTTPS GET destination",
        prepared.target.host.clone(),
        format!(
            "scheme=https host={} port=443 path_digest={} address_set_digest={} max_response={}",
            prepared.target.host,
            prepared.target.path_digest,
            prepared.address_set_digest,
            qdral_provider_network::MAX_BODY_BYTES
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

    let result = qdral_provider_network::execute_prepared(&prepared, resolver, transport)
        .map_err(map_network_error)?;
    Ok(Some(result.to_json(&workspace.id, POLICY_REVISION)))
}

fn network_digest(
    workspace: &Workspace,
    request: &RequestEnvelope,
    prepared: &qdral_provider_network::PreparedFetch,
) -> String {
    let mut hasher = Sha256::new();
    digest_field(&mut hasher, b"QDRAL_NETWORK_FETCH_APPROVAL_V1");
    digest_field(&mut hasher, workspace.id.as_bytes());
    digest_field(&mut hasher, POLICY_REVISION.as_bytes());
    digest_field(&mut hasher, request.capability.as_bytes());
    digest_field(&mut hasher, request.operation.as_bytes());
    digest_field(&mut hasher, b"GET");
    digest_field(&mut hasher, prepared.target.scheme.as_bytes());
    digest_field(&mut hasher, prepared.target.host.as_bytes());
    digest_field(&mut hasher, prepared.target.port.to_string().as_bytes());
    digest_field(&mut hasher, prepared.target.path_digest.as_bytes());
    digest_field(&mut hasher, prepared.address_set_digest.as_bytes());
    digest_field(
        &mut hasher,
        qdral_provider_network::MAX_BODY_BYTES
            .to_string()
            .as_bytes(),
    );
    digest_field(
        &mut hasher,
        qdral_provider_network::MAX_REDIRECTS.to_string().as_bytes(),
    );
    hex_lower(&hasher.finalize())
}

fn map_network_error(error: qdral_provider_network::NetworkError) -> ProviderError {
    ProviderError::new(error.code, error.message)
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
mod network_tests {
    use super::*;
    use qdral_approval::test_support::FixedApprovalBroker;
    use qdral_approval::ApprovalDecision;
    use qdral_contracts::INTERNAL_PROTOCOL_VERSION;
    use qdral_provider_network::{NetworkError, TransportResponse};
    use serde_json::json;
    use std::collections::VecDeque;
    use std::net::IpAddr;
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct Resolver {
        answers: Mutex<VecDeque<Vec<IpAddr>>>,
    }

    impl Resolver {
        fn public(times: usize) -> Self {
            let mut answers = VecDeque::new();
            for _ in 0..times {
                answers.push_back(vec!["8.8.8.8".parse().unwrap()]);
            }
            Self {
                answers: Mutex::new(answers),
            }
        }
    }

    impl DnsResolver for Resolver {
        fn resolve(&self, _host: &str) -> Result<Vec<IpAddr>, NetworkError> {
            self.answers.lock().unwrap().pop_front().ok_or_else(|| {
                NetworkError::new(FailureCode::ProviderUnavailable, "resolver exhausted")
            })
        }
    }

    struct OneResponse(Mutex<Option<TransportResponse>>);

    impl Transport for OneResponse {
        fn get(
            &self,
            _target: &qdral_provider_network::FetchTarget,
            _allowed: &[IpAddr],
        ) -> Result<TransportResponse, NetworkError> {
            self.0.lock().unwrap().take().ok_or_else(|| {
                NetworkError::new(FailureCode::ProviderUnavailable, "transport exhausted")
            })
        }
    }

    fn workspace() -> Workspace {
        Workspace {
            id: "default".into(),
            root: PathBuf::from("."),
        }
    }

    fn request() -> RequestEnvelope {
        RequestEnvelope {
            version: INTERNAL_PROTOCOL_VERSION,
            request_id: "network-fetch".into(),
            client_session_id: "session".into(),
            workspace_id: "default".into(),
            capability: "network".into(),
            operation: "fetch".into(),
            target: None,
            arguments: json!({"url": "https://example.com/data"}),
        }
    }

    #[test]
    fn network_fetch_requires_approval_and_returns_bounded_result() {
        let resolver = Resolver::public(3);
        let transport = OneResponse(Mutex::new(Some(TransportResponse {
            status: 200,
            peer: "8.8.8.8".parse().unwrap(),
            location: None,
            body: b"hello".to_vec(),
        })));
        let result = dispatch_network(
            &workspace(),
            &FixedApprovalBroker(ApprovalDecision::Approved),
            &request(),
            &resolver,
            &transport,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result["status"], 200);
        assert_eq!(result["byte_length"], 5);
        assert_eq!(result["host"], "example.com");
        assert_eq!(result["body"], json!([104, 101, 108, 108, 111]));
    }

    #[test]
    fn denied_approval_prevents_transport() {
        let resolver = Resolver::public(1);
        let transport = OneResponse(Mutex::new(Some(TransportResponse {
            status: 200,
            peer: "8.8.8.8".parse().unwrap(),
            location: None,
            body: b"should-not-run".to_vec(),
        })));
        let error = dispatch_network(
            &workspace(),
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &request(),
            &resolver,
            &transport,
        )
        .unwrap_err();
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        assert!(transport.0.lock().unwrap().is_some());
    }
}
