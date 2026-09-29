#[path = "sg000032.rs"]
mod sg000032_legacy;

pub use cotra_approval::ApprovalClass;
pub use sg000032_legacy::{
    validate_relative_target, FetchDestination, PolicyDecision, PolicyError, PushDestination,
    Workspace,
};

use cotra_contracts::{FailureCode, RequestEnvelope};
use serde_json::Value;

pub const POLICY_REVISION: &str = "sg-000033-v1";

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    legacy: sg000032_legacy::PolicyEngine,
}

impl PolicyEngine {
    pub fn new(workspaces: Vec<Workspace>) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000032_legacy::PolicyEngine::new(workspaces)?,
        })
    }

    pub fn with_destinations(
        workspaces: Vec<Workspace>,
        fetch_destinations: Vec<FetchDestination>,
        push_destinations: Vec<PushDestination>,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000032_legacy::PolicyEngine::with_destinations(
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

    /// Authorize one request. The single SG-000033 window-scoped capture
    /// shape is validated here so the UIA registry can never be reached
    /// through a predecessor engine. Observation, invoke, value, select,
    /// toggle, and scroll shapes retain their validation through the
    /// legacy engine with an upgraded revision. Every remaining shape
    /// falls through unchanged.
    pub fn authorize(&self, request: &RequestEnvelope) -> Result<PolicyDecision, PolicyError> {
        if is_capture_shape(request) {
            self.validate_capture_shape(request)?;
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
        if cotra_provider_uia::is_denied_uia_shape(&request.capability, &request.operation) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000033 authorizes only uia.process/observe, uia.window/list, uia.window/observe, uia.tree/observe, uia.element/observe, structured uia.element/invoke, structured uia.element/set_value, structured uia.element/select, structured uia.element/toggle, structured uia.element/scroll, and window-scoped uia.screenshot/capture on one exact typed window with target-window scope, and click, focus, keyboard, mouse, SendInput, coordinates, visual proposals, monitor scope, desktop scope, clipboard, network, and elevation remain absent",
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
    sg000032_legacy::approval_class_for(capability, operation)
}

fn is_capture_shape(request: &RequestEnvelope) -> bool {
    cotra_provider_uia::is_capture_shape(&request.capability, &request.operation)
}

fn is_retained_uia_shape(request: &RequestEnvelope) -> bool {
    cotra_provider_uia::is_allowed_uia_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_invoke_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_value_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_select_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_toggle_shape(&request.capability, &request.operation)
        || cotra_provider_uia::is_scroll_shape(&request.capability, &request.operation)
}

/// Fields that would widen window-scoped capture into monitor or desktop
/// capture, caller-selected regions, coordinate derivation, visual
/// proposals, input execution, unrestricted introspection, or secret
/// access. None of them is accepted by the capture shape; callers can
/// only name one server-allocated typed window plus its expected window
/// generation and the fixed target-window scope. Caller-supplied frame
/// identities are rejected here because frames are server-allocated.
const CAPTURE_WIDENING_FIELDS: &[&str] = &[
    "pid",
    "process_handle",
    "hwnd",
    "handle",
    "window_handle",
    "runtime_id",
    "runtime",
    "selector",
    "xpath",
    "frame",
    "frame_id",
    "capture_generation",
    "coordinate",
    "coordinates",
    "x",
    "y",
    "point",
    "rect",
    "region",
    "monitor",
    "desktop",
    "screen",
    "display",
    "geometry",
    "width",
    "height",
    "pixels",
    "image",
    "visual",
    "proposal",
    "propose",
    "target",
    "invoke",
    "click",
    "value",
    "select",
    "toggle",
    "scroll",
    "focus",
    "keyboard",
    "mouse",
    "sendinput",
    "synthetic",
    "input",
    "lease",
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
    "window_id_claim",
    "process_id",
    "control_type",
    "pattern",
    "enabled",
    "state",
];

impl PolicyEngine {
    /// Validate the single capture shape. This layer is pure policy: it
    /// performs no OS access and no mutation, and it accepts no widening
    /// field of any kind. Scope is fixed to `target-window`; monitor
    /// scope, desktop scope, caller-selected regions, and arbitrary
    /// geometry are denied.
    fn validate_capture_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
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
                "uia screenshot capture shapes do not accept a target field",
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
            .find(|key| CAPTURE_WIDENING_FIELDS.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "uia screenshot capture request must not carry authority-widening field: {key}; only window_id, expected_window_generation, and scope are accepted"
                ),
            ));
        }
        let window_id = required_shape_string(&shape, arguments, "window_id")?;
        if !cotra_provider_uia::is_well_formed_window_id(window_id) {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} window_id is malformed"),
            ));
        }
        required_shape_u64(&shape, arguments, "expected_window_generation")?;
        let scope = required_shape_string(&shape, arguments, "scope")?;
        if !cotra_provider_uia::is_capture_scope(scope) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "{shape} scope is fixed to target-window; monitor scope, desktop scope, and caller-selected regions are denied"
                ),
            ));
        }
        const ALLOWED: &[&str] = &["window_id", "expected_window_generation", "scope"];
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
