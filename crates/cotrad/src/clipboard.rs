//! SG-000038 bounded clipboard-read dispatch.
//!
//! This module dispatches the single SG-000038 `clipboard/read` shape
//! against the dedicated clipboard provider backed by the native
//! adapter. One explicit read returns at most 65536 bytes of Unicode
//! text with secret-pattern denial, clipboard-sequence binding, and
//! bounded secret-free evidence.
//!
//! Every read requires fresh per-read SOFT approval with exact digest
//! binding over the workspace, the policy revision, the clipboard
//! sequence observed immediately before approval, the fixed Unicode-text
//! format bound, and the fixed size bound. The sequence is revalidated
//! immediately after approval, so clipboard content that changes in
//! between fails closed as stale. Each approval authorizes at most one
//! sample: clipboard write, subscriptions, polling, monitoring, history,
//! and standing sessions have no dispatch path and return `Ok(None)` so
//! the caller fails closed through the STRONG gate or the legacy denial.
//! No MCP clipboard tool exists; the agent cannot reach reads through
//! its own tool surface.

use cotra_approval::{ApprovalBroker, ApprovalPrompt, ConsumeExpectation};
use cotra_contracts::{FailureCode, RequestEnvelope};
use cotra_policy::{Workspace, POLICY_REVISION};
use cotra_provider_clipboard::{
    clipboard_read_digest, read_bounded_text, ClipboardAdapter, NativeAdapter, MAX_CLIPBOARD_BYTES,
    UNICODE_TEXT_FORMAT,
};
use cotra_provider_fs::ProviderError;
use serde_json::Value;

/// Dispatch the SG-000038 clipboard shapes. Returns `Ok(None)` for
/// non-clipboard shapes and for denied clipboard shapes so the caller
/// falls through to the STRONG gate and the legacy dispatchers.
pub fn dispatch_clipboard(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Option<Value>, ProviderError> {
    if !cotra_provider_clipboard::is_clipboard_read_shape(&request.capability, &request.operation) {
        return Ok(None);
    }
    read_with_approval(workspace, approval, request, &NativeAdapter::new()).map(Some)
}

fn read_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
    adapter: &impl ClipboardAdapter,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "clipboard read shapes do not accept a target field",
        ));
    }
    reject_clipboard_arguments(request)?;
    // Bind clipboard state before approval without reading content: the
    // sequence number moves no clipboard bytes.
    let sequence = adapter
        .sequence_number()
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let digest = clipboard_read_digest(&workspace.id, POLICY_REVISION, sequence);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "read clipboard",
        "clipboard".to_owned(),
        format!(
            "format={UNICODE_TEXT_FORMAT} max_bytes={MAX_CLIPBOARD_BYTES} sequence={sequence} action=read"
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
            cotra_approval::now_ms(),
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let outcome = read_bounded_text(adapter, Some(sequence), &workspace.id, POLICY_REVISION)
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = outcome.evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    stamped["text"] = Value::String(outcome.text);
    Ok(stamped)
}

