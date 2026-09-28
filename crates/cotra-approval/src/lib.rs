use cotra_contracts::FailureCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const APPROVAL_TTL_MS: u64 = 5 * 60 * 1_000;
pub const SINGLE_OPERATION_SCOPE: &str = "single-operation";
pub const STRONG_VERIFY_TIMEOUT_MS: u64 = 120_000;
const LEDGER_SCHEMA: &str = "cotra-approval-ledger-v3";

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalClass {
    #[serde(rename = "SOFT")]
    Soft,
    #[serde(rename = "STRONG")]
    Strong,
}

impl ApprovalClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalClass::Soft => "SOFT",
            ApprovalClass::Strong => "STRONG",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresenceOutcome {
    #[serde(rename = "soft-approved")]
    SoftApproved,
    #[serde(rename = "verified-strong")]
    VerifiedStrong,
    #[serde(rename = "denied")]
    Denied,
    #[serde(rename = "unavailable")]
    Unavailable,
}

impl PresenceOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            PresenceOutcome::SoftApproved => "soft-approved",
            PresenceOutcome::VerifiedStrong => "verified-strong",
            PresenceOutcome::Denied => "denied",
            PresenceOutcome::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceContext {
    pub nonce: String,
    pub digest: String,
    pub workspace_id: String,
    pub action: String,
}

