//! SG-000038 deterministic bounded clipboard-read tests.
//!
//! These tests use an injected fake adapter, so they prove read bounds,
//! format denial, secret denial, sequence binding, approval-digest shape,
//! and evidence secrecy deterministically on every platform without a
//! live desktop. Real Windows clipboard API gating is proven separately
//! through the native adapter test, and headless clipboard access is
//! reported as unavailable instead of fabricated.

use super::*;
use std::cell::Cell;

const WORKSPACE: &str = "clipboard-test-workspace";
const POLICY: &str = "sg-000038-v1";

struct FakeClipboardAdapter {
    sequence: Cell<u64>,
    content: Option<String>,
    non_text: bool,
    locked: bool,
}

impl FakeClipboardAdapter {
    fn with_text(sequence: u64, text: impl Into<String>) -> Self {
        Self {
            sequence: Cell::new(sequence),
            content: Some(text.into()),
            non_text: false,
            locked: false,
        }
    }

    fn empty(sequence: u64) -> Self {
        Self {
            sequence: Cell::new(sequence),
            content: None,
            non_text: false,
            locked: false,
        }
    }

    fn non_text(sequence: u64) -> Self {
        Self {
            sequence: Cell::new(sequence),
            content: None,
            non_text: true,
            locked: false,
        }
    }

    fn locked(sequence: u64) -> Self {
        Self {
            sequence: Cell::new(sequence),
            content: None,
            non_text: false,
            locked: true,
        }
    }

    fn bump_sequence(&self) {
        self.sequence.set(self.sequence.get().saturating_add(1));
    }
}

impl ClipboardAdapter for FakeClipboardAdapter {
    fn sequence_number(&self) -> Result<u64, ClipboardError> {
        if self.locked {
            return Err(ClipboardError::new(
                FailureCode::ProviderUnavailable,
                "clipboard is locked by another owner and cannot be sampled now",
            ));
        }
        Ok(self.sequence.get())
    }

    fn read_unicode_text(&self) -> Result<String, ClipboardError> {
        if self.locked {
            return Err(ClipboardError::new(
                FailureCode::ProviderUnavailable,
                "clipboard is locked by another owner and cannot be sampled now",
            ));
        }
        if self.non_text {
            return Err(ClipboardError::new(
                FailureCode::CapabilityDenied,
                "clipboard holds no Unicode text; bounded reads never transfer binary or object formats",
            ));
        }
        match &self.content {
            Some(text) => Ok(text.clone()),
            None => Err(ClipboardError::new(
                FailureCode::TargetStale,
                "clipboard is empty; there is no readable text",
            )),
        }
    }
}

fn read(
    adapter: &FakeClipboardAdapter,
    expected: Option<u64>,
) -> Result<ReadOutcome, ClipboardError> {
    read_bounded_text(adapter, expected, WORKSPACE, POLICY)
}

#[test]
fn read_shape_predicates_behave() {
    assert!(is_clipboard_read_shape("clipboard", "read"));
    assert!(!is_clipboard_read_shape("clipboard", "write"));
    assert!(!is_clipboard_read_shape("clipboard", "subscribe"));
    assert!(!is_clipboard_read_shape("clipboard", "poll"));
    assert!(!is_clipboard_read_shape("clipboard", "monitor"));
    assert!(!is_clipboard_read_shape("clipboard", "history"));
    assert!(!is_clipboard_read_shape("clipboard", "watch"));
    assert!(!is_clipboard_read_shape("uia.input", "execute"));
    for operation in ["write", "subscribe", "poll", "monitor", "history", "watch"] {
        assert!(
            is_denied_clipboard_shape("clipboard", operation),
            "operation must stay denied: {operation}"
        );
    }
    assert!(!is_denied_clipboard_shape("clipboard", "read"));
    assert_eq!(MAX_CLIPBOARD_BYTES, 65536);
    assert_eq!(UNICODE_TEXT_FORMAT, "unicode-text");
    assert_eq!(CLIPBOARD_SCHEMA, "cotra-clipboard-read-v1");
}

