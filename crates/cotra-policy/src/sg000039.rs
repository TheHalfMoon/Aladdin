#[path = "sg000038.rs"]
mod sg000038_legacy;

pub use cotra_approval::ApprovalClass;
pub use sg000038_legacy::{
    validate_relative_target, FetchDestination, PolicyDecision, PolicyError, PushDestination,
    Workspace,
};

use cotra_contracts::{FailureCode, RequestEnvelope};
use serde_json::Value;

pub const POLICY_REVISION: &str = "sg-000039-v1";

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    legacy: sg000038_legacy::PolicyEngine,
}

impl PolicyEngine {
    pub fn new(workspaces: Vec<Workspace>) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000038_legacy::PolicyEngine::new(workspaces)?,
        })
    }

    pub fn with_destinations(
        workspaces: Vec<Workspace>,
        fetch_destinations: Vec<FetchDestination>,
        push_destinations: Vec<PushDestination>,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000038_legacy::PolicyEngine::with_destinations(
                workspaces,
                fetch_destinations,
                push_destinations,
            )?,
        })
    }

    pub fn workspace(&self, id: &str) -> Option<&Workspace> {
        self.legacy.workspace(id)
    }

    pub fn fetch_destination(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Option<&FetchDestination> {
        self.legacy.fetch_destination(workspace_id, policy_id)
    }

    pub fn push_destination(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Option<&PushDestination> {
        self.legacy.push_destination(workspace_id, policy_id)
    }

    /// Authorize one request. The single SG-000039 bounded clipboard
    /// write shape is validated here so the clipboard provider can never
    /// be reached through a predecessor engine. The clipboard subscribe,
    /// poll, monitor, history, and watch shapes stay denied. Caller text
    /// is validated for presence, Unicode-text bounds, and secret denial
    /// with every other caller-supplied field denied, because placement
    /// accepts exactly one explicit `text` argument. The SG-000038 read
    /// shape and the derivation, proposal, capture, observation, invoke,
    /// value, select, toggle, scroll, execution, and interruption shapes
    /// retain their validation through the legacy engine with an
    /// upgraded revision. Every remaining shape falls through unchanged.
    pub fn authorize(&self, request: &RequestEnvelope) -> Result<PolicyDecision, PolicyError> {
        if is_clipboard_write_shape(request) {
            self.validate_clipboard_write_shape(request)?;
            let workspace = self
                .legacy
                .workspace(&request.workspace_id)
                .ok_or_else(|| {
                    policy_error(
                        FailureCode::WorkspaceDenied,
                        "requested workspace is not configured",
                    )
                })?;
            return Ok(PolicyDecision {
                workspace: workspace.clone(),
                policy_revision: POLICY_REVISION,
            });
        }
        if is_clipboard_read_shape(request) {
            self.validate_clipboard_read_shape(request)?;
            let workspace = self
                .legacy
                .workspace(&request.workspace_id)
                .ok_or_else(|| {
                    policy_error(
                        FailureCode::WorkspaceDenied,
                        "requested workspace is not configured",
                    )
                })?;
            return Ok(PolicyDecision {
                workspace: workspace.clone(),
                policy_revision: POLICY_REVISION,
            });
        }
        if is_execute_shape(request) {
            self.validate_execute_shape(request)?;
            let workspace = self
                .legacy
                .workspace(&request.workspace_id)
                .ok_or_else(|| {
                    policy_error(
                        FailureCode::WorkspaceDenied,
                        "requested workspace is not configured",
                    )
                })?;
            return Ok(PolicyDecision {
                workspace: workspace.clone(),
                policy_revision: POLICY_REVISION,
            });
        }
        if is_retained_uia_shape(request) {
            let decision = self.legacy.authorize(request)?;
            let workspace = self
                .legacy
                .workspace(&request.workspace_id)
                .ok_or_else(|| {
                    policy_error(
                        FailureCode::WorkspaceDenied,
                        "requested workspace is not configured",
                    )
                })?;
            let _ = decision;
            return Ok(PolicyDecision {
                workspace: workspace.clone(),
                policy_revision: POLICY_REVISION,
            });
        }
        if cotra_provider_clipboard::is_denied_clipboard_shape(
            &request.capability,
            &request.operation,
        ) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000039 authorizes only one explicit clipboard/read sample and one explicit clipboard/write placement with no subscription, polling, history, watcher, or paste authority",
                    request.capability, request.operation
                ),
            ));
        }
        if cotra_provider_uia::is_denied_uia_shape(&request.capability, &request.operation) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000039 authorizes only uia.process/observe, uia.window/list, uia.window/observe, uia.tree/observe, uia.element/observe, structured uia.element/invoke, structured uia.element/set_value, structured uia.element/select, structured uia.element/toggle, structured uia.element/scroll, window-scoped uia.screenshot/capture, non-actuating uia.visual/propose, proposal-only uia.coordinates/propose, bounded uia.input/execute on one exact typed coordinate identity with click-only operation under a single-use lease, one explicit clipboard/read sample, and one explicit clipboard/write placement, and keyboard, drag, raw synthetic input, lease reuse, monitor scope, desktop scope, clipboard surveillance, paste automation, network, and elevation remain absent",
                    request.capability, request.operation
                ),
            ));
        }
        let decision = self.legacy.authorize(request)?;
        Ok(PolicyDecision {
            workspace: decision.workspace,
            policy_revision: POLICY_REVISION,
        })
    }
}

