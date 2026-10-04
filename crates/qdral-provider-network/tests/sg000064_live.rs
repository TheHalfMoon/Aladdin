//! SG-000064 live network qualification.
//!
//! The live test performs one real HTTPS GET through the native WinHTTP
//! transport and the system resolver, exactly as qdrald does after
//! approval. It needs internet access, so it is ignored by default and run
//! explicitly for evidence:
//! `cargo test -p qdral-provider-network --test sg000064_live -- --ignored`.
//! The refusal test needs no network and always runs.

use qdral_contracts::FailureCode;
use qdral_provider_network::{
    execute_prepared, prepare, SystemResolver, SystemTransport, MAX_BODY_BYTES,
};

#[test]
#[ignore = "performs one real HTTPS request; run with --ignored for live evidence"]
fn live_public_https_fetch_is_address_bound_and_bounded() {
    let resolver = SystemResolver;
    let prepared = prepare("https://example.com/", &resolver).expect("public destination");
    assert!(!prepared.addresses.is_empty());
    let result = execute_prepared(&prepared, &resolver, &SystemTransport).expect("live fetch");
    assert_eq!(result.status, 200);
    assert!(result.body.len() <= MAX_BODY_BYTES);
    let text = String::from_utf8_lossy(&result.body);
    assert!(text.contains("Example Domain"), "unexpected body");
    let json = result.to_json("live", "sg-000064");
    assert_eq!(json["host"], "example.com");
    assert_eq!(json["truncated"], false);
    eprintln!(
        "SG-000064 live network evidence: status={} bytes={} hops={} addresses={}",
        result.status,
        result.body.len(),
        result.hop_count,
        prepared.addresses.len()
    );
}

#[test]
fn private_loopback_and_non_https_destinations_are_refused_before_any_request() {
    let resolver = SystemResolver;
    for url in [
        "http://example.com/",
        "ftp://example.com/",
        "https://127.0.0.1/",
        "https://10.0.0.1/",
        "https://169.254.169.254/latest/meta-data/",
        "https://[::1]/",
        "https://localhost/",
        "https://user:pass@example.com/",
    ] {
        let error = prepare(url, &resolver).expect_err(url);
        assert!(
            matches!(
                error.code,
                FailureCode::InvalidRequest | FailureCode::CapabilityDenied
            ),
            "{url}: {:?}",
            error.code
        );
    }
}
