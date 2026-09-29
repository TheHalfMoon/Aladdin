//! SG-000031 structured Windows UI Automation TogglePattern dispatch.
//!
//! This module dispatches the five retained SG-000027 observation shapes,
//! the retained SG-000028 `uia.element/invoke` shape, the retained
//! SG-000029 `uia.element/set_value` shape, the retained SG-000030
//! `uia.element/select` shape, plus the single SG-000031 actuation shape
//! `uia.element/toggle` against a process-lifetime typed identity registry
//! backed by the native adapter. Observation remains read-only with no
//! approval. Invoke, set_value, select, and toggle each require fresh
//! per-action SOFT approval with exact digest binding and immediate
//! pre-actuation stale-target revalidation. Every other UIA-like shape
//! returns `Ok(None)` so the caller fails closed through the STRONG gate
//! or the legacy denial.
//!
//! Identities are process-lifetime: a cotrad restart drops the registry,
//! so pre-restart identities fail closed as unknown rather than retargeting.

use cotra_approval::{ApprovalBroker, ApprovalPrompt, ConsumeExpectation};
use cotra_contracts::{FailureCode, RequestEnvelope};
use cotra_policy::{Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use cotra_provider_uia::{
    invoke_approval_digest, select_approval_digest, toggle_approval_digest, value_approval_digest,
    NativeAdapter, UiaRegistry,
};
use serde_json::Value;
use std::sync::{Mutex, OnceLock};

fn registry() -> &'static Mutex<UiaRegistry> {
    static REGISTRY: OnceLock<Mutex<UiaRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(UiaRegistry::new()))
}

fn locked_registry() -> Result<std::sync::MutexGuard<'static, UiaRegistry>, ProviderError> {
    registry()
        .lock()
        .map_err(|_| ProviderError::new(FailureCode::InternalError, "uia registry is unavailable"))
}

/// Dispatch the SG-000031 UIA shapes. Returns `Ok(None)` for non-UIA shapes
/// so the caller falls through to the browser, trust, Git, and legacy
/// dispatchers.
pub fn dispatch_uia(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Option<Value>, ProviderError> {
    if cotra_provider_uia::is_toggle_shape(&request.capability, &request.operation) {
        return toggle_with_approval(workspace, approval, request).map(Some);
    }
    if cotra_provider_uia::is_select_shape(&request.capability, &request.operation) {
        return select_with_approval(workspace, approval, request).map(Some);
    }
    if cotra_provider_uia::is_value_shape(&request.capability, &request.operation) {
        return set_value_with_approval(workspace, approval, request).map(Some);
    }
    if cotra_provider_uia::is_invoke_shape(&request.capability, &request.operation) {
        return invoke_with_approval(workspace, approval, request).map(Some);
    }
    if !cotra_provider_uia::is_allowed_uia_shape(&request.capability, &request.operation) {
        return Ok(None);
    }
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia observation shapes do not accept a target field",
        ));
    }
    let adapter = NativeAdapter::new();
    match (request.capability.as_str(), request.operation.as_str()) {
        ("uia.process", "observe") => {
            let process_id = required_string(request, "process_id")?;
            let expected = required_u64(request, "expected_process_generation")?;
            let guard = locked_registry()?;
            guard
                .observe_process(&process_id, expected, &workspace.id, POLICY_REVISION)
                .map(Some)
                .map_err(|error| ProviderError::new(error.code, error.message))
        }
        ("uia.window", "list") => {
            reject_uia_arguments(request, &[])?;
            let mut guard = locked_registry()?;
            guard
                .list_windows(&adapter, &workspace.id, POLICY_REVISION)
                .map(Some)
                .map_err(|error| ProviderError::new(error.code, error.message))
        }
        ("uia.window", "observe") => {
            let window_id = required_string(request, "window_id")?;
            let expected = required_u64(request, "expected_window_generation")?;
            let guard = locked_registry()?;
            guard
                .observe_window(&window_id, expected, &workspace.id, POLICY_REVISION)
                .map(Some)
                .map_err(|error| ProviderError::new(error.code, error.message))
        }
        ("uia.tree", "observe") => {
            let window_id = required_string(request, "window_id")?;
            let expected = required_u64(request, "expected_window_generation")?;
            let max_depth = optional_u64(request, "max_depth")?;
            let max_nodes = optional_u64(request, "max_nodes")?;
            let mut guard = locked_registry()?;
            guard
                .observe_tree(
                    &adapter,
                    &window_id,
                    expected,
                    max_depth,
                    max_nodes,
                    &workspace.id,
                    POLICY_REVISION,
                )
                .map(Some)
                .map_err(|error| ProviderError::new(error.code, error.message))
        }
        ("uia.element", "observe") => {
            let element_id = required_string(request, "element_id")?;
            let expected = required_u64(request, "expected_tree_generation")?;
            let guard = locked_registry()?;
            guard
                .observe_element(&element_id, expected, &workspace.id, POLICY_REVISION)
                .map(Some)
                .map_err(|error| ProviderError::new(error.code, error.message))
        }
        _ => Ok(None),
    }
}