impl PresenceContext {
    pub fn new(
        nonce: impl Into<String>,
        digest: impl Into<String>,
        workspace_id: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            nonce: nonce.into(),
            digest: digest.into(),
            workspace_id: workspace_id.into(),
            action: action.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceAttestation {
    pub method: String,
    pub verified_at_ms: u64,
    pub nonce: String,
    pub digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceFailureReason {
    Denied,
    Cancelled,
    Timeout,
    Unavailable,
    Failed,
    InvalidResponse,
}

#[derive(Debug, Clone)]
pub struct PresenceError {
    pub code: FailureCode,
    pub message: String,
    pub reason: PresenceFailureReason,
}

impl PresenceError {
    pub fn denied(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalDenied,
            message: message.into(),
            reason: PresenceFailureReason::Denied,
        }
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalDenied,
            message: message.into(),
            reason: PresenceFailureReason::Cancelled,
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalUnavailable,
            message: message.into(),
            reason: PresenceFailureReason::Timeout,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalUnavailable,
            message: message.into(),
            reason: PresenceFailureReason::Unavailable,
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalUnavailable,
            message: message.into(),
            reason: PresenceFailureReason::Failed,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalUnavailable,
            message: message.into(),
            reason: PresenceFailureReason::InvalidResponse,
        }
    }
}

#[derive(Debug)]
pub struct InputLeaseGuard {
    suspended: bool,
}

impl InputLeaseGuard {
    pub(crate) fn suspend_for_presence() -> Self {
        Self { suspended: true }
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended
    }
}

impl Drop for InputLeaseGuard {
    fn drop(&mut self) {
        self.suspended = false;
    }
}

pub trait PresenceVerifier: Send + Sync + std::fmt::Debug {
    fn method(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn verify(
        &self,
        ctx: &PresenceContext,
        lease: &InputLeaseGuard,
    ) -> Result<PresenceAttestation, PresenceError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoPresenceVerifier;

impl PresenceVerifier for NoPresenceVerifier {
    fn method(&self) -> &'static str {
        "none"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn verify(
        &self,
        _ctx: &PresenceContext,
        _lease: &InputLeaseGuard,
    ) -> Result<PresenceAttestation, PresenceError> {
        Err(PresenceError::unavailable(
            "strong user-presence verification is unavailable on this platform; STRONG approval fails closed without SOFT downgrade",
        ))
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsHelloPresenceVerifier;

#[cfg(windows)]
impl WindowsHelloPresenceVerifier {
    pub fn new() -> Self {
        Self
    }

    fn hello_availability(&self) -> bool {
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return false;
        }
        check_hello_availability()
    }
}

#[cfg(windows)]
impl PresenceVerifier for WindowsHelloPresenceVerifier {
    fn method(&self) -> &'static str {
        "windows-hello"
    }

    fn is_available(&self) -> bool {
        self.hello_availability()
    }

    fn verify(
        &self,
        ctx: &PresenceContext,
        lease: &InputLeaseGuard,
    ) -> Result<PresenceAttestation, PresenceError> {
        if !lease.is_suspended() {
            return Err(PresenceError::invalid(
                "strong verification requires a suspended input lease",
            ));
        }
        if ctx.nonce.trim().is_empty() || ctx.digest.trim().is_empty() {
            return Err(PresenceError::invalid(
                "strong verification context is malformed",
            ));
        }
        if !self.hello_availability() {
            return Err(PresenceError::unavailable(
                "Windows Hello verification is unavailable; STRONG approval fails closed",
            ));
        }
        verify_with_hello(ctx, self.method())
    }
}

#[cfg(windows)]
fn check_hello_availability() -> bool {
    use windows::Security::Credentials::UI::UserConsentVerifier;
    let result = (|| -> windows::core::Result<bool> {
        let operation = UserConsentVerifier::CheckAvailabilityAsync()?;
        let availability = operation.get()?;
        Ok(availability
            == windows::Security::Credentials::UI::UserConsentVerifierAvailability::Available)
    })();
    result.unwrap_or(false)
}

#[cfg(windows)]
fn verify_with_hello(
    ctx: &PresenceContext,
    method: &'static str,
) -> Result<PresenceAttestation, PresenceError> {
    use std::sync::mpsc::channel;
    use std::time::Duration;
    use windows::Security::Credentials::UI::{UserConsentVerificationResult, UserConsentVerifier};

    let nonce_prefix: String = ctx.nonce.chars().take(8).collect();
    let digest_prefix: String = ctx.digest.chars().take(12).collect();
    let message = format!(
        "Cotra STRONG approval {nonce_prefix} {digest_prefix} for {} in workspace {}",
        ctx.action, ctx.workspace_id
    );
    let nonce = ctx.nonce.clone();
    let digest = ctx.digest.clone();
    let (sender, receiver) = channel();
    std::thread::spawn(move || {
        let outcome = (|| -> windows::core::Result<UserConsentVerificationResult> {
            let operation = UserConsentVerifier::RequestVerificationAsync(&message.clone().into())?;
            operation.get()
        })();
        let _ = sender.send(outcome);
    });
    let outcome = receiver
        .recv_timeout(Duration::from_millis(STRONG_VERIFY_TIMEOUT_MS))
        .map_err(|_| {
            PresenceError::timeout(
                "strong user-presence verification timed out; STRONG approval fails closed",
            )
        })?;
    let result = outcome.map_err(|_| {
        PresenceError::failed("Windows Hello verification failed; STRONG approval fails closed")
    })?;
    match result {
        UserConsentVerificationResult::Verified => Ok(PresenceAttestation {
            method: method.to_owned(),
            verified_at_ms: now_ms(),
            nonce,
            digest,
        }),
        UserConsentVerificationResult::DeviceNotPresent
        | UserConsentVerificationResult::NotConfiguredForUser
        | UserConsentVerificationResult::DisabledByPolicy
        | UserConsentVerificationResult::DeviceBusy => Err(PresenceError::unavailable(
            "Windows Hello is not configured for this device; STRONG approval fails closed",
        )),
        UserConsentVerificationResult::Canceled => Err(PresenceError::cancelled(
            "strong user-presence verification was cancelled; STRONG approval fails closed",
        )),
        UserConsentVerificationResult::RetriesExhausted => Err(PresenceError::denied(
            "strong user-presence verification retries exhausted; STRONG approval fails closed",
        )),
        _ => Err(PresenceError::invalid(
            "Windows Hello returned an unexpected result; STRONG approval fails closed",
        )),
    }
}

#[cfg(not(windows))]
pub type DefaultPresenceVerifier = NoPresenceVerifier;

#[cfg(windows)]
pub type DefaultPresenceVerifier = WindowsHelloPresenceVerifier;

fn default_presence_verifier() -> Arc<dyn PresenceVerifier> {
    Arc::new(DefaultPresenceVerifier::default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPrompt {
    pub workspace_id: String,
    pub policy_revision: String,
    pub action: String,
    pub target: String,
    pub summary: String,
    pub digest: String,
    pub nonce: String,
    pub requested_at_ms: u64,
    pub expires_at_ms: u64,
    pub reuse_scope: String,
    pub approval_class: ApprovalClass,
}

impl ApprovalPrompt {
    pub fn new(
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        summary: impl Into<String>,
        digest: impl Into<String>,
    ) -> Self {
        Self::new_with_clock(
            workspace_id,
            policy_revision,
            action,
            target,
            summary,
            digest,
            now_ms(),
        )
    }

    pub fn new_with_clock(
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        summary: impl Into<String>,
        digest: impl Into<String>,
        now_ms: u64,
    ) -> Self {
        Self::new_with_class_and_clock(
            workspace_id,
            policy_revision,
            action,
            target,
            summary,
            digest,
            ApprovalClass::Soft,
            now_ms,
        )
    }

    pub fn new_strong(
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        summary: impl Into<String>,
        digest: impl Into<String>,
    ) -> Self {
        Self::new_strong_with_clock(
            workspace_id,
            policy_revision,
            action,
            target,
            summary,
            digest,
            now_ms(),
        )
    }

    pub fn new_strong_with_clock(
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        summary: impl Into<String>,
        digest: impl Into<String>,
        now_ms: u64,
    ) -> Self {
        Self::new_with_class_and_clock(
            workspace_id,
            policy_revision,
            action,
            target,
            summary,
            digest,
            ApprovalClass::Strong,
            now_ms,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_class_and_clock(
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        summary: impl Into<String>,
        digest: impl Into<String>,
        approval_class: ApprovalClass,
        now_ms: u64,
    ) -> Self {
        let digest = digest.into();
        Self {
            workspace_id: workspace_id.into(),
            policy_revision: policy_revision.into(),
            action: action.into(),
            target: target.into(),
            summary: summary.into(),
            nonce: fresh_nonce(&digest, now_ms),
            requested_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(APPROVAL_TTL_MS),
            reuse_scope: SINGLE_OPERATION_SCOPE.to_owned(),
            digest,
            approval_class,
        }
    }

    pub fn with_class(mut self, approval_class: ApprovalClass) -> Self {
        let now = self.requested_at_ms;
        self.nonce = fresh_nonce(&self.digest, now);
        self.approval_class = approval_class;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approved,
    Denied,
}

#[derive(Debug, Clone)]
pub struct ApprovalError {
    pub code: FailureCode,
    pub message: String,
}

impl ApprovalError {
    fn denied(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalDenied,
            message: message.into(),
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::ApprovalUnavailable,
            message: message.into(),
        }
    }

    fn stale(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::TargetStale,
            message: message.into(),
        }
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: FailureCode::InvalidRequest,
            message: message.into(),
        }
    }
}

impl From<PresenceError> for ApprovalError {
    fn from(error: PresenceError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedToken {
    pub record_id: String,
    pub nonce: String,
    pub digest: String,
    pub workspace_id: String,
    pub policy_revision: String,
    pub expires_at_ms: u64,
    pub approval_class: ApprovalClass,
    pub epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeExpectation {
    pub digest: String,
    pub workspace_id: String,
    pub policy_revision: String,
    pub approval_class: ApprovalClass,
}

impl ConsumeExpectation {
    pub fn new(
        digest: impl Into<String>,
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
    ) -> Self {
        Self {
            digest: digest.into(),
            workspace_id: workspace_id.into(),
            policy_revision: policy_revision.into(),
            approval_class: ApprovalClass::Soft,
        }
    }

    pub fn new_with_class(
        digest: impl Into<String>,
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
        approval_class: ApprovalClass,
    ) -> Self {
        Self {
            digest: digest.into(),
            workspace_id: workspace_id.into(),
            policy_revision: policy_revision.into(),
            approval_class,
        }
    }

    pub fn strong(
        digest: impl Into<String>,
        workspace_id: impl Into<String>,
        policy_revision: impl Into<String>,
    ) -> Self {
        Self::new_with_class(digest, workspace_id, policy_revision, ApprovalClass::Strong)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedDecision {
    Approved,
    Denied,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredRecord {
    schema: String,
    id: String,
    nonce: String,
    digest: String,
    workspace_id: String,
    policy_revision: String,
    decision: RecordedDecision,
    approval_class: ApprovalClass,
    presence_outcome: PresenceOutcome,
    presence_method: String,
    epoch: u64,
    is_revoke: bool,
    requested_at_ms: u64,
    decided_at_ms: u64,
    expires_at_ms: u64,
    reuse_scope: String,
    consumed: bool,
    prev_checksum: String,
    checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecordSummary {
    pub id: String,
    pub digest: String,
    pub workspace_id: String,
    pub policy_revision: String,
    pub decision: RecordedDecision,
    pub approval_class: ApprovalClass,
    pub presence_outcome: PresenceOutcome,
    pub presence_method: String,
    pub epoch: u64,
    pub is_revoke: bool,
    pub requested_at_ms: u64,
    pub decided_at_ms: u64,
    pub expires_at_ms: u64,
    pub reuse_scope: String,
    pub consumed: bool,
}

pub trait ApprovalBroker {
    fn request_token(&self, prompt: &ApprovalPrompt) -> Result<ApprovedToken, ApprovalError>;

    fn consume(
        &self,
        token: &ApprovedToken,
        expected: &ConsumeExpectation,
        now_ms: u64,
    ) -> Result<(), ApprovalError>;

    fn emergency_revoke(&self, _prompt: &ApprovalPrompt) -> Result<u64, ApprovalError> {
        Err(ApprovalError::unavailable(
            "emergency revoke is unavailable through this broker",
        ))
    }

    fn request(&self, prompt: &ApprovalPrompt) -> Result<ApprovalDecision, ApprovalError> {
        let token = self.request_token(prompt)?;
        let expected = ConsumeExpectation::new_with_class(
            prompt.digest.clone(),
            prompt.workspace_id.clone(),
            prompt.policy_revision.clone(),
            prompt.approval_class,
        );
        self.consume(&token, &expected, now_ms())?;
        Ok(ApprovalDecision::Approved)
    }

    fn history(&self, _limit: usize) -> Vec<ApprovalRecordSummary> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct LocalApprovalBroker {
    ledger: Mutex<ApprovalLedger>,
    presence: Arc<dyn PresenceVerifier>,
}

impl LocalApprovalBroker {
    pub fn new() -> Self {
        Self::with_path(default_approval_history_path())
    }

    pub fn with_path(path: impl Into<std::path::PathBuf>) -> Self {
        Self::with_path_and_verifier(path, default_presence_verifier())
    }

    pub fn with_presence_verifier(presence: Arc<dyn PresenceVerifier>) -> Self {
        Self::with_path_and_verifier(default_approval_history_path(), presence)
    }

    pub fn with_path_and_verifier(
        path: impl Into<std::path::PathBuf>,
        presence: Arc<dyn PresenceVerifier>,
    ) -> Self {
        let ledger = ApprovalLedger::load_or_create(path.into());
        Self {
            ledger: Mutex::new(ledger),
            presence,
        }
    }

    pub fn presence_method(&self) -> &'static str {
        self.presence.method()
    }

    pub fn presence_available(&self) -> bool {
        self.presence.is_available()
    }

    pub fn history(&self, limit: usize) -> Vec<ApprovalRecordSummary> {
        self.ledger
            .lock()
            .map(|ledger| ledger.history(limit))
            .unwrap_or_default()
    }
}

impl Default for LocalApprovalBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalBroker for LocalApprovalBroker {
    fn request_token(&self, prompt: &ApprovalPrompt) -> Result<ApprovedToken, ApprovalError> {
        validate_prompt(prompt)?;
        match prompt.approval_class {
            ApprovalClass::Soft => self.request_soft_token(prompt),
            ApprovalClass::Strong => self.request_strong_token(prompt),
        }
    }

    fn consume(
        &self,
        token: &ApprovedToken,
        expected: &ConsumeExpectation,
        now_ms: u64,
    ) -> Result<(), ApprovalError> {
        self.ledger
            .lock()
            .map_err(|_| ApprovalError::unavailable("approval ledger is unavailable"))?
            .consume(token, expected, now_ms)
    }

    fn history(&self, limit: usize) -> Vec<ApprovalRecordSummary> {
        Self::history(self, limit)
    }

    fn emergency_revoke(&self, prompt: &ApprovalPrompt) -> Result<u64, ApprovalError> {
        validate_prompt(prompt)?;
        if prompt.approval_class != ApprovalClass::Strong {
            return Err(ApprovalError::invalid(
                "emergency revoke requires the STRONG class",
            ));
        }
        let lease = InputLeaseGuard::suspend_for_presence();
        if !lease.is_suspended() {
            return Err(ApprovalError::unavailable(
                "emergency revoke requires a suspended input lease",
            ));
        }
        let ctx = PresenceContext::new(
            prompt.nonce.clone(),
            prompt.digest.clone(),
            prompt.workspace_id.clone(),
            prompt.action.clone(),
        );
        let method = self.presence.method();
        let attestation = self.presence.verify(&ctx, &lease);
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| ApprovalError::unavailable("approval ledger is unavailable"))?;
        match attestation {
            Ok(attestation) => {
                if attestation.nonce != prompt.nonce || attestation.digest != prompt.digest {
                    let _ = ledger.record_decision(
                        prompt,
                        RecordedDecision::Unavailable,
                        PresenceOutcome::Unavailable,
                        method,
                    );
                    return Err(ApprovalError::unavailable(
                        "revoke presence result does not match the request; revoke fails closed",
                    ));
                }
                let record = ledger.record_revoke(prompt, &attestation.method)?;
                Ok(record.epoch)
            }
            Err(error) => {
                let outcome = match error.reason {
                    PresenceFailureReason::Denied | PresenceFailureReason::Cancelled => {
                        PresenceOutcome::Denied
                    }
                    _ => PresenceOutcome::Unavailable,
                };
                let decision = match outcome {
                    PresenceOutcome::Denied => RecordedDecision::Denied,
                    _ => RecordedDecision::Unavailable,
                };
                let _ = ledger.record_decision(prompt, decision, outcome, method);
                Err(error.into())
            }
        }
    }
}

impl LocalApprovalBroker {
    fn request_soft_token(&self, prompt: &ApprovalPrompt) -> Result<ApprovedToken, ApprovalError> {
        let decision = platform_prompt(prompt);
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| ApprovalError::unavailable("approval ledger is unavailable"))?;
        match decision {
            Ok(ApprovalDecision::Approved) => {
                let record = ledger.record_decision(
                    prompt,
                    RecordedDecision::Approved,
                    PresenceOutcome::SoftApproved,
                    "soft-button",
                )?;
                Ok(ApprovedToken {
                    record_id: record.id.clone(),
                    nonce: record.nonce.clone(),
                    digest: record.digest.clone(),
                    workspace_id: record.workspace_id.clone(),
                    policy_revision: record.policy_revision.clone(),
                    expires_at_ms: record.expires_at_ms,
                    approval_class: ApprovalClass::Soft,
                    epoch: record.epoch,
                })
            }
            Ok(ApprovalDecision::Denied) => {
                let _ = ledger.record_decision(
                    prompt,
                    RecordedDecision::Denied,
                    PresenceOutcome::Denied,
                    "soft-button",
                );
                Err(ApprovalError::denied("local user denied the operation"))
            }
            Err(error) => {
                let _ = ledger.record_decision(
                    prompt,
                    RecordedDecision::Unavailable,
                    PresenceOutcome::Unavailable,
                    "soft-button",
                );
                Err(error)
            }
        }
    }

    fn request_strong_token(
        &self,
        prompt: &ApprovalPrompt,
    ) -> Result<ApprovedToken, ApprovalError> {
        let lease = InputLeaseGuard::suspend_for_presence();
        if !lease.is_suspended() {
            return Err(ApprovalError::unavailable(
                "strong approval requires a suspended input lease",
            ));
        }
        let ctx = PresenceContext::new(
            prompt.nonce.clone(),
            prompt.digest.clone(),
            prompt.workspace_id.clone(),
            prompt.action.clone(),
        );
        let method = self.presence.method();
        let attestation = self.presence.verify(&ctx, &lease);
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| ApprovalError::unavailable("approval ledger is unavailable"))?;
        match attestation {
            Ok(attestation) => {
                if attestation.nonce != prompt.nonce || attestation.digest != prompt.digest {
                    let _ = ledger.record_decision(
                        prompt,
                        RecordedDecision::Unavailable,
                        PresenceOutcome::Unavailable,
                        method,
                    );
                    return Err(ApprovalError::unavailable(
                        "strong presence result does not match the approved request; STRONG approval fails closed",
                    ));
                }
                if attestation.method != method {
                    let _ = ledger.record_decision(
                        prompt,
                        RecordedDecision::Unavailable,
                        PresenceOutcome::Unavailable,
                        method,
                    );
                    return Err(ApprovalError::unavailable(
                        "strong presence method mismatch; STRONG approval fails closed",
                    ));
                }
                let record = ledger.record_decision(
                    prompt,
                    RecordedDecision::Approved,
                    PresenceOutcome::VerifiedStrong,
                    &attestation.method,
                )?;
                Ok(ApprovedToken {
                    record_id: record.id.clone(),
                    nonce: record.nonce.clone(),
                    digest: record.digest.clone(),
                    workspace_id: record.workspace_id.clone(),
                    policy_revision: record.policy_revision.clone(),
                    expires_at_ms: record.expires_at_ms,
                    approval_class: ApprovalClass::Strong,
                    epoch: record.epoch,
                })
            }
            Err(error) => {
                let outcome = match error.reason {
                    PresenceFailureReason::Denied | PresenceFailureReason::Cancelled => {
                        PresenceOutcome::Denied
                    }
                    _ => PresenceOutcome::Unavailable,
                };
                let decision = match outcome {
                    PresenceOutcome::Denied => RecordedDecision::Denied,
                    _ => RecordedDecision::Unavailable,
                };
                let _ = ledger.record_decision(prompt, decision, outcome, method);
                Err(error.into())
            }
        }
    }
}

fn validate_prompt(prompt: &ApprovalPrompt) -> Result<(), ApprovalError> {
    if prompt.workspace_id.trim().is_empty() {
        return Err(ApprovalError::invalid("approval prompt workspace is empty"));
    }
    if prompt.policy_revision.trim().is_empty() {
        return Err(ApprovalError::invalid(
            "approval prompt policy revision is empty",
        ));
    }
    if prompt.digest.trim().is_empty() {
        return Err(ApprovalError::invalid("approval prompt digest is empty"));
    }
    if prompt.nonce.trim().is_empty() {
        return Err(ApprovalError::invalid("approval prompt nonce is empty"));
    }
    if prompt.reuse_scope != SINGLE_OPERATION_SCOPE {
        return Err(ApprovalError::invalid(
            "approval prompt reuse scope must be single-operation",
        ));
    }
    if prompt.expires_at_ms <= prompt.requested_at_ms {
        return Err(ApprovalError::invalid("approval prompt expiry is invalid"));
    }
    Ok(())
}

#[derive(Debug)]
struct ApprovalLedger {
    path: std::path::PathBuf,
    records: BTreeMap<String, StoredRecord>,
    nonces: HashSet<String>,
    consumed: HashSet<String>,
    tip: String,
    revoke_epoch: u64,
}

impl ApprovalLedger {
    fn load_or_create(path: std::path::PathBuf) -> Self {
        let mut ledger = Self {
            path,
            records: BTreeMap::new(),
            nonces: HashSet::new(),
            consumed: HashSet::new(),
            tip: String::from("GENESIS"),
            revoke_epoch: 0,
        };
        if ledger.path.is_file() {
            if let Ok(text) = std::fs::read_to_string(&ledger.path) {
                let mut tip = String::from("GENESIS");
                for line in text.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let Ok(record) = serde_json::from_str::<StoredRecord>(line) else {
                        break;
                    };
                    if record.schema != LEDGER_SCHEMA || !verify_record_checksum(&record, &tip) {
                        break;
                    }
                    tip = record.checksum.clone();
                    ledger.nonces.insert(record.nonce.clone());
                    if record.consumed {
                        ledger.consumed.insert(record.nonce.clone());
                    }
                    if record.epoch > ledger.revoke_epoch {
                        ledger.revoke_epoch = record.epoch;
                    }
                    ledger.records.insert(record.id.clone(), record);
                }
                ledger.tip = tip;
            }
        } else if let Some(parent) = ledger.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        ledger
    }

    fn record_decision(
        &mut self,
        prompt: &ApprovalPrompt,
        decision: RecordedDecision,
        presence_outcome: PresenceOutcome,
        presence_method: &str,
    ) -> Result<StoredRecord, ApprovalError> {
        if !self.nonces.insert(prompt.nonce.clone()) {
            return Err(ApprovalError::invalid(
                "approval nonce was already issued; retry with a fresh prompt",
            ));
        }
        if prompt.approval_class == ApprovalClass::Strong
            && presence_outcome == PresenceOutcome::SoftApproved
        {
            return Err(ApprovalError::invalid(
                "a soft button alone never satisfies the STRONG class",
            ));
        }
        let decided_at = now_ms();
        let id = format!(
            "apr-{}-{}",
            decided_at,
            NONCE_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let mut record = StoredRecord {
            schema: LEDGER_SCHEMA.to_owned(),
            id,
            nonce: prompt.nonce.clone(),
            digest: prompt.digest.clone(),
            workspace_id: prompt.workspace_id.clone(),
            policy_revision: prompt.policy_revision.clone(),
            decision,
            approval_class: prompt.approval_class,
            presence_outcome,
            presence_method: presence_method.to_owned(),
            epoch: self.revoke_epoch,
            is_revoke: false,
            requested_at_ms: prompt.requested_at_ms,
            decided_at_ms: decided_at,
            expires_at_ms: prompt.expires_at_ms,
            reuse_scope: prompt.reuse_scope.clone(),
            consumed: false,
            prev_checksum: self.tip.clone(),
            checksum: String::new(),
        };
        record.checksum = record_checksum(&record);
        self.tip = record.checksum.clone();
        self.append_to_file(&record)?;
        self.records.insert(record.id.clone(), record.clone());
        Ok(record)
    }

    fn record_revoke(
        &mut self,
        prompt: &ApprovalPrompt,
        presence_method: &str,
    ) -> Result<StoredRecord, ApprovalError> {
        if prompt.approval_class != ApprovalClass::Strong {
            return Err(ApprovalError::invalid(
                "emergency revoke requires the STRONG class",
            ));
        }
        if !self.nonces.insert(prompt.nonce.clone()) {
            return Err(ApprovalError::invalid(
                "approval nonce was already issued; retry with a fresh prompt",
            ));
        }
        let new_epoch = self.revoke_epoch.saturating_add(1);
        let decided_at = now_ms();
        let id = format!(
            "rev-{}-{}",
            decided_at,
            NONCE_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let mut record = StoredRecord {
            schema: LEDGER_SCHEMA.to_owned(),
            id,
            nonce: prompt.nonce.clone(),
            digest: prompt.digest.clone(),
            workspace_id: prompt.workspace_id.clone(),
            policy_revision: prompt.policy_revision.clone(),
            decision: RecordedDecision::Approved,
            approval_class: ApprovalClass::Strong,
            presence_outcome: PresenceOutcome::VerifiedStrong,
            presence_method: presence_method.to_owned(),
            epoch: new_epoch,
            is_revoke: true,
            requested_at_ms: prompt.requested_at_ms,
            decided_at_ms: decided_at,
            expires_at_ms: prompt.expires_at_ms,
            reuse_scope: prompt.reuse_scope.clone(),
            consumed: true,
            prev_checksum: self.tip.clone(),
            checksum: String::new(),
        };
        record.checksum = record_checksum(&record);
        self.tip = record.checksum.clone();
        self.revoke_epoch = new_epoch;
        self.append_to_file(&record)?;
        self.records.insert(record.id.clone(), record.clone());
        Ok(record)
    }

    #[allow(dead_code)]
    fn current_epoch(&self) -> u64 {
        self.revoke_epoch
    }

    fn consume(
        &mut self,
        token: &ApprovedToken,
        expected: &ConsumeExpectation,
        now_ms: u64,
    ) -> Result<(), ApprovalError> {
        let record = self.records.get(&token.record_id).ok_or_else(|| {
            ApprovalError::invalid("approval token references an unknown approval record")
        })?;
        if record.decision != RecordedDecision::Approved {
            return Err(ApprovalError::denied(
                "approval token was not approved and cannot authorize execution",
            ));
        }
        if record.is_revoke {
            return Err(ApprovalError::denied(
                "revoke records never authorize execution",
            ));
        }
        if record.epoch != self.revoke_epoch || token.epoch != self.revoke_epoch {
            return Err(ApprovalError::denied(
                "approval was revoked by emergency revoke and cannot authorize execution",
            ));
        }
        if record.epoch != token.epoch {
            return Err(ApprovalError::stale(
                "approval epoch changed after approval; approval cannot be reused",
            ));
        }
        if record.nonce != token.nonce {
            return Err(ApprovalError::stale(
                "approval token nonce does not match the recorded approval",
            ));
        }
        if record.approval_class != token.approval_class {
            return Err(ApprovalError::stale(
                "approval class changed after approval; approval cannot be reused",
            ));
        }
        if record.approval_class != expected.approval_class
            || token.approval_class != expected.approval_class
        {
            return Err(ApprovalError::stale(
                "approval class does not match the expected class; STRONG never downgrades to SOFT",
            ));
        }
        if now_ms > record.expires_at_ms || now_ms > token.expires_at_ms {
            return Err(ApprovalError::denied(
                "local approval expired before execution",
            ));
        }
        if record.digest != expected.digest || token.digest != expected.digest {
            return Err(ApprovalError::stale(
                "operation digest changed after approval; approval cannot be reused",
            ));
        }
        if record.workspace_id != expected.workspace_id
            || token.workspace_id != expected.workspace_id
        {
            return Err(ApprovalError::stale(
                "workspace changed after approval; approval cannot be reused",
            ));
        }
        if record.policy_revision != expected.policy_revision
            || token.policy_revision != expected.policy_revision
        {
            return Err(ApprovalError::stale(
                "policy revision changed after approval; approval cannot be reused",
            ));
        }
        if record.consumed || !self.consumed.insert(record.nonce.clone()) {
            return Err(ApprovalError::denied(
                "approval was already consumed and cannot authorize another operation",
            ));
        }
        if let Some(stored) = self.records.get_mut(&token.record_id) {
            stored.consumed = true;
            let updated = stored.clone();
            self.tip = updated.checksum.clone();
            self.append_consumed_marker(&updated).map_err(|error| {
                ApprovalError::unavailable(format!(
                    "persist approval consumption; retry with a fresh approval: {error}"
                ))
            })?;
        }
        Ok(())
    }

    fn append_to_file(&self, record: &StoredRecord) -> Result<(), ApprovalError> {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| {
                ApprovalError::unavailable(format!("persist approval record: {error}"))
            })?;
        serde_json::to_writer(&mut file, record).map_err(|error| {
            ApprovalError::unavailable(format!("serialize approval record: {error}"))
        })?;
        file.write_all(b"\n").map_err(|error| {
            ApprovalError::unavailable(format!("persist approval record: {error}"))
        })?;
        file.flush().map_err(|error| {
            ApprovalError::unavailable(format!("persist approval record: {error}"))
        })?;
        Ok(())
    }

    fn append_consumed_marker(&self, record: &StoredRecord) -> std::io::Result<()> {
        use std::io::Write as _;
        let mut marker = record.clone();
        marker.consumed = true;
        marker.prev_checksum = self.tip.clone();
        marker.checksum = record_checksum(&marker);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        serde_json::to_writer(&mut file, &marker)?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(())
    }

    fn history(&self, limit: usize) -> Vec<ApprovalRecordSummary> {
        let bound = limit.clamp(1, 200);
        self.records
            .values()
            .rev()
            .take(bound)
            .map(|record| ApprovalRecordSummary {
                id: record.id.clone(),
                digest: record.digest.clone(),
                workspace_id: record.workspace_id.clone(),
                policy_revision: record.policy_revision.clone(),
                decision: record.decision,
                approval_class: record.approval_class,
                presence_outcome: record.presence_outcome,
                presence_method: record.presence_method.clone(),
                epoch: record.epoch,
                is_revoke: record.is_revoke,
                requested_at_ms: record.requested_at_ms,
                decided_at_ms: record.decided_at_ms,
                expires_at_ms: record.expires_at_ms,
                reuse_scope: record.reuse_scope.clone(),
                consumed: self.consumed.contains(&record.nonce),
            })
            .collect()
    }
}

fn record_checksum(record: &StoredRecord) -> String {
    let mut hasher = Sha256::new();
    checksum_field(&mut hasher, LEDGER_SCHEMA.as_bytes());
    checksum_field(&mut hasher, record.id.as_bytes());
    checksum_field(&mut hasher, record.nonce.as_bytes());
    checksum_field(&mut hasher, record.digest.as_bytes());
    checksum_field(&mut hasher, record.workspace_id.as_bytes());
    checksum_field(&mut hasher, record.policy_revision.as_bytes());
    checksum_field(&mut hasher, format!("{:?}", record.decision).as_bytes());
    checksum_field(&mut hasher, record.approval_class.as_str().as_bytes());
    checksum_field(&mut hasher, record.presence_outcome.as_str().as_bytes());
    checksum_field(&mut hasher, record.presence_method.as_bytes());
    checksum_field(&mut hasher, record.epoch.to_string().as_bytes());
    checksum_field(
        &mut hasher,
        (if record.is_revoke { "1" } else { "0" }).as_bytes(),
    );
    checksum_field(&mut hasher, record.requested_at_ms.to_string().as_bytes());
    checksum_field(&mut hasher, record.decided_at_ms.to_string().as_bytes());
    checksum_field(&mut hasher, record.expires_at_ms.to_string().as_bytes());
    checksum_field(&mut hasher, record.reuse_scope.as_bytes());
    checksum_field(
        &mut hasher,
        (if record.consumed { "1" } else { "0" }).as_bytes(),
    );
    checksum_field(&mut hasher, record.prev_checksum.as_bytes());
    hex_lower(&hasher.finalize())
}

fn verify_record_checksum(record: &StoredRecord, tip: &str) -> bool {
    if record.prev_checksum != tip {
        return false;
    }
    record.checksum == record_checksum(record)
}

fn checksum_field(hasher: &mut Sha256, value: &[u8]) {
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

fn fresh_nonce(digest: &str, now_ms: u64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher as _};
    let count = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = DefaultHasher::new();
    std::process::id().hash(&mut hasher);
    now_ms.hash(&mut hasher);
    count.hash(&mut hasher);
    digest.hash(&mut hasher);
    format!("{:016x}{:016x}", hasher.finish(), count ^ now_ms)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn default_approval_history_path() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("COTRA_APPROVAL_HISTORY_PATH") {
        return std::path::PathBuf::from(path);
    }
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        return std::path::PathBuf::from(local_app_data)
            .join("Cotra")
            .join("approval-history.jsonl");
    }
    std::env::temp_dir()
        .join("cotra")
        .join("approval-history.jsonl")
}

#[cfg(windows)]
fn platform_prompt(prompt: &ApprovalPrompt) -> Result<ApprovalDecision, ApprovalError> {
    use std::ffi::c_void;
    use std::ptr;

    type Hwnd = *mut c_void;
    const MB_YESNO: u32 = 0x0000_0004;
    const MB_ICONWARNING: u32 = 0x0000_0030;
    const MB_DEFBUTTON2: u32 = 0x0000_0100;
    const MB_SYSTEMMODAL: u32 = 0x0000_1000;
    const IDYES: i32 = 6;
    const IDNO: i32 = 7;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: Hwnd, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    debug_assert_eq!(prompt.approval_class, ApprovalClass::Soft);
    let body = format!(
        "Cotra requests a local action.\n\nWorkspace: {}\nAction: {}\nTarget: {}\n{}\nDigest: {}\nApproval: SOFT {} (single use, expires in 5 minutes)\n\nApprove this exact operation?",
        prompt.workspace_id,
        prompt.action,
        prompt.target,
        prompt.summary,
        prompt.digest,
        &prompt.nonce[..prompt.nonce.len().min(8)],
    );
    let body = wide(&body);
    let caption = wide("Cotra approval SOFT");
    let result = unsafe {
        MessageBoxW(
            ptr::null_mut(),
            body.as_ptr(),
            caption.as_ptr(),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2 | MB_SYSTEMMODAL,
        )
    };

    match result {
        IDYES => Ok(ApprovalDecision::Approved),
        IDNO => Ok(ApprovalDecision::Denied),
        _ => Err(ApprovalError::unavailable(format!(
            "Windows approval prompt returned unexpected result {result}"
        ))),
    }
}

#[cfg(not(windows))]
fn platform_prompt(prompt: &ApprovalPrompt) -> Result<ApprovalDecision, ApprovalError> {
    debug_assert_eq!(prompt.approval_class, ApprovalClass::Soft);
    let _ = prompt;
    Err(ApprovalError::unavailable(
        "local approval UI is Windows-only in the current Cotra runtime",
    ))
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::{Mutex, OnceLock};

    fn issued() -> &'static Mutex<BTreeMap<String, StoredRecord>> {
        static ISSUED: OnceLock<Mutex<BTreeMap<String, StoredRecord>>> = OnceLock::new();
        ISSUED.get_or_init(|| Mutex::new(BTreeMap::new()))
    }

    fn consumed_set() -> &'static Mutex<BTreeSet<String>> {
        static CONSUMED: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
        CONSUMED.get_or_init(|| Mutex::new(BTreeSet::new()))
    }

    #[derive(Debug, Clone, Copy)]
    pub struct FixedApprovalBroker(pub ApprovalDecision);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TestPresenceResult {
        Verified,
        Denied,
        Cancelled,
        Timeout,
        Unavailable,
        Failed,
        Invalid,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct TestPresenceVerifier {
        pub available: bool,
        pub result: TestPresenceResult,
        pub method: &'static str,
    }

    impl TestPresenceVerifier {
        pub fn verified() -> Self {
            Self {
                available: true,
                result: TestPresenceResult::Verified,
                method: "test-hello",
            }
        }

        pub fn unavailable() -> Self {
            Self {
                available: false,
                result: TestPresenceResult::Unavailable,
                method: "test-hello",
            }
        }

        pub fn denied() -> Self {
            Self {
                available: true,
                result: TestPresenceResult::Denied,
                method: "test-hello",
            }
        }
    }

    impl PresenceVerifier for TestPresenceVerifier {
        fn method(&self) -> &'static str {
            self.method
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn verify(
            &self,
            ctx: &PresenceContext,
            lease: &InputLeaseGuard,
        ) -> Result<PresenceAttestation, PresenceError> {
            if !lease.is_suspended() {
                return Err(PresenceError::invalid(
                    "test presence requires a suspended input lease",
                ));
            }
            if ctx.nonce.trim().is_empty() || ctx.digest.trim().is_empty() {
                return Err(PresenceError::invalid("test presence context is malformed"));
            }
            if !self.available {
                return Err(PresenceError::unavailable(
                    "test presence provider is unavailable",
                ));
            }
            match self.result {
                TestPresenceResult::Verified => Ok(PresenceAttestation {
                    method: self.method.to_owned(),
                    verified_at_ms: ctx.nonce.len() as u64,
                    nonce: ctx.nonce.clone(),
                    digest: ctx.digest.clone(),
                }),
                TestPresenceResult::Denied => {
                    Err(PresenceError::denied("test presence denied the operation"))
                }
                TestPresenceResult::Cancelled => Err(PresenceError::cancelled(
                    "test presence cancelled the operation",
                )),
                TestPresenceResult::Timeout => {
                    Err(PresenceError::timeout("test presence timed out"))
                }
                TestPresenceResult::Unavailable => {
                    Err(PresenceError::unavailable("test presence is unavailable"))
                }
                TestPresenceResult::Failed => Err(PresenceError::failed("test presence failed")),
                TestPresenceResult::Invalid => Err(PresenceError::invalid(
                    "test presence returned invalid data",
                )),
            }
        }
    }

    pub fn broker_with_presence(
        path: std::path::PathBuf,
        verifier: TestPresenceVerifier,
    ) -> LocalApprovalBroker {
        LocalApprovalBroker::with_path_and_verifier(path, Arc::new(verifier))
    }

    fn test_record(prompt: &ApprovalPrompt) -> StoredRecord {
        StoredRecord {
            schema: LEDGER_SCHEMA.to_owned(),
            id: format!("test-{}", prompt.nonce),
            nonce: prompt.nonce.clone(),
            digest: prompt.digest.clone(),
            workspace_id: prompt.workspace_id.clone(),
            policy_revision: prompt.policy_revision.clone(),
            decision: RecordedDecision::Approved,
            approval_class: prompt.approval_class,
            presence_outcome: match prompt.approval_class {
                ApprovalClass::Soft => PresenceOutcome::SoftApproved,
                ApprovalClass::Strong => PresenceOutcome::VerifiedStrong,
            },
            presence_method: String::from("test"),
            epoch: 0,
            is_revoke: false,
            requested_at_ms: prompt.requested_at_ms,
            decided_at_ms: prompt.requested_at_ms,
            expires_at_ms: prompt.expires_at_ms,
            reuse_scope: prompt.reuse_scope.clone(),
            consumed: false,
            prev_checksum: String::from("TEST"),
            checksum: String::from("TEST"),
        }
    }

    impl ApprovalBroker for FixedApprovalBroker {
        fn request_token(&self, prompt: &ApprovalPrompt) -> Result<ApprovedToken, ApprovalError> {
            validate_prompt(prompt)?;
            if prompt.approval_class == ApprovalClass::Strong {
                return Err(ApprovalError::unavailable(
                    "STRONG approval requires platform-mediated presence; a soft test button never satisfies STRONG",
                ));
            }
            match self.0 {
                ApprovalDecision::Approved => {
                    let mut issued = issued().lock().expect("test ledger");
                    if issued.contains_key(&prompt.nonce) {
                        return Err(ApprovalError::invalid(
                            "test approval nonce was already issued",
                        ));
                    }
                    issued.insert(prompt.nonce.clone(), test_record(prompt));
                    Ok(ApprovedToken {
                        record_id: format!("test-{}", prompt.nonce),
                        nonce: prompt.nonce.clone(),
                        digest: prompt.digest.clone(),
                        workspace_id: prompt.workspace_id.clone(),
                        policy_revision: prompt.policy_revision.clone(),
                        expires_at_ms: prompt.expires_at_ms,
                        approval_class: ApprovalClass::Soft,
                        epoch: 0,
                    })
                }
                ApprovalDecision::Denied => Err(ApprovalError::denied(
                    "local user denied the operation in test",
                )),
            }
        }

        fn consume(
            &self,
            token: &ApprovedToken,
            expected: &ConsumeExpectation,
            now_ms: u64,
        ) -> Result<(), ApprovalError> {
            if token.approval_class != expected.approval_class {
                return Err(ApprovalError::stale(
                    "test approval class does not match; STRONG never downgrades to SOFT",
                ));
            }
            let issued = issued().lock().expect("test ledger");
            let record = issued
                .get(&token.nonce)
                .ok_or_else(|| ApprovalError::invalid("test approval token is unknown"))?;
            if record.digest != expected.digest || token.digest != expected.digest {
                return Err(ApprovalError::stale(
                    "test operation digest changed after approval",
                ));
            }
            if record.workspace_id != expected.workspace_id
                || record.policy_revision != expected.policy_revision
            {
                return Err(ApprovalError::stale("test approval scope changed"));
            }
            if now_ms > record.expires_at_ms {
                return Err(ApprovalError::denied("test approval expired"));
            }
            drop(issued);
            let mut consumed = consumed_set().lock().expect("test ledger");
            if !consumed.insert(token.nonce.clone()) {
                return Err(ApprovalError::denied("test approval was already consumed"));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{
        broker_with_presence, FixedApprovalBroker, TestPresenceResult, TestPresenceVerifier,
    };
    use super::*;

    fn prompt_with(digest: &str, now: u64) -> ApprovalPrompt {
        ApprovalPrompt::new_with_clock(
            "default",
            "sg-000019-v1",
            "test action",
            "target",
            "summary",
            digest,
            now,
        )
    }

    fn strong_prompt_with(digest: &str, now: u64) -> ApprovalPrompt {
        ApprovalPrompt::new_strong_with_clock(
            "default",
            "sg-000019-v1",
            "privileged test action",
            "target",
            "summary",
            digest,
            now,
        )
    }

    fn ledger_in(path: &std::path::Path) -> ApprovalLedger {
        ApprovalLedger::load_or_create(path.to_path_buf())
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("cotra-approval-{name}-{suffix}.jsonl"))
    }

    fn token_from(record: &StoredRecord) -> ApprovedToken {
        ApprovedToken {
            record_id: record.id.clone(),
            nonce: record.nonce.clone(),
            digest: record.digest.clone(),
            workspace_id: record.workspace_id.clone(),
            policy_revision: record.policy_revision.clone(),
            expires_at_ms: record.expires_at_ms,
            approval_class: record.approval_class,
            epoch: record.epoch,
        }
    }

    #[test]
    fn nonces_are_fresh_unique_and_bound_to_digest() {
        let first = prompt_with("digest-a", 1_000);
        let second = prompt_with("digest-a", 1_000);
        assert_ne!(first.nonce, second.nonce);
        assert_eq!(first.expires_at_ms, 1_000 + APPROVAL_TTL_MS);
        assert_eq!(first.reuse_scope, SINGLE_OPERATION_SCOPE);
        assert_eq!(first.approval_class, ApprovalClass::Soft);
        let other = prompt_with("digest-b", 1_000);
        assert_ne!(first.nonce, other.nonce);
    }

    #[test]
    fn broker_contract_distinguishes_soft_and_strong_classes() {
        let soft = prompt_with("digest-soft", 1_000);
        let strong = strong_prompt_with("digest-strong", 1_000);
        assert_eq!(soft.approval_class, ApprovalClass::Soft);
        assert_eq!(strong.approval_class, ApprovalClass::Strong);
        assert_ne!(soft.nonce, strong.nonce);
    }

    #[test]
    fn ledger_consumes_once_then_rejects_replay() {
        let path = temp_path("replay");
        let mut ledger = ledger_in(&path);
        let prompt = prompt_with("digest-replay", 5_000);
        let record = ledger
            .record_decision(
                &prompt,
                RecordedDecision::Approved,
                PresenceOutcome::SoftApproved,
                "soft-button",
            )
            .expect("record");
        let token = token_from(&record);
        let expected = ConsumeExpectation::new(record.digest.clone(), "default", "sg-000019-v1");
        ledger.consume(&token, &expected, 6_000).expect("first use");
        let replay = ledger
            .consume(&token, &expected, 6_000)
            .expect_err("replay");
        assert_eq!(replay.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn strong_consumes_once_then_rejects_replay() {
        let path = temp_path("strong-replay");
        let mut ledger = ledger_in(&path);
        let prompt = strong_prompt_with("digest-strong-replay", 5_000);
        let record = ledger
            .record_decision(
                &prompt,
                RecordedDecision::Approved,
                PresenceOutcome::VerifiedStrong,
                "test-hello",
            )
            .expect("record");
        assert_eq!(record.approval_class, ApprovalClass::Strong);
        let token = token_from(&record);
        let expected = ConsumeExpectation::strong(record.digest.clone(), "default", "sg-000019-v1");
        ledger.consume(&token, &expected, 6_000).expect("first use");
        let replay = ledger
            .consume(&token, &expected, 6_000)
            .expect_err("replay");
        assert_eq!(replay.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn soft_button_never_satisfies_strong_class() {
        let broker = FixedApprovalBroker(ApprovalDecision::Approved);
        let strong = strong_prompt_with("digest-strong-soft", 9_000);
        let error = broker.request_token(&strong).expect_err("STRONG via soft");
        assert_eq!(error.code, FailureCode::ApprovalUnavailable);
    }

    #[test]
    fn strong_requires_platform_presence_and_fails_closed_when_unavailable() {
        let path = temp_path("strong-unavailable");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::unavailable());
        let prompt = strong_prompt_with("digest-unavailable", 5_000);
        let error = broker.request_token(&prompt).expect_err("unavailable");
        assert_eq!(error.code, FailureCode::ApprovalUnavailable);
        let history = broker.history(10);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].approval_class, ApprovalClass::Strong);
        assert_eq!(history[0].presence_outcome, PresenceOutcome::Unavailable);
        assert_eq!(history[0].decision, RecordedDecision::Unavailable);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn strong_succeeds_only_with_verified_presence() {
        let path = temp_path("strong-verified");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let prompt = strong_prompt_with("digest-verified", 5_000);
        let token = broker.request_token(&prompt).expect("verified");
        assert_eq!(token.approval_class, ApprovalClass::Strong);
        let expected = ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        broker.consume(&token, &expected, 6_000).expect("consume");
        let history = broker.history(10);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].presence_outcome, PresenceOutcome::VerifiedStrong);
        assert_eq!(history[0].presence_method, "test-hello");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn strong_denial_cancellation_timeout_and_failure_fail_closed() {
        for result in [
            TestPresenceResult::Denied,
            TestPresenceResult::Cancelled,
            TestPresenceResult::Timeout,
            TestPresenceResult::Failed,
            TestPresenceResult::Invalid,
        ] {
            let path = temp_path("strong-negative");
            let broker = broker_with_presence(
                path.clone(),
                TestPresenceVerifier {
                    available: true,
                    result,
                    method: "test-hello",
                },
            );
            let prompt = strong_prompt_with("digest-negative", 5_000);
            let error = broker.request_token(&prompt).expect_err("must fail closed");
            assert!(
                matches!(
                    error.code,
                    FailureCode::ApprovalDenied | FailureCode::ApprovalUnavailable
                ),
                "unexpected code for {result:?}"
            );
            let history = broker.history(10);
            assert_eq!(history.len(), 1);
            assert_ne!(history[0].decision, RecordedDecision::Approved);
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn strong_never_downgrades_to_soft() {
        let path = temp_path("strong-no-downgrade");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let prompt = strong_prompt_with("digest-no-downgrade", 5_000);
        let token = broker.request_token(&prompt).expect("verified");
        let soft_expected =
            ConsumeExpectation::new(prompt.digest.clone(), "default", "sg-000019-v1");
        let error = broker
            .consume(&token, &soft_expected, 6_000)
            .expect_err("downgrade");
        assert_eq!(error.code, FailureCode::TargetStale);
        let strong_expected =
            ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        broker
            .consume(&token, &strong_expected, 6_000)
            .expect("strong consume");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn weak_approval_cannot_satisfy_strong_expectation() {
        let path = temp_path("weak-for-strong");
        let mut ledger = ledger_in(&path);
        let soft_prompt = prompt_with("digest-weak", 5_000);
        let record = ledger
            .record_decision(
                &soft_prompt,
                RecordedDecision::Approved,
                PresenceOutcome::SoftApproved,
                "soft-button",
            )
            .expect("record");
        let token = token_from(&record);
        let strong_expected =
            ConsumeExpectation::strong(record.digest.clone(), "default", "sg-000019-v1");
        let error = ledger
            .consume(&token, &strong_expected, 6_000)
            .expect_err("weak for strong");
        assert_eq!(error.code, FailureCode::TargetStale);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn forged_attestation_without_broker_token_cannot_authorize() {
        let path = temp_path("forged");
        let mut ledger = ledger_in(&path);
        let prompt = strong_prompt_with("digest-forged", 5_000);
        let forged = ApprovedToken {
            record_id: "apr-forged".to_owned(),
            nonce: prompt.nonce.clone(),
            digest: prompt.digest.clone(),
            workspace_id: prompt.workspace_id.clone(),
            policy_revision: prompt.policy_revision.clone(),
            expires_at_ms: prompt.expires_at_ms,
            approval_class: ApprovalClass::Strong,
            epoch: 0,
        };
        let expected = ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        let error = ledger
            .consume(&forged, &expected, 6_000)
            .expect_err("forged");
        assert_eq!(error.code, FailureCode::InvalidRequest);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn ledger_rejects_expiry_mismatch_and_drift() {
        let path = temp_path("drift");
        let mut ledger = ledger_in(&path);
        let prompt = prompt_with("digest-drift", 5_000);
        let record = ledger
            .record_decision(
                &prompt,
                RecordedDecision::Approved,
                PresenceOutcome::SoftApproved,
                "soft-button",
            )
            .expect("record");
        let token = token_from(&record);
        let expected = ConsumeExpectation::new(record.digest.clone(), "default", "sg-000019-v1");
        let expired = ledger
            .consume(&token, &expected, record.expires_at_ms + 1)
            .expect_err("expired");
        assert_eq!(expired.code, FailureCode::ApprovalDenied);
        let changed = ConsumeExpectation::new("other-digest", "default", "sg-000019-v1");
        let mismatch = ledger
            .consume(&token, &changed, 6_000)
            .expect_err("mismatch");
        assert_eq!(mismatch.code, FailureCode::TargetStale);
        let moved =
            ConsumeExpectation::new(record.digest.clone(), "other-workspace", "sg-000019-v1");
        let drift = ledger.consume(&token, &moved, 6_000).expect_err("drift");
        assert_eq!(drift.code, FailureCode::TargetStale);
        let stale_policy =
            ConsumeExpectation::new(record.digest.clone(), "default", "sg-000018-v1");
        let revision = ledger
            .consume(&token, &stale_policy, 6_000)
            .expect_err("revision drift");
        assert_eq!(revision.code, FailureCode::TargetStale);
        let class_drift =
            ConsumeExpectation::strong(record.digest.clone(), "default", "sg-000019-v1");
        let class_error = ledger
            .consume(&token, &class_drift, 6_000)
            .expect_err("class drift");
        assert_eq!(class_error.code, FailureCode::TargetStale);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn denied_and_unavailable_records_never_authorize() {
        let path = temp_path("denied");
        let mut ledger = ledger_in(&path);
        let prompt = prompt_with("digest-denied", 5_000);
        let record = ledger
            .record_decision(
                &prompt,
                RecordedDecision::Denied,
                PresenceOutcome::Denied,
                "soft-button",
            )
            .expect("record");
        let token = token_from(&record);
        let expected = ConsumeExpectation::new(record.digest.clone(), "default", "sg-000019-v1");
        let error = ledger
            .consume(&token, &expected, 6_000)
            .expect_err("denied");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn strong_denied_records_never_authorize() {
        let path = temp_path("strong-denied");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::denied());
        let prompt = strong_prompt_with("digest-strong-denied", 5_000);
        let error = broker.request_token(&prompt).expect_err("denied");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        let history = broker.history(10);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].presence_outcome, PresenceOutcome::Denied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn history_records_class_and_presence_without_secrets() {
        let path = temp_path("history-class");
        let soft_broker = broker_with_presence(path.clone(), TestPresenceVerifier::unavailable());
        let soft_prompt = prompt_with("digest-soft-history", 5_000);
        let _ = soft_broker.ledger.lock().map(|mut ledger| {
            ledger
                .record_decision(
                    &soft_prompt,
                    RecordedDecision::Approved,
                    PresenceOutcome::SoftApproved,
                    "soft-button",
                )
                .expect("soft record")
        });
        let strong_broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        drop(strong_broker);
        let mut ledger = ledger_in(&path);
        let strong_prompt = strong_prompt_with("digest-strong-history", 5_000);
        ledger
            .record_decision(
                &strong_prompt,
                RecordedDecision::Approved,
                PresenceOutcome::VerifiedStrong,
                "test-hello",
            )
            .expect("strong record");
        let full = ledger.history(200);
        assert!(full.len() >= 2);
        let rendered = format!("{full:?}");
        assert!(!rendered.contains("summary"));
        assert!(!rendered.contains("target"));
        assert!(!rendered.contains("biometric"));
        assert!(!rendered.contains("secret"));
        assert!(rendered.contains("Soft") || rendered.contains("Strong"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn history_is_redacted_bounded_and_tamper_evident() {
        let path = temp_path("history");
        let mut ledger = ledger_in(&path);
        for digest in ["digest-one", "digest-two", "digest-three"] {
            let prompt = prompt_with(digest, 5_000);
            ledger
                .record_decision(
                    &prompt,
                    RecordedDecision::Approved,
                    PresenceOutcome::SoftApproved,
                    "soft-button",
                )
                .expect("record");
        }
        let full = ledger.history(200);
        assert_eq!(full.len(), 3);
        let rendered = format!("{full:?}");
        assert!(!rendered.contains("summary"));
        assert!(!rendered.contains("target"));
        let bounded = ledger.history(2);
        assert_eq!(bounded.len(), 2);
        drop(ledger);
        let mut text = std::fs::read_to_string(&path).expect("read ledger");
        text = text.replace("digest-two", "digest-tampered");
        std::fs::write(&path, text).expect("tamper ledger");
        let reloaded = ledger_in(&path);
        assert!(reloaded.records.len() < 3);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn forged_history_is_rejected() {
        let path = temp_path("history-forge");
        let mut ledger = ledger_in(&path);
        let prompt = strong_prompt_with("digest-forge", 5_000);
        ledger
            .record_decision(
                &prompt,
                RecordedDecision::Approved,
                PresenceOutcome::VerifiedStrong,
                "test-hello",
            )
            .expect("record");
        drop(ledger);
        let mut text = std::fs::read_to_string(&path).expect("read");
        text = text.replace("verified-strong", "soft-approved");
        std::fs::write(&path, text).expect("tamper");
        let reloaded = ledger_in(&path);
        assert!(reloaded.records.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn fixed_broker_enforces_one_shot_without_real_ui() {
        let broker = FixedApprovalBroker(ApprovalDecision::Approved);
        let prompt = prompt_with("digest-fixed", 9_000);
        let token = broker.request_token(&prompt).expect("token");
        let expected = ConsumeExpectation::new("digest-fixed", "default", "sg-000019-v1");
        broker.consume(&token, &expected, 9_500).expect("first use");
        let replay = broker
            .consume(&token, &expected, 9_500)
            .expect_err("replay");
        assert_eq!(replay.code, FailureCode::ApprovalDenied);
        let denied = FixedApprovalBroker(ApprovalDecision::Denied);
        let error = denied.request_token(&prompt).expect_err("denied");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
    }

    #[test]
    fn duplicate_consumption_is_replay_for_strong() {
        let path = temp_path("strong-double");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let prompt = strong_prompt_with("digest-double", 5_000);
        let token = broker.request_token(&prompt).expect("token");
        let expected = ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        broker.consume(&token, &expected, 6_000).expect("first");
        let replay = broker
            .consume(&token, &expected, 6_000)
            .expect_err("replay");
        assert_eq!(replay.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn expired_strong_approval_fails_closed() {
        let path = temp_path("strong-expired");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let prompt = strong_prompt_with("digest-expired", 5_000);
        let token = broker.request_token(&prompt).expect("token");
        let expected = ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        let error = broker
            .consume(&token, &expected, prompt.expires_at_ms + 1)
            .expect_err("expired");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn wrong_nonce_workspace_and_revision_fail_closed_for_strong() {
        let path = temp_path("strong-drift");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let prompt = strong_prompt_with("digest-drift", 5_000);
        let token = broker.request_token(&prompt).expect("token");
        let wrong_digest = ConsumeExpectation::strong("other-digest", "default", "sg-000019-v1");
        assert_eq!(
            broker
                .consume(&token, &wrong_digest, 6_000)
                .expect_err("digest")
                .code,
            FailureCode::TargetStale
        );
        let wrong_workspace =
            ConsumeExpectation::strong(prompt.digest.clone(), "other", "sg-000019-v1");
        let path2 = temp_path("strong-drift-2");
        let broker2 = broker_with_presence(path2.clone(), TestPresenceVerifier::verified());
        let prompt2 = strong_prompt_with("digest-drift", 5_000);
        let token2 = broker2.request_token(&prompt2).expect("token");
        assert_eq!(
            broker2
                .consume(&token2, &wrong_workspace, 6_000)
                .expect_err("workspace")
                .code,
            FailureCode::TargetStale
        );
        let wrong_revision =
            ConsumeExpectation::strong(prompt2.digest.clone(), "default", "sg-000018-v1");
        let path3 = temp_path("strong-drift-3");
        let broker3 = broker_with_presence(path3.clone(), TestPresenceVerifier::verified());
        let prompt3 = strong_prompt_with("digest-drift", 5_000);
        let token3 = broker3.request_token(&prompt3).expect("token");
        assert_eq!(
            broker3
                .consume(&token3, &wrong_revision, 6_000)
                .expect_err("revision")
                .code,
            FailureCode::TargetStale
        );
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path2);
        let _ = std::fs::remove_file(path3);
    }

    #[cfg(windows)]
    #[test]
    fn windows_real_presence_path_reports_availability_explicitly() {
        let verifier = WindowsHelloPresenceVerifier::new();
        let available = verifier.is_available();
        let method = verifier.method();
        assert_eq!(method, "windows-hello");
        if available {
            assert!(available);
        } else {
            let ctx = PresenceContext::new("nonce-probe", "digest-probe", "default", "probe");
            let lease = InputLeaseGuard::suspend_for_presence();
            let error = verifier
                .verify(&ctx, &lease)
                .expect_err("unavailable fails closed");
            assert_eq!(error.code, FailureCode::ApprovalUnavailable);
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_default_presence_is_unavailable_fail_closed() {
        let verifier = NoPresenceVerifier;
        assert!(!verifier.is_available());
        let ctx = PresenceContext::new("nonce-probe", "digest-probe", "default", "probe");
        let lease = InputLeaseGuard::suspend_for_presence();
        let error = verifier.verify(&ctx, &lease).expect_err("unavailable");
        assert_eq!(error.code, FailureCode::ApprovalUnavailable);
    }

    #[test]
    fn emergency_revoke_requires_strong_and_invalidates_prior_tokens() {
        let path = temp_path("revoke");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let prompt = strong_prompt_with("digest-before-revoke", 5_000);
        let token = broker.request_token(&prompt).expect("token");
        assert_eq!(token.epoch, 0);
        let revoke_prompt = ApprovalPrompt::new_strong_with_clock(
            "default",
            "sg-000019-v1",
            "emergency revoke",
            "revoke",
            "revoke all pending approvals",
            "digest-revoke",
            5_500,
        );
        let epoch = broker.emergency_revoke(&revoke_prompt).expect("revoke");
        assert_eq!(epoch, 1);
        let expected = ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        let error = broker
            .consume(&token, &expected, 6_000)
            .expect_err("revoked");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        let history = broker.history(10);
        assert!(history.iter().any(|record| record.is_revoke));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn emergency_revoke_with_soft_class_fails_closed() {
        let path = temp_path("revoke-soft");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let soft = prompt_with("digest-soft-revoke", 5_000);
        let error = broker.emergency_revoke(&soft).expect_err("soft revoke");
        assert_eq!(error.code, FailureCode::InvalidRequest);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn emergency_revoke_without_presence_fails_closed() {
        let path = temp_path("revoke-unavailable");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::unavailable());
        let revoke_prompt = ApprovalPrompt::new_strong_with_clock(
            "default",
            "sg-000019-v1",
            "emergency revoke",
            "revoke",
            "revoke all pending approvals",
            "digest-revoke",
            5_500,
        );
        let error = broker
            .emergency_revoke(&revoke_prompt)
            .expect_err("unavailable");
        assert_eq!(error.code, FailureCode::ApprovalUnavailable);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn revoked_tokens_fail_closed_after_reload() {
        let path = temp_path("revoke-reload");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let prompt = strong_prompt_with("digest-reload", 5_000);
        let token = broker.request_token(&prompt).expect("token");
        let revoke_prompt = ApprovalPrompt::new_strong_with_clock(
            "default",
            "sg-000019-v1",
            "emergency revoke",
            "revoke",
            "revoke all pending approvals",
            "digest-revoke",
            5_500,
        );
        broker.emergency_revoke(&revoke_prompt).expect("revoke");
        drop(broker);
        let reloaded = ApprovalLedger::load_or_create(path.clone());
        assert_eq!(reloaded.current_epoch(), 1);
        let expected = ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        let mut reloaded = reloaded;
        let error = reloaded
            .consume(&token, &expected, 6_000)
            .expect_err("revoked after reload");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn approvals_after_revoke_use_new_epoch() {
        let path = temp_path("revoke-new-epoch");
        let broker = broker_with_presence(path.clone(), TestPresenceVerifier::verified());
        let revoke_prompt = ApprovalPrompt::new_strong_with_clock(
            "default",
            "sg-000019-v1",
            "emergency revoke",
            "revoke",
            "revoke all pending approvals",
            "digest-revoke",
            5_000,
        );
        let epoch = broker.emergency_revoke(&revoke_prompt).expect("revoke");
        assert_eq!(epoch, 1);
        let prompt = strong_prompt_with("digest-after", 6_000);
        let token = broker.request_token(&prompt).expect("token");
        assert_eq!(token.epoch, 1);
        let expected = ConsumeExpectation::strong(prompt.digest.clone(), "default", "sg-000019-v1");
        broker.consume(&token, &expected, 6_500).expect("consume");
        let _ = std::fs::remove_file(path);
    }
}