pub fn approval_class_for(capability: &str, operation: &str) -> ApprovalClass {
    if cotra_provider_clipboard::is_clipboard_write_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_clipboard::is_clipboard_read_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_input_execute_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_coordinate_propose_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_visual_propose_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_capture_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_scroll_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_toggle_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_select_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_value_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_invoke_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_allowed_uia_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if cotra_provider_uia::is_denied_uia_shape(capability, operation) {
        return ApprovalClass::Strong;
    }
    if cotra_provider_clipboard::is_denied_clipboard_shape(capability, operation) {
        return ApprovalClass::Strong;
    }
    sg000038_legacy::approval_class_for(capability, operation)
}

fn is_clipboard_read_shape(request: &RequestEnvelope) -> bool {
    cotra_provider_clipboard::is_clipboard_read_shape(&request.capability, &request.operation)
}

fn is_clipboard_write_shape(request: &RequestEnvelope) -> bool {
    cotra_provider_clipboard::is_clipboard_write_shape(&request.capability, &request.operation)
}

fn is_execute_shape(request: &RequestEnvelope) -> bool {
    cotra_provider_uia::is_input_execute_shape(&request.capability, &request.operation)
}

fn is_retained_uia_shape(request: &RequestEnvelope) -> bool {
    cotra_provider_uia::is_allowed_uia_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_invoke_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_value_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_select_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_toggle_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_scroll_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_capture_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_visual_propose_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_coordinate_propose_shape(&request.capability, &request.operation)
}

/// Fields that would widen one bounded clipboard placement into
/// surveillance, watcher creation, paste automation, binary transfer,
/// or secret access. None of them is accepted by the write shape, which
/// takes exactly one explicit `text` argument; format and sequence are
/// server-derived.
const CLIPBOARD_WRITE_WIDENING_FIELDS: &[&str] = &[
    "format",
    "encoding",
    "content",
    "bytes",
    "size",
    "max_bytes",
    "sequence",
    "expected_sequence",
    "subscribe",
    "subscription",
    "poll",
    "interval",
    "monitor",
    "watch",
    "watcher",
    "history",
    "retain",
    "paste",
    "input",
    "keyboard",
    "key",
    "sendinput",
    "click",
    "focus",
    "set",
    "clear",
    "image",
    "bitmap",
    "file",
    "files",
    "html",
    "rtf",
    "object",
    "binary",
    "approval",
    "approval_id",
    "nonce",
    "digest",
    "password",
    "secret",
    "token",
    "credentials",
];

/// Fields that would widen one bounded clipboard sample into write,
/// subscription, polling, monitoring, history, binary transfer, or
/// secret access. None of them is accepted by the read shape, which
/// takes no caller arguments at all; format, size, sequence, and content
/// are server-derived.
const CLIPBOARD_WIDENING_FIELDS: &[&str] = &[
    "format",
    "encoding",
    "text",
    "content",
    "bytes",
    "size",
    "max_bytes",
    "sequence",
    "subscribe",
    "subscription",
    "poll",
    "interval",
    "monitor",
    "watch",
    "watcher",
    "history",
    "retain",
    "write",
    "set",
    "clear",
    "image",
    "bitmap",
    "file",
    "files",
    "html",
    "rtf",
    "object",
    "binary",
    "approval",
    "approval_id",
    "nonce",
    "digest",
    "password",
    "secret",
    "token",
    "credentials",
];

