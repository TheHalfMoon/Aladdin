use qdral_contracts::FailureCode;
use qdral_provider_network::{
    execute_prepared, is_public_address, normalize_public_set, parse_target, DnsResolver,
    NetworkError, PreparedFetch, Transport, TransportResponse, MAX_BODY_BYTES, MAX_REDIRECTS,
    MAX_URL_CHARS,
};
use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::Mutex;

struct SequenceResolver {
    answers: Mutex<VecDeque<Vec<IpAddr>>>,
}

impl SequenceResolver {
    fn new(answers: Vec<Vec<IpAddr>>) -> Self {
        Self {
            answers: Mutex::new(answers.into()),
        }
    }
}

impl DnsResolver for SequenceResolver {
    fn resolve(&self, _host: &str) -> Result<Vec<IpAddr>, NetworkError> {
        self.answers.lock().unwrap().pop_front().ok_or_else(|| {
            NetworkError::new(FailureCode::ProviderUnavailable, "resolver exhausted")
        })
    }
}

struct SequenceTransport {
    responses: Mutex<VecDeque<Result<TransportResponse, NetworkError>>>,
}

impl SequenceTransport {
    fn new(responses: Vec<Result<TransportResponse, NetworkError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl Transport for SequenceTransport {
    fn get(
        &self,
        _target: &qdral_provider_network::FetchTarget,
        _allowed: &[IpAddr],
    ) -> Result<TransportResponse, NetworkError> {
        self.responses.lock().unwrap().pop_front().ok_or_else(|| {
            NetworkError::new(FailureCode::ProviderUnavailable, "transport exhausted")
        })?
    }
}

fn public() -> IpAddr {
    "8.8.8.8".parse().unwrap()
}

fn prepared(url: &str) -> PreparedFetch {
    PreparedFetch {
        target: parse_target(url).unwrap(),
        addresses: vec![public()],
        address_set_digest: qdral_provider_network::address_set_digest(&[public()]),
    }
}

fn response(
    status: u16,
    location: Option<&str>,
    body: &[u8],
) -> Result<TransportResponse, NetworkError> {
    Ok(TransportResponse {
        status,
        peer: public(),
        location: location.map(str::to_owned),
        body: body.to_vec(),
    })
}

fn public_answers(count: usize) -> Vec<Vec<IpAddr>> {
    (0..count).map(|_| vec![public()]).collect()
}

#[test]
fn url_bound_is_exact_and_widened_authorities_fail_closed() {
    let prefix = "https://example.com/";
    let exact = format!(
        "{prefix}{}",
        "a".repeat(MAX_URL_CHARS - prefix.chars().count())
    );
    assert_eq!(exact.chars().count(), MAX_URL_CHARS);
    assert!(parse_target(&exact).is_ok());

    let oversized = format!("{exact}a");
    let error = parse_target(&oversized).unwrap_err();
    assert_eq!(error.code, FailureCode::InvalidRequest);

    for denied in [
        "HTTPS://example.com/",
        "https://example.com:80/",
        "https://example.com:444/",
        "https://user:pass@example.com/",
        "https://foo.localhost/",
        "https://127.0.0.1/",
        "https://0177.0.0.1/",
        "https://2130706433/",
        "https://0x7f000001/",
        "https://169.254.169.254/latest/meta-data/",
        "https://example.com/#fragment",
        "https://example.com\\evil",
    ] {
        assert!(parse_target(denied).is_err(), "must reject {denied}");
    }
}

#[test]
fn public_only_catalog_denies_special_ipv4_ipv6_and_mapped_private() {
    for denied in [
        "0.0.0.0",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.168.0.1",
        "192.0.2.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "fc00::1",
        "fe80::1",
        "fec0::1",
        "2001:db8::1",
        "2002::1",
        "3fff::1",
        "4000::1",
        "ff02::1",
    ] {
        let address: IpAddr = denied.parse().unwrap();
        assert!(!is_public_address(&address), "must deny {denied}");
    }
    assert!(is_public_address(&"1.1.1.1".parse().unwrap()));
    assert!(is_public_address(&"2001:4860:4860::8888".parse().unwrap()));
}

#[test]
fn mixed_public_private_resolution_is_denied_before_transport() {
    let error =
        normalize_public_set(vec![public(), "169.254.169.254".parse().unwrap()]).unwrap_err();
    assert_eq!(error.code, FailureCode::CapabilityDenied);
}

#[test]
fn post_approval_dns_drift_is_stale() {
    let resolver = SequenceResolver::new(vec![vec!["1.1.1.1".parse().unwrap()]]);
    let transport = SequenceTransport::new(vec![]);
    let error =
        execute_prepared(&prepared("https://example.com/"), &resolver, &transport).unwrap_err();
    assert_eq!(error.code, FailureCode::TargetStale);
}

#[test]
fn per_hop_dns_drift_is_stale_before_second_request() {
    let resolver = SequenceResolver::new(vec![
        vec![public()],
        vec![public()],
        vec!["1.1.1.1".parse().unwrap()],
    ]);
    let transport = SequenceTransport::new(vec![
        response(302, Some("/next"), b""),
        response(200, None, b"must-not-run"),
    ]);
    let error = execute_prepared(
        &prepared("https://example.com/start"),
        &resolver,
        &transport,
    )
    .unwrap_err();
    assert_eq!(error.code, FailureCode::TargetStale);
}

#[test]
fn peer_mismatch_is_stale_even_after_matching_resolution() {
    let resolver = SequenceResolver::new(public_answers(2));
    let transport = SequenceTransport::new(vec![Ok(TransportResponse {
        status: 200,
        peer: "1.1.1.1".parse().unwrap(),
        location: None,
        body: b"must-not-return".to_vec(),
    })]);
    let error =
        execute_prepared(&prepared("https://example.com/"), &resolver, &transport).unwrap_err();
    assert_eq!(error.code, FailureCode::TargetStale);
}

#[test]
fn redirect_widening_private_literal_and_scheme_downgrade_fail_closed() {
    for location in [
        "https://other.example/path",
        "https://127.0.0.1/private",
        "http://example.com/downgrade",
    ] {
        let resolver = SequenceResolver::new(public_answers(2));
        let transport = SequenceTransport::new(vec![response(302, Some(location), b"")]);
        let error = execute_prepared(
            &prepared("https://example.com/start"),
            &resolver,
            &transport,
        )
        .unwrap_err();
        assert!(
            matches!(
                error.code,
                FailureCode::CapabilityDenied | FailureCode::InvalidRequest
            ),
            "unexpected code for {location}: {:?}",
            error.code
        );
    }
}

#[test]
fn redirect_loop_is_denied() {
    let resolver = SequenceResolver::new(public_answers(2));
    let transport = SequenceTransport::new(vec![response(302, Some("/start"), b"")]);
    let error = execute_prepared(
        &prepared("https://example.com/start"),
        &resolver,
        &transport,
    )
    .unwrap_err();
    assert_eq!(error.code, FailureCode::CapabilityDenied);
}

#[test]
fn redirect_ceiling_is_exact() {
    let mut responses = Vec::new();
    for hop in 0..MAX_REDIRECTS {
        responses.push(response(302, Some(&format!("/hop{}", hop + 1)), b""));
    }
    responses.push(response(200, None, b"ok"));
    let resolver = SequenceResolver::new(public_answers(MAX_REDIRECTS + 2));
    let transport = SequenceTransport::new(responses);
    let result = execute_prepared(
        &prepared("https://example.com/start"),
        &resolver,
        &transport,
    )
    .unwrap();
    assert_eq!(result.hop_count, MAX_REDIRECTS);
    assert_eq!(result.body, b"ok");

    let mut responses = Vec::new();
    for hop in 0..=MAX_REDIRECTS {
        responses.push(response(302, Some(&format!("/too-many{}", hop + 1)), b""));
    }
    let resolver = SequenceResolver::new(public_answers(MAX_REDIRECTS + 2));
    let transport = SequenceTransport::new(responses);
    let error = execute_prepared(
        &prepared("https://example.com/start"),
        &resolver,
        &transport,
    )
    .unwrap_err();
    assert_eq!(error.code, FailureCode::CapabilityDenied);
}

#[test]
fn oversized_response_and_remote_error_body_never_escape() {
    let resolver = SequenceResolver::new(public_answers(2));
    let transport = SequenceTransport::new(vec![response(200, None, &vec![0; MAX_BODY_BYTES + 1])]);
    let error =
        execute_prepared(&prepared("https://example.com/"), &resolver, &transport).unwrap_err();
    assert_eq!(error.code, FailureCode::OutputLimit);

    let resolver = SequenceResolver::new(public_answers(2));
    let transport = SequenceTransport::new(vec![response(500, None, b"remote-secret-error-body")]);
    let error =
        execute_prepared(&prepared("https://example.com/"), &resolver, &transport).unwrap_err();
    assert_eq!(error.code, FailureCode::PostconditionFailed);
    assert!(!error.message.contains("remote-secret"));
}

#[test]
fn transport_timeout_classification_fails_closed() {
    let resolver = SequenceResolver::new(public_answers(2));
    let transport = SequenceTransport::new(vec![Err(NetworkError::new(
        FailureCode::ProviderUnavailable,
        "transport timeout",
    ))]);
    let error =
        execute_prepared(&prepared("https://example.com/"), &resolver, &transport).unwrap_err();
    assert_eq!(error.code, FailureCode::ProviderUnavailable);
}
