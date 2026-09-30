//! SG-000037 human-interruption invalidation dispatch.
//!
//! This module dispatches the five retained SG-000027 observation shapes,
//! the retained SG-000028 `uia.element/invoke` shape, the retained
//! SG-000029 `uia.element/set_value` shape, the retained SG-000030
//! `uia.element/select` shape, the retained SG-000031 `uia.element/toggle`
//! shape, the retained SG-000032 `uia.element/scroll` shape, the retained
//! SG-000033 `uia.screenshot/capture` shape, the retained SG-000034
//! `uia.visual/propose` shape, the retained SG-000035
//! `uia.coordinates/propose` shape, plus the single SG-000036 bounded
//! execution shape `uia.input/execute` against a process-lifetime typed
//! identity registry backed by the native adapter. Observation remains
//! read-only with no approval. Invoke, set_value, select, toggle,
//! scroll, capture, propose, coordinate derivation, and bounded
//! execution each require fresh per-action SOFT approval with exact
//! digest binding and immediate pre-action stale-target revalidation.
//! Execution is click-only, confined to the exact owning window, gated
//! by an explicit single-use input lease that is consumed exactly once,
//! and bound to the SG-000037 monotonic human-interruption epoch: any
//! material human interaction after the lease grant revokes the lease,
//! the pre-interruption approval digest fails closed, interrupted actions
//! are never replayed, and a retry needs a fresh lease and fresh
//! approval. The interruption report path is reachable only through the
//! local session broker, never through MCP requests, and Cotra synthetic
//! execution never counts as human interruption. Every other UIA-like
//! shape returns `Ok(None)` so the caller fails closed through the
//! STRONG gate or the legacy denial.
//!
//! Identities are process-lifetime: a cotrad restart drops the registry,
//! so pre-restart identities fail closed as unknown rather than retargeting.