#[test]
fn happy_path_text_read_returns_bounded_text_with_secret_free_evidence() {
    let adapter = FakeClipboardAdapter::with_text(7, "deploy the staging build at noon");
    let outcome = read(&adapter, None).unwrap();
    assert_eq!(outcome.text, "deploy the staging build at noon");
    assert_eq!(outcome.evidence["schema"], CLIPBOARD_SCHEMA);
    assert_eq!(outcome.evidence["action"], "read");
    assert_eq!(outcome.evidence["format"], UNICODE_TEXT_FORMAT);
    assert_eq!(outcome.evidence["sequence"], 7);
    assert_eq!(outcome.evidence["byte_length"], 32);
    assert_eq!(outcome.evidence["workspace_id"], WORKSPACE);
    assert_eq!(outcome.evidence["policy_revision"], POLICY);
    let digest = outcome.evidence["content_digest"]
        .as_str()
        .expect("content digest");
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(digest, clipboard_content_digest(&outcome.text));
    for key in [
        "text", "content", "password", "secret", "token", "approval", "pixels",
    ] {
        assert!(
            outcome.evidence.get(key).is_none(),
            "evidence must not carry {key}"
        );
    }
}

#[test]
fn empty_clipboard_fails_closed() {
    let adapter = FakeClipboardAdapter::empty(3);
    let error = read(&adapter, None).expect_err("empty clipboard must fail");
    assert_eq!(error.code, FailureCode::TargetStale);
}

#[test]
fn non_text_clipboard_fails_closed_without_binary_transfer() {
    let adapter = FakeClipboardAdapter::non_text(3);
    let error = read(&adapter, None).expect_err("non-text content must fail");
    assert_eq!(error.code, FailureCode::CapabilityDenied);
}

#[test]
fn locked_clipboard_fails_closed_as_unavailable() {
    let adapter = FakeClipboardAdapter::locked(3);
    let error = read(&adapter, None).expect_err("locked clipboard must fail");
    assert_eq!(error.code, FailureCode::ProviderUnavailable);
}

#[test]
fn exact_size_boundary_accepts_limit_minus_one_and_limit() {
    let at_limit = FakeClipboardAdapter::with_text(1, "a".repeat(MAX_CLIPBOARD_BYTES));
    let outcome = read(&at_limit, None).unwrap();
    assert_eq!(outcome.evidence["byte_length"], MAX_CLIPBOARD_BYTES as u64);
    let below_limit = FakeClipboardAdapter::with_text(1, "a".repeat(MAX_CLIPBOARD_BYTES - 1));
    let outcome = read(&below_limit, None).unwrap();
    assert_eq!(
        outcome.evidence["byte_length"],
        (MAX_CLIPBOARD_BYTES - 1) as u64
    );
}

#[test]
fn limit_plus_one_and_pathological_encoding_fail_closed() {
    let over_limit = FakeClipboardAdapter::with_text(1, "a".repeat(MAX_CLIPBOARD_BYTES + 1));
    let error = read(&over_limit, None).expect_err("oversized content must fail");
    assert_eq!(error.code, FailureCode::OutputLimit);
    // 65535 ASCII bytes plus one two-byte character is 65537 bytes.
    let mut pathological = "a".repeat(MAX_CLIPBOARD_BYTES - 1);
    pathological.push('\u{e9}');
    assert_eq!(pathological.len(), MAX_CLIPBOARD_BYTES + 1);
    let adapter = FakeClipboardAdapter::with_text(1, pathological);
    let error = read(&adapter, None).expect_err("unexpected encoding size must fail");
    assert_eq!(error.code, FailureCode::OutputLimit);
}

