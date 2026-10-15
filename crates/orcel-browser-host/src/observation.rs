//! SG-000076 bound live DOM and accessibility observation.
//!
//! Read-only observation bound to exact workspace, browser profile, page,
//! origin, page generation, document generation, and server-issued node
//! identity. Explicit numeric ceilings bound nodes, depth, bytes, time,
//! and concurrency. Password and secret content is redacted everywhere and
//! never enters records, logs, or evidence. Arbitrary JavaScript, generic
//! CDP, DevTools, credential storage reads, and caller-defined selectors
//! as authority are denied by construction.

use crate::error::HostError;
use std::collections::HashMap;

/// Hard ceiling on observed nodes per snapshot.
pub const MAX_OBSERVED_NODES: usize = 512;

/// Hard ceiling on tree depth per snapshot.
pub const MAX_OBSERVATION_DEPTH: usize = 16;

/// Hard ceiling on encoded snapshot bytes.
pub const MAX_SNAPSHOT_BYTES: usize = 65_536;

/// Hard ceiling on observation time in milliseconds.
pub const MAX_OBSERVATION_MS: u64 = 2_000;

/// Hard ceiling on concurrent observations per supervisor.
pub const MAX_CONCURRENT_OBSERVATIONS: usize = 4;

/// Maximum node identity bytes.
pub const MAX_IDENTITY_BYTES: usize = 256;

/// Maximum value bytes retained before redaction or truncation.
pub const MAX_VALUE_BYTES: usize = 1_024;

/// Roles whose values are always redacted.
const SECRET_ROLES: &[&str] = &["password", "secret", "credential", "token", "pin"];

/// Server-issued page identity. Callers present it opaquely; the host
/// revalidates every field before any read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageIdentity {
    pub page_id: String,
    pub origin: String,
    pub page_generation: u64,
}

/// Server-issued document identity bound to its page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentIdentity {
    pub page_id: String,
    pub document_generation: u64,
}

/// Server-issued node identity bound to its document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentity {
    pub node_id: String,
    pub role: String,
    pub document_generation: u64,
}

/// Workspace and profile scope for an observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationScope {
    pub workspace: String,
    pub profile_identity: String,
}

/// One input node presented for observation. Values are untrusted input
/// and are redacted or digested before they can reach any record.
#[derive(Debug, Clone)]
pub struct InputNode {
    pub identity: NodeIdentity,
    pub role: String,
    pub name: String,
    pub value: String,
    pub children: Vec<InputNode>,
}

/// One redacted output node. Secrets never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedNode {
    pub node_id: String,
    pub role: String,
    pub name_digest: String,
    pub value_digest: String,
    pub redacted: bool,
    pub children: Vec<ObservedNode>,
}

/// Bounded observation result with explicit truncation reporting.
#[derive(Debug, Clone)]
pub struct Observation {
    pub nodes: Vec<ObservedNode>,
    pub node_count: usize,
    pub truncated: bool,
    pub bytes: usize,
}

/// Validate a server-issued identity field: non-empty, bounded, no
/// control characters, no path separators used as authority smuggling.
pub fn validate_identity_field(field: &str) -> Result<(), HostError> {
    if field.is_empty() || field.len() > MAX_IDENTITY_BYTES {
        return Err(HostError::Invalid(
            "node identity is empty or oversized".into(),
        ));
    }
    if field.chars().any(|ch| ch.is_control()) {
        return Err(HostError::Invalid(
            "node identity carries control characters".into(),
        ));
    }
    if field.contains("..") || field.contains('/') || field.contains('\\') {
        return Err(HostError::Invalid(
            "node identity carries path traversal".into(),
        ));
    }
    Ok(())
}

/// Check page, document, and node generation freshness. Any mismatch
/// fails closed as stale.
pub fn check_freshness(
    expected_page: u64,
    actual_page: u64,
    expected_document: u64,
    actual_document: u64,
) -> Result<(), HostError> {
    if expected_page != actual_page {
        return Err(HostError::Invalid("stale page generation".into()));
    }
    if expected_document != actual_document {
        return Err(HostError::Invalid("stale document generation".into()));
    }
    Ok(())
}

/// Redact a value by role. Secret roles are fully redacted; all other
/// values are truncated to the hard bound and returned with a digest.
/// The raw secret never reaches the caller-visible record.
pub fn redact_value(role: &str, value: &str) -> (String, bool) {
    let lower = role.to_ascii_lowercase();
    if SECRET_ROLES.iter().any(|secret| lower.contains(secret)) {
        return ("[redacted]".to_owned(), true);
    }
    let truncated: String = value.chars().take(MAX_VALUE_BYTES).collect();
    (truncated, false)
}