fn reject_clipboard_arguments(request: &RequestEnvelope) -> Result<(), ProviderError> {
    let arguments = request.arguments.as_object().ok_or_else(|| {
        ProviderError::new(
            FailureCode::InvalidRequest,
            "clipboard arguments must be an object",
        )
    })?;
    if let Some(key) = arguments.keys().next() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            format!("clipboard/read does not accept argument field: {key}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cotra_approval::test_support::FixedApprovalBroker;
    use cotra_approval::ApprovalDecision;
    use cotra_provider_clipboard::{ClipboardAdapter, ClipboardError};
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    struct FakeClipboard {
        sequence: Cell<u64>,
        content: RefCell<Option<String>>,
        non_text: bool,
        bump_after_first_sequence: Cell<bool>,
        sequence_calls: Cell<usize>,
        reads: Cell<usize>,
    }

    impl FakeClipboard {
        fn with_text(sequence: u64, text: &str) -> Self {
            Self {
                sequence: Cell::new(sequence),
                content: RefCell::new(Some(text.to_owned())),
                non_text: false,
                bump_after_first_sequence: Cell::new(false),
                sequence_calls: Cell::new(0),
                reads: Cell::new(0),
            }
        }

        fn empty() -> Self {
            Self {
                sequence: Cell::new(9),
                content: RefCell::new(None),
                non_text: false,
                bump_after_first_sequence: Cell::new(false),
                sequence_calls: Cell::new(0),
                reads: Cell::new(0),
            }
        }
    }

    impl ClipboardAdapter for FakeClipboard {
        fn sequence_number(&self) -> Result<u64, ClipboardError> {
            let calls = self.sequence_calls.get();
            self.sequence_calls.set(calls + 1);
            if self.bump_after_first_sequence.get() && calls >= 1 {
                self.sequence.set(self.sequence.get().saturating_add(1));
            }
            Ok(self.sequence.get())
        }

        fn read_unicode_text(&self) -> Result<String, ClipboardError> {
            self.reads.set(self.reads.get() + 1);
            if self.non_text {
                return Err(ClipboardError::new(
                    FailureCode::CapabilityDenied,
                    "clipboard holds no Unicode text",
                ));
            }
            self.content
                .borrow()
                .clone()
                .ok_or_else(|| ClipboardError::new(FailureCode::TargetStale, "clipboard is empty"))
        }
    }

    fn workspace() -> Workspace {
        Workspace {
            id: "clipboard-dispatch-workspace".to_owned(),
            root: PathBuf::from("C:\\clipboard-test"),
        }
    }

    fn read_request() -> RequestEnvelope {
        RequestEnvelope {
            version: cotra_contracts::INTERNAL_PROTOCOL_VERSION,
            request_id: "clipboard-read-1".to_owned(),
            client_session_id: "session-1".to_owned(),
            workspace_id: "clipboard-dispatch-workspace".to_owned(),
            capability: "clipboard".to_owned(),
            operation: "read".to_owned(),
            target: None,
            arguments: serde_json::json!({}),
        }
    }

    fn approved_read(
        adapter: &FakeClipboard,
        request: &RequestEnvelope,
    ) -> Result<Value, ProviderError> {
        read_with_approval(
            &workspace(),
            &FixedApprovalBroker(ApprovalDecision::Approved),
            request,
            adapter,
        )
    }

    #[test]
    fn non_clipboard_shapes_fall_through_without_actuation() {
        let workspace = workspace();
        let approval = FixedApprovalBroker(ApprovalDecision::Approved);
        for (capability, operation) in [
            ("clipboard", "write"),
            ("clipboard", "subscribe"),
            ("clipboard", "poll"),
            ("clipboard", "monitor"),
            ("clipboard", "history"),
            ("clipboard", "watch"),
            ("uia.input", "execute"),
            ("fs", "read"),
        ] {
            let mut request = read_request();
            request.capability = capability.to_owned();
            request.operation = operation.to_owned();
            let result = dispatch_clipboard(&workspace, &approval, &request).unwrap();
            assert!(
                result.is_none(),
                "shape must fall through: {capability}/{operation}"
            );
        }
    }

    #[test]
    fn target_and_argument_fields_are_rejected() {
        let approval = FixedApprovalBroker(ApprovalDecision::Approved);
        let adapter = FakeClipboard::with_text(4, "plain dispatch text");
        let mut with_target = read_request();
        with_target.target = Some("clipboard".to_owned());
        let error = read_with_approval(&workspace(), &approval, &with_target, &adapter)
            .expect_err("target must fail");
        assert!(matches!(error.code, FailureCode::InvalidRequest));
        let mut with_args = read_request();
        with_args.arguments = serde_json::json!({"format": "text"});
        let error = read_with_approval(&workspace(), &approval, &with_args, &adapter)
            .expect_err("arguments must fail");
        assert!(matches!(error.code, FailureCode::InvalidRequest));
        assert_eq!(adapter.reads.get(), 0);
    }

    #[test]
    fn approved_read_returns_text_with_evidence_and_approval_record() {
        let adapter = FakeClipboard::with_text(11, "quarterly report draft");
        let result = approved_read(&adapter, &read_request()).unwrap();
        assert_eq!(result["action"], "read");
        assert_eq!(result["format"], "unicode-text");
        assert_eq!(result["sequence"], 11);
        assert_eq!(result["text"], "quarterly report draft");
        assert!(result["approval_record"].is_string());
        assert!(result["content_digest"].is_string());
        assert_eq!(adapter.reads.get(), 1);
    }

    #[test]
    fn denied_approval_reads_no_clipboard_content() {
        let adapter = FakeClipboard::with_text(11, "quarterly report draft");
        let error = read_with_approval(
            &workspace(),
            &FixedApprovalBroker(ApprovalDecision::Denied),
            &read_request(),
            &adapter,
        )
        .expect_err("denied approval must fail");
        assert!(matches!(
            error.code,
            FailureCode::ApprovalDenied | FailureCode::ApprovalUnavailable
        ));
        assert_eq!(adapter.reads.get(), 0);
    }

    #[test]
    fn clipboard_drift_between_approval_and_read_fails_closed() {
        let adapter = FakeClipboard::with_text(11, "quarterly report draft");
        adapter.bump_after_first_sequence.set(true);
        let error =
            approved_read(&adapter, &read_request()).expect_err("drifted clipboard must fail");
        assert!(matches!(error.code, FailureCode::TargetStale));
    }

    #[test]
    fn secret_and_empty_clipboard_fail_closed_in_dispatch() {
        let secret = FakeClipboard::with_text(5, "api_key=DO-NOT-SHARE");
        let error = approved_read(&secret, &read_request()).expect_err("secret content must fail");
        assert!(matches!(error.code, FailureCode::CapabilityDenied));
        let empty = FakeClipboard::empty();
        let error = approved_read(&empty, &read_request()).expect_err("empty clipboard must fail");
        assert!(matches!(error.code, FailureCode::TargetStale));
    }
}
