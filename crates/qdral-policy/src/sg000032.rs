#[path = "sg000031.rs"]
mod sg000031_legacy;

pub use qdral_approval::ApprovalClass;
pub use sg000031_legacy::{
    validate_relative_target, FetchDestination, PolicyDecision, PolicyError, PushDestination,
    Workspace,
};

use qdral_contracts::{FailureCode, RequestEnvelope};
use serde_json::Value;

pub const POLICY_REVISION: &str = "sg-000032-v1";

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    legacy: sg000031_legacy::PolicyEngine,
}

impl PolicyEngine {
    pub fn new(workspaces: Vec<Workspace>) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000031_legacy::PolicyEngine::new(workspaces)?,
        })
    }

    pub fn with_destinations(
        workspaces: Vec<Workspace>,
        fetch_destinations: Vec<FetchDestination>,
        push_destinations: Vec<PushDestination>,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000031_legacy::PolicyEngine::with_destinations(
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

    /// Authorize one request. The single SG-000032 scroll shape is validated
    /// here so the UIA registry can never be reached through a predecessor
    /// engine. Observation, invoke, value, select, and toggle shapes retain
    /// their validation through the legacy engine with an upgraded revision.
    /// Every remaining shape falls through unchanged.
    pub fn authorize(&self, request: &RequestEnvelope) -> Result<PolicyDecision, PolicyError> {
        if is_scroll_shape(request) {
            self.validate_scroll_shape(request)?;
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
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000032 authorizes only uia.process/observe, uia.window/list, uia.window/observe, uia.tree/observe, uia.element/observe, structured uia.element/invoke, structured uia.element/set_value, structured uia.element/select, structured uia.element/toggle, and structured uia.element/scroll on exact typed eligible elements, and click, focus, keyboard, mouse, SendInput, coordinates, screenshots, clipboard, network, and elevation remain absent",
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
    sg000031_legacy::approval_class_for(capability, operation)
}

fn is_scroll_shape(request: &RequestEnvelope) -> bool {
    qdral_provider_uia::is_scroll_shape(&request.capability, &request.operation)
}

fn is_retained_uia_shape(request: &RequestEnvelope) -> bool {
    qdral_provider_uia::is_allowed_uia_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_invoke_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_value_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_select_shape(&request.capability, &request.operation)
        || qdral_provider_uia::is_toggle_shape(&request.capability, &request.operation)
}

/// Fields that would widen UIA scroll into generic actuation, unbounded
/// scrolling, synthetic input, unrestricted introspection, coordinate
/// control, or secret access. None of them is accepted by the scroll shape;
/// callers can only name one server-allocated typed element plus its
/// expected tree generation, expected control type, bounded direction,
/// bounded amount, and expected scroll position.
const SCROLL_WIDENING_FIELDS: &[&str] = &[
    "pid",
    "process_handle",
    "hwnd",
    "handle",
    "window_handle",
    "runtime_id",
    "runtime",
    "selector",
    "xpath",
    "coordinate",
    "coordinates",
    "x",
    "y",
    "point",
    "rect",
    "invoke",
    "click",
    "value",
    "select",
    "toggle",
    "focus",
    "keyboard",
    "mouse",
    "sendinput",
    "synthetic",
    "screenshot",
    "frame",
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
    "window_id",
    "process_id",
    "control_type",
    "pattern",
    "enabled",
    "state",
    "repeat",
    "repeats",
    "repeat_count",
    "count",
    "wheel",
    "touch",
    "gesture",
    "pageup",
    "pagedown",
];

impl PolicyEngine {
    /// Validate the single scroll shape. This layer is pure policy: it
    /// performs no OS access and no mutation, and it accepts no widening
    /// field of any kind. Direction is limited to up, down, left, and
    /// right; amount is bounded between 1 and 100; expected scroll
    /// positions are bounded between 0 and 100; repeat counts and
    /// indefinite scrolling are denied.
    fn validate_scroll_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
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
                "uia scroll shapes do not accept a target field",
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
            .find(|key| SCROLL_WIDENING_FIELDS.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "uia scroll request must not carry authority-widening field: {key}; only element_id, expected_tree_generation, expected_control_type, direction, amount, expected_horizontal_percent, and expected_vertical_percent are accepted"
                ),
            ));
        }
        let element_id = required_shape_string(&shape, arguments, "element_id")?;
        if !qdral_provider_uia::is_well_formed_element_id(element_id) {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} element_id is malformed"),
            ));
        }
        required_shape_u64(&shape, arguments, "expected_tree_generation")?;
        let control_type = required_shape_string(&shape, arguments, "expected_control_type")?;
        if control_type.is_empty() || control_type.len() > 256 {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} expected_control_type is empty or too large"),
            ));
        }
        if !qdral_provider_uia::is_scroll_eligible_control_type(control_type) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!("{shape} expected_control_type does not permit Scroll actuation"),
            ));
        }
        let direction = required_shape_string(&shape, arguments, "direction")?;
        if !qdral_provider_uia::is_scroll_direction(direction) {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} direction must be one of up, down, left, or right"),
            ));
        }
        let amount = required_shape_u64(&shape, arguments, "amount")?;
        if !(1..=qdral_provider_uia::MAX_SCROLL_AMOUNT).contains(&amount) {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!(
                    "{shape} amount must be between 1 and {} inclusive",
                    qdral_provider_uia::MAX_SCROLL_AMOUNT
                ),
            ));
        }
        let expected_horizontal =
            required_shape_u64(&shape, arguments, "expected_horizontal_percent")?;
        if expected_horizontal > 100 {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} expected_horizontal_percent must be between 0 and 100 inclusive"),
            ));
        }
        let expected_vertical = required_shape_u64(&shape, arguments, "expected_vertical_percent")?;
        if expected_vertical > 100 {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} expected_vertical_percent must be between 0 and 100 inclusive"),
            ));
        }
        const ALLOWED: &[&str] = &[
            "element_id",
            "expected_tree_generation",
            "expected_control_type",
            "direction",
            "amount",
            "expected_horizontal_percent",
            "expected_vertical_percent",
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