use cotra_approval::{ApprovalBroker, ApprovalPrompt, ConsumeExpectation};
use cotra_contracts::{FailureCode, RequestEnvelope};
use cotra_policy::{Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use cotra_provider_uia::{
    capture_approval_digest, coordinate_derivation_digest, execute_approval_digest,
    invoke_approval_digest, scroll_approval_digest, select_approval_digest, toggle_approval_digest,
    value_approval_digest, visual_proposal_digest, NativeAdapter, UiaRegistry, VisualRegion,
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

/// Dispatch the SG-000036 UIA shapes. Returns `Ok(None)` for non-UIA shapes
/// so the caller falls through to the browser, trust, Git, and legacy
/// dispatchers.
pub fn dispatch_uia(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Option<Value>, ProviderError> {
    if cotra_provider_uia::is_input_execute_shape(&request.capability, &request.operation) {
        return execute_with_approval(workspace, approval, request).map(Some);
    }
    if cotra_provider_uia::is_coordinate_propose_shape(&request.capability, &request.operation) {
        return derive_with_approval(workspace, approval, request).map(Some);
    }
    if cotra_provider_uia::is_visual_propose_shape(&request.capability, &request.operation) {
        return propose_with_approval(workspace, approval, request).map(Some);
    }
    if cotra_provider_uia::is_capture_shape(&request.capability, &request.operation) {
        return capture_with_approval(workspace, approval, request).map(Some);
    }
    if cotra_provider_uia::is_scroll_shape(&request.capability, &request.operation) {
        return scroll_with_approval(workspace, approval, request).map(Some);
    }
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

fn execute_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia bounded execution shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &["coord_id", "expected_derivation_generation", "operation"],
    )?;
    let coord_id = required_string(request, "coord_id")?;
    if !cotra_provider_uia::is_well_formed_coord_id(&coord_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia bounded execution coord_id is malformed",
        ));
    }
    let expected_derivation_generation = required_u64(request, "expected_derivation_generation")?;
    let operation = required_string(request, "operation")?;
    if !cotra_provider_uia::is_execute_operation(&operation) {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "uia bounded execution operation is fixed to click",
        ));
    }
    let binding = {
        let mut guard = locked_registry()?;
        guard
            .grant_input_lease(
                &coord_id,
                expected_derivation_generation,
                &operation,
                &workspace.id,
                POLICY_REVISION,
                cotra_approval::now_ms(),
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = execute_approval_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "execute bounded click",
        coord_id.clone(),
        format!(
            "coord={} derivation_generation={} window={} coordinates={},{} operation={} lease={} interruption_epoch={} action=execute",
            binding.coord_id,
            binding.derivation_generation,
            binding.window_id,
            binding.x,
            binding.y,
            binding.operation,
            binding.lease_id,
            binding.interruption_epoch,
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
        .execute_input(
            &adapter,
            &binding.lease_id,
            &workspace.id,
            POLICY_REVISION,
            cotra_approval::now_ms(),
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn derive_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia coordinate derivation shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(request, &["proposal_id", "expected_proposal_generation"])?;
    let proposal_id = required_string(request, "proposal_id")?;
    if !cotra_provider_uia::is_well_formed_proposal_id(&proposal_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia coordinate derivation proposal_id is malformed",
        ));
    }
    let expected_proposal_generation = required_u64(request, "expected_proposal_generation")?;
    let binding = {
        let guard = locked_registry()?;
        guard
            .derivation_binding(
                &proposal_id,
                expected_proposal_generation,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = coordinate_derivation_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "derive proposal coordinates",
        proposal_id.clone(),
        format!(
            "proposal={} proposal_generation={} frame={} coordinates={},{} action=derive",
            binding.proposal_id,
            binding.proposal_generation,
            binding.frame_id,
            binding.x,
            binding.y,
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
    let mut guard = locked_registry()?;
    let evidence = guard
        .derive_coordinates(
            &proposal_id,
            expected_proposal_generation,
            &workspace.id,
            POLICY_REVISION,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn propose_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia visual proposal shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &[
            "frame_id",
            "expected_capture_generation",
            "x",
            "y",
            "width",
            "height",
        ],
    )?;
    let frame_id = required_string(request, "frame_id")?;
    if !cotra_provider_uia::is_well_formed_frame_id(&frame_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia visual proposal frame_id is malformed",
        ));
    }
    let expected_capture_generation = required_u64(request, "expected_capture_generation")?;
    let x = required_u64(request, "x")?;
    let y = required_u64(request, "y")?;
    let width = required_u64(request, "width")?;
    if width == 0 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia visual proposal width must be nonzero",
        ));
    }
    let height = required_u64(request, "height")?;
    if height == 0 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia visual proposal height must be nonzero",
        ));
    }
    let region = VisualRegion {
        x,
        y,
        width,
        height,
    };
    let binding = {
        let guard = locked_registry()?;
        guard
            .proposal_binding(
                &frame_id,
                expected_capture_generation,
                region,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = visual_proposal_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "propose visual target",
        frame_id.clone(),
        format!(
            "frame={} capture_generation={} window={} region={},{},{}x{} action=propose",
            binding.frame_id,
            binding.capture_generation,
            binding.window_id,
            binding.region.x,
            binding.region.y,
            binding.region.width,
            binding.region.height,
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
    let mut guard = locked_registry()?;
    let evidence = guard
        .propose_visual_target(
            &frame_id,
            expected_capture_generation,
            region,
            &workspace.id,
            POLICY_REVISION,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn capture_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia screenshot capture shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &["window_id", "expected_window_generation", "scope"],
    )?;
    let window_id = required_string(request, "window_id")?;
    if !cotra_provider_uia::is_well_formed_window_id(&window_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia screenshot capture window_id is malformed",
        ));
    }
    let expected_window_generation = required_u64(request, "expected_window_generation")?;
    let scope = required_string(request, "scope")?;
    if !cotra_provider_uia::is_capture_scope(&scope) {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "uia screenshot capture scope is fixed to target-window; monitor scope, desktop scope, and caller-selected regions are denied",
        ));
    }
    let binding = {
        let guard = locked_registry()?;
        guard
            .capture_binding(
                &window_id,
                expected_window_generation,
                &scope,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = capture_approval_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "capture window screenshot",
        window_id.clone(),
        format!(
            "process={} window={} window_generation={} scope={} action=capture",
            binding.process_id, binding.window_id, binding.window_generation, binding.scope,
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
        .capture_window(
            &adapter,
            &window_id,
            expected_window_generation,
            &scope,
            &workspace.id,
            POLICY_REVISION,
        )
        .map_err(|error| ProviderError::new(error.code, error.message))?;
    let mut stamped = evidence;
    stamped["approval_record"] = Value::String(token.record_id);
    Ok(stamped)
}

fn scroll_with_approval(
    workspace: &Workspace,
    approval: &impl ApprovalBroker,
    request: &RequestEnvelope,
) -> Result<Value, ProviderError> {
    if request.target.is_some() {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia scroll shapes do not accept a target field",
        ));
    }
    reject_uia_arguments(
        request,
        &[
            "element_id",
            "expected_tree_generation",
            "expected_control_type",
            "direction",
            "amount",
            "expected_horizontal_percent",
            "expected_vertical_percent",
        ],
    )?;
    let element_id = required_string(request, "element_id")?;
    if !cotra_provider_uia::is_well_formed_element_id(&element_id) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia scroll element_id is malformed",
        ));
    }
    let expected_tree_generation = required_u64(request, "expected_tree_generation")?;
    let expected_control_type = required_string(request, "expected_control_type")?;
    if expected_control_type.is_empty() || expected_control_type.len() > 256 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia scroll expected_control_type is empty or too large",
        ));
    }
    if !cotra_provider_uia::is_scroll_eligible_control_type(&expected_control_type) {
        return Err(ProviderError::new(
            FailureCode::CapabilityDenied,
            "uia scroll actuates only ScrollBar, Pane, List, and Tree elements",
        ));
    }
    let direction = required_string(request, "direction")?;
    if !cotra_provider_uia::is_scroll_direction(&direction) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia scroll direction must be one of up, down, left, or right",
        ));
    }
    let amount = required_u64(request, "amount")?;
    if !(1..=cotra_provider_uia::MAX_SCROLL_AMOUNT).contains(&amount) {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia scroll amount must be between 1 and 100 inclusive",
        ));
    }
    let expected_horizontal_percent = required_u64(request, "expected_horizontal_percent")?;
    if expected_horizontal_percent > 100 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia scroll expected_horizontal_percent must be between 0 and 100 inclusive",
        ));
    }
    let expected_vertical_percent = required_u64(request, "expected_vertical_percent")?;
    if expected_vertical_percent > 100 {
        return Err(ProviderError::new(
            FailureCode::InvalidRequest,
            "uia scroll expected_vertical_percent must be between 0 and 100 inclusive",
        ));
    }
    let binding = {
        let guard = locked_registry()?;
        guard
            .scroll_binding(
                &element_id,
                expected_tree_generation,
                &expected_control_type,
                expected_horizontal_percent,
                expected_vertical_percent,
                &direction,
                amount,
                &workspace.id,
                POLICY_REVISION,
            )
            .map_err(|error| ProviderError::new(error.code, error.message))?
    };
    let digest = scroll_approval_digest(&workspace.id, POLICY_REVISION, &binding);
    let prompt = ApprovalPrompt::new(
        workspace.id.clone(),
        POLICY_REVISION,
        "scroll UIA element",
        element_id.clone(),
        format!(
            "process={} window={} element={} control={} tree={} action=scroll direction={} amount={} expected_h={} expected_v={}",
            binding.process_id,
            binding.window_id,
            binding.element_id,
            binding.control_type,
            binding.tree_generation,
            binding.direction,
            binding.amount,
            binding.expected_horizontal_percent,
            binding.expected_vertical_percent,
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
        .scroll_element(
            &adapter,
            &element_id,
            expected_tree_generation,
            &expected_control_type,
            expected_horizontal_percent,
            expected_vertical_percent,
            &direction,
            amount,
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