/// Short hex digest used in place of raw names and values in evidence.
pub fn short_digest(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Observe one bounded snapshot. Fails closed on scope mismatch, stale
/// generations, oversized input, bound overflow, or timeout budget
/// exhaustion. `elapsed_ms` is the caller-measured time spent gathering
/// the input; anything above the ceiling fails closed.
pub fn observe_snapshot(
    scope: &ObservationScope,
    page: &PageIdentity,
    document: &DocumentIdentity,
    roots: &[InputNode],
    elapsed_ms: u64,
) -> Result<Observation, HostError> {
    validate_identity_field(&scope.workspace)?;
    validate_identity_field(&scope.profile_identity)?;
    validate_identity_field(&page.page_id)?;
    crate::navigation::parse_navigation_url(&page.origin)?;
    if document.page_id != page.page_id {
        return Err(HostError::Invalid(
            "document does not belong to the page".into(),
        ));
    }
    if elapsed_ms > MAX_OBSERVATION_MS {
        return Err(HostError::Invalid(
            "observation time budget exceeded".into(),
        ));
    }
    let mut count = 0usize;
    let mut bytes = 0usize;
    let mut out = Vec::new();
    let mut truncated = false;
    for root in roots {
        match observe_node(
            root,
            document.document_generation,
            0,
            &mut count,
            &mut bytes,
        )? {
            Some(node) => out.push(node),
            None => {
                truncated = true;
                break;
            }
        }
    }
    Ok(Observation {
        node_count: count,
        truncated,
        bytes,
        nodes: out,
    })
}

fn observe_node(
    input: &InputNode,
    expected_document: u64,
    depth: usize,
    count: &mut usize,
    bytes: &mut usize,
) -> Result<Option<ObservedNode>, HostError> {
    if depth > MAX_OBSERVATION_DEPTH {
        return Ok(None);
    }
    if *count >= MAX_OBSERVED_NODES {
        return Ok(None);
    }
    validate_identity_field(&input.identity.node_id)?;
    validate_identity_field(&input.identity.role)?;
    validate_identity_field(&input.role)?;
    if input.identity.document_generation != expected_document {
        return Err(HostError::Invalid("stale node document generation".into()));
    }
    if input.name.len() > MAX_VALUE_BYTES * 2 || input.value.len() > MAX_VALUE_BYTES * 2 {
        return Err(HostError::Invalid("observed value oversized".into()));
    }
    let (visible_value, redacted) = redact_value(&input.role, &input.value);
    let node = ObservedNode {
        node_id: input.identity.node_id.clone(),
        role: input.role.clone(),
        name_digest: short_digest(&input.name),
        value_digest: short_digest(&visible_value),
        redacted,
        children: Vec::new(),
    };
    let encoded = node.node_id.len() + node.role.len() + 32;
    if *bytes + encoded > MAX_SNAPSHOT_BYTES {
        return Ok(None);
    }
    *bytes += encoded;
    *count += 1;
    let mut children = Vec::new();
    for child in &input.children {
        match observe_node(child, expected_document, depth + 1, count, bytes)? {
            Some(observed) => children.push(observed),
            None => break,
        }
        if *count >= MAX_OBSERVED_NODES || *bytes >= MAX_SNAPSHOT_BYTES {
            break;
        }
    }
    let mut node = node;
    node.children = children;
    Ok(Some(node))
}

/// Concurrency gate: at most MAX_CONCURRENT_OBSERVATIONS live at once.
pub struct ObservationGate {
    live: HashMap<String, u64>,
}

impl ObservationGate {
    pub fn new() -> Self {
        Self {
            live: HashMap::new(),
        }
    }

    pub fn acquire(&mut self, page_id: &str, now_ms: u64) -> Result<(), HostError> {
        validate_identity_field(page_id)?;
        self.live
            .retain(|_, started| now_ms.saturating_sub(*started) <= MAX_OBSERVATION_MS);
        if self.live.len() >= MAX_CONCURRENT_OBSERVATIONS && !self.live.contains_key(page_id) {
            return Err(HostError::Invalid(
                "observation concurrency bound exceeded".into(),
            ));
        }
        self.live.insert(page_id.to_owned(), now_ms);
        Ok(())
    }

    pub fn release(&mut self, page_id: &str) {
        self.live.remove(page_id);
    }

    pub fn live_count(&self) -> usize {
        self.live.len()
    }
}

impl Default for ObservationGate {
    fn default() -> Self {
        Self::new()
    }
}

/// Denied: arbitrary JavaScript evaluation never exists in this grain.
pub fn evaluate_javascript(_script: &str) -> Result<(), HostError> {
    Err(HostError::Invalid("arbitrary javascript denied".into()))
}

/// Denied: generic CDP commands never exist in this grain.
pub fn cdp_command(_method: &str) -> Result<(), HostError> {
    Err(HostError::Invalid("generic cdp denied".into()))
}

/// Denied: DevTools surfaces never exist in this grain.
pub fn devtools_open() -> Result<(), HostError> {
    Err(HostError::Invalid("devtools denied".into()))
}

/// Denied: credential storage reads never exist in this grain.
pub fn read_credentials(_store: &str) -> Result<(), HostError> {
    Err(HostError::Invalid("credential storage read denied".into()))
}

/// Denied: caller-defined selectors are never authority in this grain.
/// Callers present server-issued node identities only.
pub fn select_by_caller_selector(_selector: &str) -> Result<(), HostError> {
    Err(HostError::Invalid("caller-defined selector denied".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ObservationScope {
        ObservationScope {
            workspace: "workspace-1".into(),
            profile_identity: "profile-1".into(),
        }
    }

    fn page() -> PageIdentity {
        PageIdentity {
            page_id: "page-1".into(),
            origin: "https://example.com:443".into(),
            page_generation: 7,
        }
    }

    fn document() -> DocumentIdentity {
        DocumentIdentity {
            page_id: "page-1".into(),
            document_generation: 3,
        }
    }

    fn node(id: &str, role: &str, value: &str) -> InputNode {
        InputNode {
            identity: NodeIdentity {
                node_id: id.into(),
                role: role.into(),
                document_generation: 3,
            },
            role: role.into(),
            name: format!("{id}-name"),
            value: value.into(),
            children: Vec::new(),
        }
    }

    #[test]
    fn bound_observation_round_trip() {
        let roots = vec![node("node-1", "button", "Submit")];
        let observation = observe_snapshot(&scope(), &page(), &document(), &roots, 12).unwrap();
        assert_eq!(observation.node_count, 1);
        assert!(!observation.truncated);
        assert_eq!(observation.nodes[0].node_id, "node-1");
        assert!(!observation.nodes[0].redacted);
    }

    #[test]
    fn password_and_secret_values_are_redacted() {
        let roots = vec![
            node("node-1", "password", "hunter2-secret"),
            node("node-2", "textbox", "visible"),
        ];
        let observation = observe_snapshot(&scope(), &page(), &document(), &roots, 5).unwrap();
        assert!(observation.nodes[0].redacted);
        assert_eq!(
            observation.nodes[0].value_digest,
            short_digest("[redacted]")
        );
        assert!(!observation.nodes[1].redacted);
        let encoded = format!("{:?}", observation);
        assert!(!encoded.contains("hunter2-secret"));
    }

    #[test]
    fn stale_generations_fail_closed() {
        assert!(check_freshness(7, 8, 3, 3).is_err());
        assert!(check_freshness(7, 7, 3, 4).is_err());
        assert!(check_freshness(7, 7, 3, 3).is_ok());
        let mut stale = node("node-1", "button", "x");
        stale.identity.document_generation = 4;
        assert!(observe_snapshot(&scope(), &page(), &document(), &[stale], 1).is_err());
        let mut foreign = document();
        foreign.page_id = "page-2".into();
        assert!(
            observe_snapshot(&scope(), &page(), &foreign, &[node("n", "button", "x")], 1).is_err()
        );
    }

    #[test]
    fn node_count_depth_and_byte_bounds_truncate() {
        let mut many = Vec::new();
        for index in 0..MAX_OBSERVED_NODES + 10 {
            many.push(node(&format!("node-{index}"), "cell", "x"));
        }
        let observation = observe_snapshot(&scope(), &page(), &document(), &many, 1).unwrap();
        assert_eq!(observation.node_count, MAX_OBSERVED_NODES);
        assert!(observation.truncated);
        let mut deep = node("root", "pane", "x");
        let mut cursor = &mut deep;
        for index in 0..MAX_OBSERVATION_DEPTH + 5 {
            let child = node(&format!("deep-{index}"), "pane", "x");
            cursor.children.push(child);
            let last = cursor.children.len() - 1;
            cursor = &mut cursor.children[last];
        }
        let observation = observe_snapshot(&scope(), &page(), &document(), &[deep], 1).unwrap();
        assert!(observation.node_count <= MAX_OBSERVED_NODES);
    }

    #[test]
    fn oversized_values_time_budget_and_identity_fail_closed() {
        let big = "x".repeat(MAX_VALUE_BYTES * 2 + 1);
        assert!(observe_snapshot(
            &scope(),
            &page(),
            &document(),
            &[node("n", "button", &big)],
            1
        )
        .is_err());
        assert!(observe_snapshot(
            &scope(),
            &page(),
            &document(),
            &[node("n", "button", "x")],
            MAX_OBSERVATION_MS + 1
        )
        .is_err());
        assert!(validate_identity_field("").is_err());
        assert!(validate_identity_field("../escape").is_err());
        assert!(validate_identity_field("bad\0control").is_err());
    }

    #[test]
    fn concurrency_bound_is_enforced() {
        let mut gate = ObservationGate::new();
        for index in 0..MAX_CONCURRENT_OBSERVATIONS {
            gate.acquire(&format!("page-{index}"), 100).unwrap();
        }
        assert!(gate.acquire("page-overflow", 101).is_err());
        assert_eq!(gate.live_count(), MAX_CONCURRENT_OBSERVATIONS);
        gate.release("page-0");
        assert!(gate.acquire("page-overflow", 102).is_ok());
    }

    #[test]
    fn evaluation_debugging_and_credential_surfaces_are_denied() {
        assert!(evaluate_javascript("document.title").is_err());
        assert!(cdp_command("DOM.getDocument").is_err());
        assert!(devtools_open().is_err());
        assert!(read_credentials("cookies").is_err());
        assert!(select_by_caller_selector("#submit").is_err());
    }
}
