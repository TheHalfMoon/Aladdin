#[path = "sg000025.rs"]
mod sg000025_legacy;

pub use qdral_approval::ApprovalClass;
pub use sg000025_legacy::{
    validate_relative_target, FetchDestination, PolicyDecision, PolicyError, PushDestination,
    Workspace,
};

use qdral_contracts::{FailureCode, RequestEnvelope};
use serde_json::Value;

pub const POLICY_REVISION: &str = "sg-000026-v1";

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    legacy: sg000025_legacy::PolicyEngine,
}

impl PolicyEngine {
    pub fn new(workspaces: Vec<Workspace>) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000025_legacy::PolicyEngine::new(workspaces)?,
        })
    }

    pub fn with_destinations(
        workspaces: Vec<Workspace>,
        fetch_destinations: Vec<FetchDestination>,
        push_destinations: Vec<PushDestination>,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            legacy: sg000025_legacy::PolicyEngine::with_destinations(
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

    /// Authorize one request. Upload shapes are validated here so the upload
    /// source and target policy can never be reached through a predecessor
    /// engine. Every other browser shape retains its SG-000025 validation and
    /// every remaining shape falls through unchanged.
    pub fn authorize(&self, request: &RequestEnvelope) -> Result<PolicyDecision, PolicyError> {
        if is_upload_shape(request) {
            self.validate_upload_shape(request)?;
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
        if qdral_provider_browser::is_denied_browser_shape(&request.capability, &request.operation)
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "capability/operation is denied by {POLICY_REVISION}: {}/{}; SG-000026 authorizes only browser.profile/status, browser.destination/validate, browser.page/open, browser.navigation/preview, browser.navigation/navigate, read-only browser.snapshot/observe, structured browser.dom/click and browser.dom/fill, scoped browser.download/preview and browser.download/download into the approved workspace download root, and scoped browser.upload/preview and browser.upload/submit of a recorded approved download artifact to one typed file-input node, and generic file read, directory upload, multiple-file upload, page byte transfer, downloaded-file execution, file opening, archive extraction, personal-profile access, debugging, scripting, and network egress remain absent",
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
    if is_upload_shape_str(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_browser::is_allowed_browser_shape(capability, operation) {
        return ApprovalClass::Soft;
    }
    if qdral_provider_browser::is_denied_browser_shape(capability, operation) {
        return ApprovalClass::Strong;
    }
    sg000025_legacy::approval_class_for(capability, operation)
}

fn is_upload_shape(request: &RequestEnvelope) -> bool {
    is_upload_shape_str(&request.capability, &request.operation)
}

fn is_upload_shape_str(capability: &str, operation: &str) -> bool {
    matches!(
        (capability, operation),
        ("browser.upload", "preview") | ("browser.upload", "submit")
    )
}
/// Fields that would widen upload authority into generic filesystem read or
/// into page byte transfer. None of them is accepted by any upload shape; in
/// particular no path field of any kind is accepted, so a caller can never name
/// the file to read.
const UPLOAD_WIDENING_FIELDS: &[&str] = &[
    "path",
    "paths",
    "file",
    "files",
    "file_path",
    "source_path",
    "relative_path",
    "absolute_path",
    "destination",
    "directory",
    "directories",
    "root",
    "download_root",
    "destination_root",
    "drive",
    "unc",
    "content",
    "content_base64",
    "bytes",
    "data",
    "read",
    "recursive",
    "glob",
    "wildcard",
    "profile_root",
    "argv",
    "executable",
    "script",
    "javascript",
    "command",
    "personal",
    "credentials",
    "cookies",
    "passwords",
    "session",
    "extensions",
    "devtools",
    "cdp",
    "approval",
    "token",
    "nonce",
    "digest",
    "selector",
    "coordinate",
    "x",
    "y",
    "execute",
    "open",
    "extract",
    "spawn",
    "shell",
    "run",
    "submit_form",
    "transfer",
];

impl PolicyEngine {
    /// Validate one upload shape. This layer is pure policy: it performs no
    /// filesystem access, no network activity, and no mutation, and it accepts
    /// no path field of any kind.
    fn validate_upload_shape(&self, request: &RequestEnvelope) -> Result<(), PolicyError> {
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
                "browser upload shapes do not accept a target field",
            ));
        }
        let arguments = request.arguments.as_object().ok_or_else(|| {
            policy_error(
                FailureCode::InvalidRequest,
                "browser arguments must be an object",
            )
        })?;
        let shape = format!("{}/{}", request.capability, request.operation);
        if let Some(key) = arguments
            .keys()
            .find(|key| UPLOAD_WIDENING_FIELDS.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::CapabilityDenied,
                format!(
                    "browser upload request must not carry authority-widening field: {key}; no path, file, content, directory, page-transfer, or execution field is accepted, and an upload source can only be named by a recorded approved download identity"
                ),
            ));
        }
        let allowed: &[&str] = match request.operation.as_str() {
            "preview" => {
                validate_upload_preview_arguments(&shape, arguments)?;
                &[
                    "page_id",
                    "expected_origin",
                    "expected_generation",
                    "expected_document_generation",
                    "node_id",
                    "expected_role",
                    "expected_input_type",
                    "expected_state",
                    "expected_trust_revision",
                    "artifact_source_id",
                ]
            }
            "submit" => {
                validate_upload_submit_arguments(&shape, arguments)?;
                &[
                    "page_id",
                    "expected_origin",
                    "expected_generation",
                    "expected_document_generation",
                    "source_id",
                ]
            }
            _ => {
                return Err(policy_error(
                    FailureCode::CapabilityDenied,
                    format!("{shape} is not an authorized upload verb"),
                ))
            }
        };
        if let Some(key) = arguments
            .keys()
            .find(|key| !allowed.contains(&key.as_str()))
        {
            return Err(policy_error(
                FailureCode::InvalidRequest,
                format!("{shape} does not accept argument field: {key}"),
            ));
        }
        Ok(())
    }
}
fn validate_upload_preview_arguments(
    shape: &str,
    arguments: &serde_json::Map<String, Value>,
) -> Result<(), PolicyError> {
    let page_id = required_shape_string(shape, arguments, "page_id")?;
    if !page_id.starts_with(qdral_provider_browser::PAGE_ID_PREFIX) {
        return Err(policy_error(
            FailureCode::InvalidRequest,
            format!("{shape} page_id is malformed"),
        ));
    }
    required_shape_string(shape, arguments, "expected_origin")?;
    required_shape_u64(shape, arguments, "expected_generation")?;
    required_shape_u64(shape, arguments, "expected_document_generation")?;
    let node_id = required_shape_string(shape, arguments, "node_id")?;
    if !node_id.starts_with(qdral_provider_browser::SNAPSHOT_NODE_PREFIX) {
        return Err(policy_error(
            FailureCode::InvalidRequest,
            format!("{shape} node_id is malformed"),
        ));
    }
    let role = required_shape_string(shape, arguments, "expected_role")?;
    if role != qdral_provider_browser::UPLOAD_NODE_ROLE {
        return Err(policy_error(
            FailureCode::CapabilityDenied,
            format!("{shape} expected_role '{role}' is not an authorized upload target role"),
        ));
    }
    let input_type = required_shape_string(shape, arguments, "expected_input_type")?;
    if !input_type.eq_ignore_ascii_case(qdral_provider_browser::UPLOAD_INPUT_TYPE) {
        return Err(policy_error(
            FailureCode::CapabilityDenied,
            format!(
                "{shape} expected_input_type '{input_type}' is not an authorized upload target input type"
            ),
        ));
    }
    let state = required_shape_string(shape, arguments, "expected_state")?;
    if state != qdral_provider_browser::ENABLED_NODE_STATE {
        return Err(policy_error(
            FailureCode::CapabilityDenied,
            format!(
                "{shape} expected_state '{state}' is not an authorized upload target state; only enabled nodes accept uploads"
            ),
        ));
    }
    required_shape_u64(shape, arguments, "expected_trust_revision")?;
    let artifact = required_shape_string(shape, arguments, "artifact_source_id")?;
    if !qdral_provider_browser::is_well_formed_download_source_id(artifact) {
        return Err(policy_error(
            FailureCode::InvalidRequest,
            format!("{shape} artifact_source_id is malformed"),
        ));
    }
    Ok(())
}

fn validate_upload_submit_arguments(
    shape: &str,
    arguments: &serde_json::Map<String, Value>,
) -> Result<(), PolicyError> {
    let page_id = required_shape_string(shape, arguments, "page_id")?;
    if !page_id.starts_with(qdral_provider_browser::PAGE_ID_PREFIX) {
        return Err(policy_error(
            FailureCode::InvalidRequest,
            format!("{shape} page_id is malformed"),
        ));
    }
    required_shape_string(shape, arguments, "expected_origin")?;
    required_shape_u64(shape, arguments, "expected_generation")?;
    required_shape_u64(shape, arguments, "expected_document_generation")?;
    let source_id = required_shape_string(shape, arguments, "source_id")?;
    if !qdral_provider_browser::is_well_formed_upload_source_id(source_id) {
        return Err(policy_error(
            FailureCode::InvalidRequest,
            format!("{shape} source_id is malformed"),
        ));
    }
    Ok(())
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
