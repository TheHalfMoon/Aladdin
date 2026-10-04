#[path = "sg000027.rs"]
mod sg000027_legacy;

pub use qdral_approval::ApprovalClass;
pub use sg000027_legacy::{
    validate_relative_target, FetchDestination, PolicyDecision, PolicyError, PushDestination,
    Workspace,
};

use qdral_contracts::{FailureCode, RequestEnvelope};
use serde_json::Value;

pub const POLICY_REVISION: &str = "sg-000028-v1";

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    legacy: sg000027_legacy::PolicyEngine,
}

impl PolicyEngine {
    pub fn new(workspaces: Vec<Workspace>) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000027_legacy::PolicyEngine::new(workspaces)?,
        })
    }

    pub fn with_destinations(
        workspaces: Vec<Workspace>,
        fetch_destinations: Vec<FetchDestination>,
        push_destinations: Vec<PushDestination>,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000027_legacy::PolicyEngine::with_destinations(
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

    /// Authorize one request. The single SG-000028 invoke shape is validated
    /// here so the UIA registry can never be reached through a predecessor
    /// engine. Observation shapes retain their SG-000027 validation through
    /// the legacy engine with an upgraded revision. Every remaining shape
    /// falls through unchanged.
    pub fn authorize(&self, request: &RequestEnvelope) -> Result<PolicyDecision, PolicyError> {
        if is_invoke_shape(request) {
            self.validate_invoke_shape(request)?;
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
        if is_uia_observation_shape(request) {
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
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000028 authorizes only uia.process/observe, uia.window/list, uia.window/observe, uia.tree/observe, uia.element/observe, and structured uia.element/invoke on exact typed eligible elements, and click, value setting, select, toggle, scroll, focus, keyboard, mouse, SendInput, coordinates, screenshots, clipboard, network, and elevation remain absent",
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
    if qdral_provider_uia::is_invoke_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_allowed_uia_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_uia::is_denied_uia_shape(capability, operation) {
        return ApprovalClass::Strong;
    }
    sg000027_legacy::approval_class_for(capability, operation)
}

fn is_invoke_shape(request: &RequestEnvelope) -> bool {
    qdral_provider_uia::is_invoke_shape(&request.capability, &request.operation)
}

fn is_uia_observation_shape(request: &RequestEnvelope) -> bool {
    qdral_provider_uia::is_allowed_uia_shape(&request.capability, &request.operation)
}

/// Fields that would widen UIA invoke into generic actuation, synthetic
/// input, unrestricted introspection, coordinate control, or secret access.
/// None of them is accepted by the invoke shape; in particular no
/// caller-supplied PID, HWND, runtime id, selector, coordinate, approval
/// material, secret field, value, or fallback directive is accepted, so
/// callers can only name one server-allocated typed element plus its
/// expected tree generation and expected control type.
const INVOKE_WIDENING_FIELDS: &[&str] = &[
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
    "set_value",
    "text",
    "keys",
    "select",
    "toggle",
    "scroll",
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
    "pattern_invoke",
    "process_command",
    "window_id",
    "process_id",
    "control_type",
    "pattern",
    "enabled",
    "state",
];

impl PolicyEngine {
    /// Validate the single invoke shape. This layer is pure policy: it
    /// performs no OS access and no mutation, and it accepts no widening
    /// field of any kind.
    fn validate_invoke_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
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
                "uia invoke shapes do not accept a target field",
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
            .find(|key| INVOKE_WIDENING_FIELDS.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "uia invoke request must not carry authority-widening field: {key}; only element_id, expected_tree_generation, and expected_control_type are accepted"
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
        if !qdral_provider_uia::is_invoke_eligible_control_type(control_type) {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!("{shape} expected_control_type does not permit InvokePattern actuation"),
            ));
        }
        const ALLOWED: &[&str] = &[
            "element_id",
            "expected_tree_generation",
            "expected_control_type",
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
