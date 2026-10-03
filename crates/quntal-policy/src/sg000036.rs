#[path = "sg000035.rs"]
mod sg000035_legacy;

pub use quntal_approval::ApprovalClass;
pub use sg000035_legacy::{
    validate_relative_target, FetchDestination, PolicyDecision, PolicyError, PushDestination,
    Workspace,
};

use quntal_contracts::{FailureCode, RequestEnvelope};
use serde_json::Value;

pub const POLICY_REVISION: &str = "sg-000036-v1";

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    legacy: sg000035_legacy::PolicyEngine,
}

impl PolicyEngine {
    pub fn new(workspaces: Vec<Workspace>) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000035_legacy::PolicyEngine::new(workspaces)?,
        })
    }

    pub fn with_destinations(
        workspaces: Vec<Workspace>,
        fetch_destinations: Vec<FetchDestination>,
        push_destinations: Vec<PushDestination>,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000035_legacy::PolicyEngine::with_destinations(
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

    /// Authorize one request. The single SG-000036 bounded execution
    /// shape is validated here so the UIA registry can never be reached
    /// through a predecessor engine. The raw `uia.input/keyboard`,
    /// `uia.input/mouse`, and `uia.input/sendinput` shapes stay denied.
    /// Derivation, proposal, capture, observation, invoke, value, select,
    /// toggle, and scroll shapes retain their validation through the
    /// legacy engine with an upgraded revision. Every remaining shape
    /// falls through unchanged.
    pub fn authorize(&self, request: &RequestEnvelope) -> Result<PolicyDecision, PolicyError> {
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
        if quntal_provider_uia::is_denied_uia_shape(&request.capability, &request.operation) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000036 authorizes only uia.process/observe, uia.window/list, uia.window/observe, uia.tree/observe, uia.element/observe, structured uia.element/invoke, structured uia.element/set_value, structured uia.element/select, structured uia.element/toggle, structured uia.element/scroll, window-scoped uia.screenshot/capture, non-actuating uia.visual/propose, proposal-only uia.coordinates/propose, and bounded uia.input/execute on one exact typed coordinate identity with click-only operation under a single-use lease, and keyboard, drag, raw synthetic input, lease reuse, monitor scope, desktop scope, clipboard, network, and elevation remain absent",
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
    if quntal_provider_uia::is_input_execute_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_coordinate_propose_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_visual_propose_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_capture_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_scroll_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_toggle_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_select_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_value_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_invoke_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_allowed_uia_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if quntal_provider_uia::is_denied_uia_shape(capability, operation) {
        return ApprovalClass::Strong;
    }
    sg000035_legacy::approval_class_for(capability, operation)
}

fn is_execute_shape(request: &RequestEnvelope) -> bool {
    quntal_provider_uia::is_input_execute_shape(&request.capability, &request.operation)
}

fn is_retained_uia_shape(request: &RequestEnvelope) -> bool {
    quntal_provider_uia::is_allowed_uia_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_invoke_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_value_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_select_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_toggle_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_scroll_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_capture_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_visual_propose_shape(&request.capability, &request.operation)
        || quntal_provider_uia::is_coordinate_propose_shape(&request.capability, &request.operation)
}

/// Fields that would widen bounded click execution into keyboard, drag,
/// raw synthetic input, standing sessions, lease reuse, monitor or
/// desktop actuation, or secret access. None of them is accepted by the
/// execution shape; callers can only name one server-allocated typed
/// coordinate identity plus its expected derivation generation and the
/// click-only operation. Caller-supplied lease identities, window
/// handles, and coordinates are rejected here because leases, windows,
/// and coordinates are server-derived.
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
        if request.version != quntal_contracts::INTERNAL_PROTOCOL_VERSION {
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
        if !quntal_provider_uia::is_well_formed_coord_id(coord_id) {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} coord_id is malformed"),
            ));
        }
        required_shape_u64(&shape, arguments, "expected_derivation_generation")?;
        let operation = required_shape_string(&shape, arguments, "operation")?;
        if !quntal_provider_uia::is_execute_operation(operation) {
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