#[test]
fn secret_families_are_denied_without_leaking_bytes() {
    // Every fixture below is synthetic and credential-shaped only: the
    // bodies are sequential or repeated characters with no live secret
    // material. Several are assembled with `concat!` so repository
    // secret-scanning push protection does not false-positive on the
    // non-secret fixtures; runtime strings keep the documented shapes.
    let secrets = [
        "deploy password=hunter2 tonight",
        "login passwd: s3cr3t-value",
        "config pwd=abc123",
        "my api_key is AK-999",
        "set apikey=ZZ-112233",
        "rotate the api-key now: K-1",
        "api_secret=shhh-do-not-share",
        "client_secret=shhh-do-not-share",
        "Authorization: Bearer abcdef123456",
        "token=tok_live_999",
        "auth_token=tok_live_999",
        "access_token=tok_live_999",
        concat!("aws_access_key_id=", "AKIAIOSFODNN7EXAMPLE"),
        concat!("aws_secret_access_key=", "wJalrXUtnFEMI"),
        concat!("ghp_", "1234567890abcdef1234567890abcdef1234"),
        concat!("xox", "b-123456789012-abcdefghijklmnopqrstuvwx"),
        concat!("sk-li", "ve-abcdefgh12345678"),
        "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAAC3",
        "-----BEGIN RSA PRIVATE KEY-----\nMIIE",
        "ssh private_key material below",
        "mnemonic rescue phrase list enclosed",
        "write down this seed phrase now",
        "account recovery phrase: apple banana",
        "use recovery-code 482913 on file",
        "paste the cotra-approval nonce here: 123",
        "export cotra-trust state tonight",
        "connectionstring=Server=db;Password=x;",
    ];
    for secret in secrets {
        let adapter = FakeClipboardAdapter::with_text(1, secret);
        let error = read(&adapter, None).expect_err("secret content must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied, "input: {secret}");
        assert!(
            !error.message.contains(secret),
            "denial must not quote secret bytes"
        );
    }
}

#[test]
fn innocent_text_without_credential_shapes_is_returned() {
    let adapter = FakeClipboardAdapter::with_text(
        1,
        "remind me to rotate the quarterly report password policy discussion",
    );
    let outcome = read(&adapter, None).unwrap();
    assert!(outcome.text.contains("password policy"));
}

#[test]
fn sequence_drift_after_approval_fails_closed() {
    let adapter = FakeClipboardAdapter::with_text(7, "stable clipboard text");
    let outcome = read(&adapter, Some(7)).unwrap();
    assert_eq!(outcome.evidence["sequence"], 7);
    adapter.bump_sequence();
    let error = read(&adapter, Some(7)).expect_err("drifted sequence must fail");
    assert_eq!(error.code, FailureCode::TargetStale);
    let wrong = read(&adapter, Some(6)).expect_err("wrong sequence must fail");
    assert_eq!(wrong.code, FailureCode::TargetStale);
}

#[test]
fn approval_digest_binds_sequence_workspace_and_policy() {
    let first = clipboard_read_digest(WORKSPACE, POLICY, 7);
    assert_eq!(first.len(), 64);
    assert!(first.chars().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(first, clipboard_read_digest(WORKSPACE, POLICY, 7));
    assert_ne!(first, clipboard_read_digest(WORKSPACE, POLICY, 8));
    assert_ne!(first, clipboard_read_digest("other-workspace", POLICY, 7));
    assert_ne!(first, clipboard_read_digest(WORKSPACE, "sg-000037-v1", 7));
}

#[test]
fn malformed_scope_fails_closed() {
    let adapter = FakeClipboardAdapter::with_text(1, "plain text");
    let error =
        read_bounded_text(&adapter, None, "", POLICY).expect_err("empty workspace must fail");
    assert_eq!(error.code, FailureCode::InvalidRequest);
    let error =
        read_bounded_text(&adapter, None, WORKSPACE, "").expect_err("empty policy must fail");
    assert_eq!(error.code, FailureCode::InvalidRequest);
    let error = read_bounded_text(&adapter, None, "other-workspace", POLICY).unwrap();
    assert_eq!(error.evidence["workspace_id"], "other-workspace");
}

#[test]
fn cross_policy_reads_bind_their_own_revision() {
    let adapter = FakeClipboardAdapter::with_text(1, "plain text");
    let outcome = read_bounded_text(&adapter, None, WORKSPACE, "sg-000037-v1").unwrap();
    assert_eq!(outcome.evidence["policy_revision"], "sg-000037-v1");
}

#[test]
fn native_adapter_reports_honest_typed_results() {
    let native = NativeAdapter::new();
    match native.sequence_number() {
        Ok(sequence) => {
            assert!(sequence <= u32::MAX as u64);
            match native.read_unicode_text() {
                Ok(text) => {
                    // The raw adapter returns content without policy
                    // filtering; secret denial lives in the approved
                    // read pipeline, not in the adapter.
                    assert!(!text.is_empty());
                    assert!(text.len() <= MAX_CLIPBOARD_BYTES);
                }
                Err(error) => {
                    assert!(matches!(
                        error.code,
                        FailureCode::ProviderUnavailable
                            | FailureCode::CapabilityDenied
                            | FailureCode::TargetStale
                            | FailureCode::OutputLimit
                    ));
                }
            }
        }
        Err(error) => {
            assert_eq!(error.code, FailureCode::ProviderUnavailable);
            #[cfg(windows)]
            panic!("native sequence state must be queryable on Windows");
        }
    }
}