/// Fields that would widen bounded click execution into keyboard, drag,
/// raw synthetic input, standing sessions, lease reuse, monitor or
/// desktop actuation, forged interruption state, or secret access. None
/// of them is accepted by the execution shape; callers can only name one
/// server-allocated typed coordinate identity plus its expected
/// derivation generation and the click-only operation. Caller-supplied
/// lease identities, window handles, coordinates, interruption epochs,
/// reasons, origins, and injection flags are rejected here because
/// leases, windows, coordinates, epochs, and origins are server-derived.
const EXECUTE_WIDENING_FIELDS: &[&str] = &[
    "pid",
    "process_handle",
    "hwnd",
    "handle",
    "window_handle",
    "window_id",
    "process_id",
    "frame_id",
    "proposal_id",
    "coord_id_claim",
    "lease",
    "lease_id",
    "lease_expiry",
    "expires",
    "interruption_epoch",
    "interruption",
    "interrupt",
    "epoch",
    "reason",
    "origin",
    "input_origin",
    "injected",
    "is_injected",
    "is_cotra_own",
    "human",
    "x",
    "y",
    "point",
    "rect",
    "region",
    "width",
    "height",
    "screen",
    "monitor",
    "desktop",
    "display",
    "button",
    "buttons",
    "click_count",
    "count",
    "repeat",
    "drag",
    "keyboard",
    "key",
    "keys",
    "text",
    "wheel",
    "touch",
    "gesture",
    "focus",
    "sendinput",
    "synthetic",
    "input",
    "pixels",
    "image",
    "screenshot",
    "scope",
    "coordinate",
    "coordinates",
    "proposal",
    "frame",
    "invoke",
    "value",
    "select",
    "toggle",
    "scroll",
    "clipboard",
    "password",
    "passwords",
    "secret",
    "secrets",
    "token",
    "tokens",
    "credentials",
    "cookies",
    "session",
    "approval",
    "approval_id",
    "nonce",
    "digest",
    "path",
    "file",
    "executable",
    "script",
    "command",
    "argv",
    "elevate",
    "elevation",
    "admin",
    "com_object",
    "pattern_value",
    "process_command",
    "control_type",
    "pattern",
    "enabled",
    "state",
];

