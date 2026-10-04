#[path = "sg000033.rs"]
mod sg000033_legacy;

pub use qdral_approval::ApprovalClass;
pub use sg000033_legacy::{
    validate_relative_target, FetchDestination, PolicyDecision, PolicyError, PushDestination,
    Workspace,
};

use qdral_contracts::{FailureCode, RequestEnvelope};
use serde_json::Value;

pub const POLICY_REVISION: &str = "sg-000034-v1";

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    legacy: sg000033_legacy::PolicyEngine,
}

impl PolicyEngine {
    pub fn new(workspaces: Vec<Workspace>) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000033_legacy::PolicyEngine::new(workspaces)?,
        })
    }

    pub fn with_destinations(
        workspaces: Vec<Workspace>,
        fetch_destinations: Vec<FetchDestination>,
        push_destinations: Vec<PushDestination>,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000033_legacy::PolicyEngine::with_destinations(
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

    /// Authorize one request. The single SG-000034 non-actuating proposal
    /// shape is validated here so the UIA registry can never be reached
    /// through a predecessor engine. Capture, observation, invoke, value,
    /// select, toggle, and scroll shapes retain their validation through
    /// the legacy engine with an upgraded revision. Every remaining shape
    /// falls through unchanged.
    pub fn authorize(&self, request: &RequestEnvelope) -> Result<PolicyDecision, PolicyError> {
        if is_visual_shape(request) {
            self.validate_visual_shape(request)?;
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
        if qdral_provider_uia::is_denied_uia_shape(&request.capability, &request.operation) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000034 authorizes only uia.process/observe, uia.window/list, uia.window/observe, uia.tree/observe, uia.element/observe, structured uia.element/invoke, structured uia.element/set_value, structured uia.element/select, structured uia.element/toggle, structured uia.element/scroll, window-scoped uia.screenshot/capture, and non-actuating uia.visual/propose on one exact typed frame with a bounded inside-frame region, and click, focus, keyboard, mouse, SendInput, coordinates, monitor scope, desktop scope, clipboard, network, and elevation remain absent",
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
    if qdral_provider_uia::is_visual_propose_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_capture_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_scroll_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_toggle_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_select_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_value_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_invoke_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_allowed_uia_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_denied_uia_shape(capability, operation) {
        return ApprovalClass::Strong;
    }
    sg000033_legacy::approval_class_for(capability, operation)
}

fn is_visual_shape(request: &RequestEnvelope) -> bool {
    qdral_provider_uia::is_visual_propose_shape(&request.capability, &request.operation)
}

fn is_retained_uia_shape(request: &RequestEnvelope) -> bool {
    qdral_provider_uia::is_allowed_uia_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_invoke_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_value_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_select_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_toggle_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_scroll_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_capture_shape(&request.capability, &request.operation)
}

/// Fields that would widen a bounded frame-region proposal into
/// coordinate derivation, input execution, unrestricted introspection,
/// monitor or desktop capture, or secret access. None of them is accepted
/// by the proposal shape; callers can only name one server-allocated
/// typed frame plus its expected capture generation and bounded region.
/// Caller-supplied proposal identities are rejected here because
/// proposals are server-allocated.
const VISUAL_WIDENING_FIELDS: &[&str] = &[
    "pid",
    "process_handle",
    "hwnd",
    "handle",
    "window_handle",
    "window_id",
    "process_id",
    "runtime_id",
    "runtime",
    "selector",
    "xpath",
    "proposal",
    "proposal_id",
    "proposal_generation",
    "capture_generation_claim",
    "frame_bytes",
    "frame_pixels",
    "pixels",
    "image",
    "screenshot",
    "scope",
    "coordinate",
    "coordinates",
    "point",
    "screen",
    "monitor",
    "desktop",
    "display",
    "region_hint",
    "label",
    "model",
    "inference",
    "click",
    "mouse",
    "keyboard",
    "sendinput",
    "synthetic",
    "input",
    "execute",
    "execution",
    "lease",
    "invoke",
    "value",
    "select",
    "toggle",
    "scroll",
    "focus",
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
    /// Validate the single proposal shape. This layer is pure policy: it
    /// performs no OS access and no mutation, and it accepts no widening
    /// field of any kind. The region carries explicit x, y, width, and
    /// height as unsigned integers; inside-frame confinement is enforced
    /// by the registry against the exact frame geometry.
    fn validate_visual_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
        if request.version != qdral_contracts::INTERNAL_PROTOCOL_VERSION {
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
                "uia visual proposal shapes do not accept a target field",
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
            .find(|key| VISUAL_WIDENING_FIELDS.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "uia visual proposal request must not carry authority-widening field: {key}; only frame_id, expected_capture_generation, x, y, width, and height are accepted"
                ),
            ));
        }
        let frame_id = required_shape_string(&shape, arguments, "frame_id")?;
        if !qdral_provider_uia::is_well_formed_frame_id(frame_id) {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} frame_id is malformed"),
            ));
        }
        required_shape_u64(&shape, arguments, "expected_capture_generation")?;
        required_shape_u64(&shape, arguments, "x")?;
        required_shape_u64(&shape, arguments, "y")?;
        let width = required_shape_u64(&shape, arguments, "width")?;
        if width == 0 {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} width must be nonzero"),
            ));
        }
        let height = required_shape_u64(&shape, arguments, "height")?;
        if height == 0 {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} height must be nonzero"),
            ));
        }
        const ALLOWED: &[&str] = &[
            "frame_id",
            "expected_capture_generation",
            "x",
            "y",
            "width",
            "height",
        ];
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
