use cotra_contracts::FailureCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub const APPROVAL_TTL_MS: u64 = 5 * 60 * 1_000;
pub const SINGLE_OPERATION_SCOPE: &str = "single-operation";
const LEDGER_SCHEMA: &str = "cotra-approval-ledger-v1";

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedToken {
    pub record_id: String,
    pub nonce: String,
    pub digest: String,
    pub workspace_id: String,
    pub policy_revision: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeExpectation {
    pub digest: String,
    pub workspace_id: String,
    pub policy_revision: String,
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
        }
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

    fn request(&self, prompt: &ApprovalPrompt) -> Result<ApprovalDecision, ApprovalError> {
        let token = self.request_token(prompt)?;
        let expected = ConsumeExpectation::new(
            prompt.digest.clone(),
            prompt.workspace_id.clone(),
            prompt.policy_revision.clone(),
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
}

impl LocalApprovalBroker {
    pub fn new() -> Self {
        Self::with_path(default_approval_history_path())
    }

    pub fn with_path(path: impl Into<std::path::PathBuf>) -> Self {
        let ledger = ApprovalLedger::load_or_create(path.into());
        Self {
            ledger: Mutex::new(ledger),
        }
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
        let decision = platform_prompt(prompt);
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| ApprovalError::unavailable("approval ledger is unavailable"))?;
        match decision {
            Ok(ApprovalDecision::Approved) => {
                let record = ledger.record_decision(prompt, RecordedDecision::Approved)?;
                Ok(ApprovedToken {
                    record_id: record.id.clone(),
                    nonce: record.nonce.clone(),
                    digest: record.digest.clone(),
                    workspace_id: record.workspace_id.clone(),
                    policy_revision: record.policy_revision.clone(),
                    expires_at_ms: record.expires_at_ms,
                })
            }
            Ok(ApprovalDecision::Denied) => {
                let _ = ledger.record_decision(prompt, RecordedDecision::Denied);
                Err(ApprovalError::denied("local user denied the operation"))
            }
            Err(error) => {
                let _ = ledger.record_decision(prompt, RecordedDecision::Unavailable);
                Err(error)
            }
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
}

impl ApprovalLedger {
    fn load_or_create(path: std::path::PathBuf) -> Self {
        let mut ledger = Self {
            path,
            records: BTreeMap::new(),
            nonces: HashSet::new(),
            consumed: HashSet::new(),
            tip: String::from("GENESIS"),
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
    ) -> Result<StoredRecord, ApprovalError> {
        if !self.nonces.insert(prompt.nonce.clone()) {
            return Err(ApprovalError::invalid(
                "approval nonce was already issued; retry with a fresh prompt",
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
        if record.nonce != token.nonce {
            return Err(ApprovalError::stale(
                "approval token nonce does not match the recorded approval",
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
            let _ = self.append_consumed_marker(&updated);
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

    let body = format!(
        "Cotra requests a local action.\n\nWorkspace: {}\nAction: {}\nTarget: {}\n{}\nDigest: {}\nApproval: {} (single use, expires in 5 minutes)\n\nApprove this exact operation?",
        prompt.workspace_id,
        prompt.action,
        prompt.target,
        prompt.summary,
        prompt.digest,
        &prompt.nonce[..prompt.nonce.len().min(8)],
    );
    let body = wide(&body);
    let caption = wide("Cotra approval");
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
fn platform_prompt(_prompt: &ApprovalPrompt) -> Result<ApprovalDecision, ApprovalError> {
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

    fn test_record(prompt: &ApprovalPrompt) -> StoredRecord {
        StoredRecord {
            schema: LEDGER_SCHEMA.to_owned(),
            id: format!("test-{}", prompt.nonce),
            nonce: prompt.nonce.clone(),
            digest: prompt.digest.clone(),
            workspace_id: prompt.workspace_id.clone(),
            policy_revision: prompt.policy_revision.clone(),
            decision: RecordedDecision::Approved,
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
    use super::test_support::FixedApprovalBroker;
    use super::*;

    fn prompt_with(digest: &str, now: u64) -> ApprovalPrompt {
        ApprovalPrompt::new_with_clock(
            "default",
            "sg-000018-v1",
            "test action",
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

    #[test]
    fn nonces_are_fresh_unique_and_bound_to_digest() {
        let first = prompt_with("digest-a", 1_000);
        let second = prompt_with("digest-a", 1_000);
        assert_ne!(first.nonce, second.nonce);
        assert_eq!(first.expires_at_ms, 1_000 + APPROVAL_TTL_MS);
        assert_eq!(first.reuse_scope, SINGLE_OPERATION_SCOPE);
        let other = prompt_with("digest-b", 1_000);
        assert_ne!(first.nonce, other.nonce);
    }

    #[test]
    fn ledger_consumes_once_then_rejects_replay() {
        let path = temp_path("replay");
        let mut ledger = ledger_in(&path);
        let prompt = prompt_with("digest-replay", 5_000);
        let record = ledger
            .record_decision(&prompt, RecordedDecision::Approved)
            .expect("record");
        let token = ApprovedToken {
            record_id: record.id.clone(),
            nonce: record.nonce.clone(),
            digest: record.digest.clone(),
            workspace_id: record.workspace_id.clone(),
            policy_revision: record.policy_revision.clone(),
            expires_at_ms: record.expires_at_ms,
        };
        let expected = ConsumeExpectation::new(record.digest.clone(), "default", "sg-000018-v1");
        ledger.consume(&token, &expected, 6_000).expect("first use");
        let replay = ledger
            .consume(&token, &expected, 6_000)
            .expect_err("replay");
        assert_eq!(replay.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn ledger_rejects_expiry_mismatch_and_drift() {
        let path = temp_path("drift");
        let mut ledger = ledger_in(&path);
        let prompt = prompt_with("digest-drift", 5_000);
        let record = ledger
            .record_decision(&prompt, RecordedDecision::Approved)
            .expect("record");
        let token = ApprovedToken {
            record_id: record.id.clone(),
            nonce: record.nonce.clone(),
            digest: record.digest.clone(),
            workspace_id: record.workspace_id.clone(),
            policy_revision: record.policy_revision.clone(),
            expires_at_ms: record.expires_at_ms,
        };
        let expected = ConsumeExpectation::new(record.digest.clone(), "default", "sg-000018-v1");
        let expired = ledger
            .consume(&token, &expected, record.expires_at_ms + 1)
            .expect_err("expired");
        assert_eq!(expired.code, FailureCode::ApprovalDenied);
        let changed = ConsumeExpectation::new("other-digest", "default", "sg-000018-v1");
        let mismatch = ledger
            .consume(&token, &changed, 6_000)
            .expect_err("mismatch");
        assert_eq!(mismatch.code, FailureCode::TargetStale);
        let moved =
            ConsumeExpectation::new(record.digest.clone(), "other-workspace", "sg-000018-v1");
        let drift = ledger.consume(&token, &moved, 6_000).expect_err("drift");
        assert_eq!(drift.code, FailureCode::TargetStale);
        let stale_policy =
            ConsumeExpectation::new(record.digest.clone(), "default", "sg-000017-v1");
        let revision = ledger
            .consume(&token, &stale_policy, 6_000)
            .expect_err("revision drift");
        assert_eq!(revision.code, FailureCode::TargetStale);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn denied_and_unavailable_records_never_authorize() {
        let path = temp_path("denied");
        let mut ledger = ledger_in(&path);
        let prompt = prompt_with("digest-denied", 5_000);
        let record = ledger
            .record_decision(&prompt, RecordedDecision::Denied)
            .expect("record");
        let token = ApprovedToken {
            record_id: record.id.clone(),
            nonce: record.nonce.clone(),
            digest: record.digest.clone(),
            workspace_id: record.workspace_id.clone(),
            policy_revision: record.policy_revision.clone(),
            expires_at_ms: record.expires_at_ms,
        };
        let expected = ConsumeExpectation::new(record.digest.clone(), "default", "sg-000018-v1");
        let error = ledger
            .consume(&token, &expected, 6_000)
            .expect_err("denied");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn history_is_redacted_bounded_and_tamper_evident() {
        let path = temp_path("history");
        let mut ledger = ledger_in(&path);
        for digest in ["digest-one", "digest-two", "digest-three"] {
            let prompt = prompt_with(digest, 5_000);
            ledger
                .record_decision(&prompt, RecordedDecision::Approved)
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
    fn fixed_broker_enforces_one_shot_without_real_ui() {
        let broker = FixedApprovalBroker(ApprovalDecision::Approved);
        let prompt = prompt_with("digest-fixed", 9_000);
        let token = broker.request_token(&prompt).expect("token");
        let expected = ConsumeExpectation::new("digest-fixed", "default", "sg-000018-v1");
        broker.consume(&token, &expected, 9_500).expect("first use");
        let replay = broker
            .consume(&token, &expected, 9_500)
            .expect_err("replay");
        assert_eq!(replay.code, FailureCode::ApprovalDenied);
        let denied = FixedApprovalBroker(ApprovalDecision::Denied);
        let error = denied.request_token(&prompt).expect_err("denied");
        assert_eq!(error.code, FailureCode::ApprovalDenied);
    }
}