impl PolicyEngine {
    /// Validate the single execution shape. This layer is pure policy: it
    /// performs no OS access and no mutation, and it accepts no widening
    /// field of any kind. Operation is fixed to `click`; every other
    /// operation is denied here and again in the registry.
    fn validate_execute_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
        if request.version != cotra_contracts::INTERNAL_PROTOCOL_VERSION {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                "unsupported internal protocol version",
            ));
        }
        if self.legacy.workspace(&request.workspace_id).is_none() {
            return Err(policy_error(
                FailureCode::WorkspaceDenied,
                "requested workspace is not configured",
            ));
        }
        if request.target.is_some() {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                "uia bounded execution shapes do not accept a target field",
            ));
        }
        let arguments = request.arguments.as_object().ok_or_else(|| {
            policy_error(
                FailureCode::InvalidRequest,
                "uia arguments must be an object",
            )
        })?;
        let shape = format!("{}/{}", request.capability, request.operation);
        if let Some(key) = arguments
            .keys()
            .find(|key| EXECUTE_WIDENING_FIELDS.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "uia bounded execution request must not carry authority-widening field: {key}; only coord_id, expected_derivation_generation, and operation are accepted"
                ),
            ));
        }
        let coord_id = required_shape_string(&shape, arguments, "coord_id")?;
        if !cotra_provider_uia::is_well_formed_coord_id(coord_id) {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} coord_id is malformed"),
            ));
        }
        required_shape_u64(&shape, arguments, "expected_derivation_generation")?;
        let operation = required_shape_string(&shape, arguments, "operation")?;
        if !cotra_provider_uia::is_execute_operation(operation) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!("{shape} operation is fixed to click"),
            ));
        }
        const ALLOWED: &[&str] = &["coord_id", "expected_derivation_generation", "operation"];
        if let Some(key) = arguments
            .keys()
            .find(|key| !ALLOWED.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} does not accept argument field: {key}"),
            ));
        }
        Ok(())
    }

    /// Validate the single clipboard-read shape. This layer is pure
    /// policy: it performs no OS access and no mutation, and it accepts
    /// no caller arguments at all. Format, size, sequence, and content
    /// are server-derived; any caller-supplied field is an explicit
    /// widening attempt and fails closed.
    fn validate_clipboard_read_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
        if request.version != cotra_contracts::INTERNAL_PROTOCOL_VERSION {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                "unsupported internal protocol version",
            ));
        }
        if self.legacy.workspace(&request.workspace_id).is_none() {
            return Err(policy_error(
                FailureCode::WorkspaceDenied,
                "requested workspace is not configured",
            ));
        }
        if request.target.is_some() {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                "clipboard read shapes do not accept a target field",
            ));
        }
        let arguments = request.arguments.as_object().ok_or_else(|| {
            policy_error(
                FailureCode::InvalidRequest,
                "clipboard arguments must be an object",
            )
        })?;
        let shape = format!("{}/{}", request.capability, request.operation);
        if let Some(key) = arguments.keys().next() {
            if CLIPBOARD_WIDENING_FIELDS.contains(&key.as_str()) {
                return Err(policy_error(
                    FailureCode::CapabilityDenied,
                    format!(
                        "clipboard read request must not carry authority-widening field: {key}; clipboard/read accepts no caller arguments"
                    ),
                ));
            }
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} does not accept argument field: {key}"),
            ));
        }
        Ok(())
    }

    /// Validate the single clipboard-write shape. This layer is pure
    /// policy: it performs no OS access and no mutation. Placement
    /// accepts exactly one explicit `text` argument holding Unicode text
    /// within the hard bound; format and sequence are server-derived and
    /// every other caller-supplied field is an explicit widening attempt
    /// and fails closed. Secret denial is enforced here and again in the
    /// provider so denied text never reaches the operating system.
    fn validate_clipboard_write_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
        if request.version != cotra_contracts::INTERNAL_PROTOCOL_VERSION {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                "unsupported internal protocol version",
            ));
        }
        if self.legacy.workspace(&request.workspace_id).is_none() {
            return Err(policy_error(
                FailureCode::WorkspaceDenied,
                "requested workspace is not configured",
            ));
        }
        if request.target.is_some() {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                "clipboard write shapes do not accept a target field",
            ));
        }
        let arguments = request.arguments.as_object().ok_or_else(|| {
            policy_error(
                FailureCode::InvalidRequest,
                "clipboard arguments must be an object",
            )
        })?;
        let shape = format!("{}/{}", request.capability, request.operation);
        if let Some(key) = arguments
            .keys()
            .find(|key| CLIPBOARD_WRITE_WIDENING_FIELDS.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "clipboard write request must not carry authority-widening field: {key}; clipboard/write accepts only the text argument"
                ),
            ));
        }
        let text = required_shape_string(&shape, arguments, "text")?;
        if text.is_empty() {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} text must not be empty"),
            ));
        }
        if text.len() > cotra_provider_clipboard::MAX_CLIPBOARD_BYTES {
            return Err(policy_error(
                FailureCode::OutputLimit,
                format!("{shape} text exceeds the bounded placement size"),
            ));
        }
        if cotra_provider_clipboard::contains_secret_material(text) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!("{shape} text holds secret material which bounded writes never place"),
            ));
        }
        const ALLOWED: &[&str] = &["text"];
        if let Some(key) = arguments
            .keys()
            .find(|key| !ALLOWED.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} does not accept argument field: {key}"),
            ));
        }
        Ok(())
    }
}

fn required_shape_string<'a>(
    shape: &str,
    arguments: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Result<&'a str, PolicyError> {
    arguments.get(name).and_then(Value::as_str).ok_or_else(|| {
        policy_error(
            FailureCode::InvalidRequest,
            format!("{shape} requires arguments.{name} as a string"),
        )
    })
}

fn required_shape_u64(
    shape: &str,
    arguments: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<u64, PolicyError> {
    arguments.get(name).and_then(Value::as_u64).ok_or_else(|| {
        policy_error(
            FailureCode::InvalidRequest,
            format!("{shape} requires arguments.{name} as an unsigned integer"),
        )
    })
}

fn policy_error(code: FailureCode, message: impl Into<String>) -> PolicyError {
    PolicyError {
        code,
        message: message.into(),
    }
}