fn invoke_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia invoke shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &[
            "element_id",
            "expected_tree_generation",
            "expected_control_type",
        ],
    )?;
    let element_id = required_string(request, "element_id")?;
    if !cotra_provider_uia::is_well_formed_element_id(&element_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia invoke element_id is malformed",
        ));
    }
    let expected_tree_generation = required_u64(request, "expected_tree_generation")?;
    let expected_control_type = required_string(request, "expected_control_type")?;
    if expected_control_type.is_empty() || expected_control_type.len() > 256 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia invoke expected_control_type is empty or too large",
        ));
    }
    if !cotra_provider_uia::is_invoke_eligible_control_type(&expected_control_type) {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "uia invoke actuates only Button, Hyperlink, MenuItem, and SplitButton elements",
        ));
    }
    let binding = {
        let guard = locked_registry()?;
        guard
            .invoke_binding(
                &element_id,
                expected_tree_generation,
                &expected_control_type,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = invoke_approval_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "invoke UIA element",
        element_id.clone(),
        format!(
            "process={} window={} element={} control={} tree={} action=invoke",
            binding.process_id,
            binding.window_id,
            binding.element_id,
            binding.control_type,
            binding.tree_generation,
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
    let adapter = NativeAdapter::new();
    let mut guard = locked_registry()?;
    let evidence = guard
        .invoke_element(
            &adapter,
            &element_id,
            expected_tree_generation,
            &expected_control_type,
            &workspace.id,
            POLICY_REVISION,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn set_value_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia set_value shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &[
            "element_id",
            "expected_tree_generation",
            "expected_control_type",
            "value",
        ],
    )?;
    let element_id = required_string(request, "element_id")?;
    if !cotra_provider_uia::is_well_formed_element_id(&element_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia set_value element_id is malformed",
        ));
    }
    let expected_tree_generation = required_u64(request, "expected_tree_generation")?;
    let expected_control_type = required_string(request, "expected_control_type")?;
    if expected_control_type.is_empty() || expected_control_type.len() > 256 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia set_value expected_control_type is empty or too large",
        ));
    }
    if !cotra_provider_uia::is_value_eligible_control_type(&expected_control_type) {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "uia set_value actuates only Edit, Document, and ComboBox elements",
        ));
    }
    let value = required_string(request, "value")?;
    if value.chars().count() > cotra_provider_uia::MAX_VALUE_CHARS {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia set_value value exceeds the bounded length",
        ));
    }
    if value.contains('\0') {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia set_value value contains a NUL byte",
        ));
    }
    let binding = {
        let guard = locked_registry()?;
        guard
            .value_binding(
                &element_id,
                expected_tree_generation,
                &expected_control_type,
                &value,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = value_approval_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "set UIA element value",
        element_id.clone(),
        format!(
            "process={} window={} element={} control={} tree={} action=set_value value_digest={}",
            binding.process_id,
            binding.window_id,
            binding.element_id,
            binding.control_type,
            binding.tree_generation,
            binding.value_digest,
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
    let adapter = NativeAdapter::new();
    let mut guard = locked_registry()?;
    let evidence = guard
        .set_value_element(
            &adapter,
            &element_id,
            expected_tree_generation,
            &expected_control_type,
            &value,
            &workspace.id,
            POLICY_REVISION,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn select_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia select shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &[
            "element_id",
            "expected_tree_generation",
            "expected_control_type",
            "expected_selected",
            "selected",
        ],
    )?;
    let element_id = required_string(request, "element_id")?;
    if !cotra_provider_uia::is_well_formed_element_id(&element_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia select element_id is malformed",
        ));
    }
    let expected_tree_generation = required_u64(request, "expected_tree_generation")?;
    let expected_control_type = required_string(request, "expected_control_type")?;
    if expected_control_type.is_empty() || expected_control_type.len() > 256 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia select expected_control_type is empty or too large",
        ));
    }
    if !cotra_provider_uia::is_select_eligible_control_type(&expected_control_type) {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "uia select actuates only ListItem, TreeItem, and TabItem elements",
        ));
    }
    let expected_selected = required_bool(request, "expected_selected")?;
    let selected = required_bool(request, "selected")?;
    let binding = {
        let guard = locked_registry()?;
        guard
            .select_binding(
                &element_id,
                expected_tree_generation,
                &expected_control_type,
                expected_selected,
                selected,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = select_approval_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "select UIA element",
        element_id.clone(),
        format!(
            "process={} window={} element={} control={} tree={} action=select expected_selected={} selected={}",
            binding.process_id,
            binding.window_id,
            binding.element_id,
            binding.control_type,
            binding.tree_generation,
            binding.expected_selected,
            binding.selected,
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
    let adapter = NativeAdapter::new();
    let mut guard = locked_registry()?;
    let evidence = guard
        .select_element(
            &adapter,
            &element_id,
            expected_tree_generation,
            &expected_control_type,
            expected_selected,
            selected,
            &workspace.id,
            POLICY_REVISION,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn toggle_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia toggle shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &[
            "element_id",
            "expected_tree_generation",
            "expected_control_type",
            "expected_toggled",
            "toggled",
        ],
    )?;
    let element_id = required_string(request, "element_id")?;
    if !cotra_provider_uia::is_well_formed_element_id(&element_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia toggle element_id is malformed",
        ));
    }
    let expected_tree_generation = required_u64(request, "expected_tree_generation")?;
    let expected_control_type = required_string(request, "expected_control_type")?;
    if expected_control_type.is_empty() || expected_control_type.len() > 256 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia toggle expected_control_type is empty or too large",
        ));
    }
    if !cotra_provider_uia::is_toggle_eligible_control_type(&expected_control_type) {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "uia toggle actuates only CheckBox and RadioButton elements",
        ));
    }
    let expected_toggled = required_bool(request, "expected_toggled")?;
    let toggled = required_bool(request, "toggled")?;
    let binding = {
        let guard = locked_registry()?;
        guard
            .toggle_binding(
                &element_id,
                expected_tree_generation,
                &expected_control_type,
                expected_toggled,
                toggled,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = toggle_approval_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "toggle UIA element",
        element_id.clone(),
        format!(
            "process={} window={} element={} control={} tree={} action=toggle expected_toggled={} toggled={}",
            binding.process_id,
            binding.window_id,
            binding.element_id,
            binding.control_type,
            binding.tree_generation,
            binding.expected_toggled,
            binding.toggled,
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
    let adapter = NativeAdapter::new();
    let mut guard = locked_registry()?;
    let evidence = guard
        .toggle_element(
            &adapter,
            &element_id,
            expected_tree_generation,
            &expected_control_type,
            expected_toggled,
            toggled,
            &workspace.id,
            POLICY_REVISION,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn required_string(request: &RequestEnvelope, name: &str) -> Result<String, ProviderError> {
    request
        .arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("uia shape requires arguments.{name} as a string"),
            )
        })
}

fn required_u64(request: &RequestEnvelope, name: &str) -> Result<u64, ProviderError> {
    request
        .arguments
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("uia shape requires arguments.{name} as an unsigned integer"),
            )
        })
}

fn required_bool(request: &RequestEnvelope, name: &str) -> Result<bool, ProviderError> {
    request
        .arguments
        .get(name)
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("uia shape requires arguments.{name} as a boolean"),
            )
        })
}

fn optional_u64(request: &RequestEnvelope, name: &str) -> Result<Option<u64>, ProviderError> {
    match request.arguments.get(name) {
        None => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("uia shape requires arguments.{name} as an unsigned integer"),
            )
        }),
    }
}

fn reject_uia_arguments(request: &RequestEnvelope, allowed: &[&str]) -> Result<(), ProviderError> {
    let arguments = request.arguments.as_object().ok_or_else(|| {
        ProviderError::new(
            FailureCode::InvalidRequest,
            "uia arguments must be an object",
        )
    })?;
    if let Some(key) = arguments
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            format!("uia shape does not accept argument field: {key}"),
        ));
    }
    Ok(())
}
