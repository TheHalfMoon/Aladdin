//! SG-000027 read-only Windows UI Automation observation dispatch.
//!
//! This module dispatches exactly the five authorized UIA observation
//! shapes against a process-lifetime typed identity registry backed by the
//! native adapter. Observation is read-only: no shape here requests
//! approval, mutates the computer, launches processes, injects input, or
//! touches the network. Every other UIA-like shape returns `Ok(None)` so
//! the caller fails closed through the STRONG gate or the legacy denial.
//!
//! Identities are process-lifetime: a cotrad restart drops the registry,
//! so pre-restart identities fail closed as unknown rather than retargeting.

use cotra_contracts::{FailureCode, RequestEnvelope};
use cotra_policy::{Workspace, POLICY_REVISION};
use cotra_provider_fs::ProviderError;
use cotra_provider_uia::{NativeAdapter, UiaRegistry};
use serde_json::Value;
use std::sync::{Mutex, OnceLock};

fn registry() -> &'static Mutex<UiaRegistry> {
    static REGISTRY: OnceLock<Mutex<UiaRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(UiaRegistry::new()))
}

fn locked_registry() -> Result<std::sync::MutexGuard<'static, UiaRegistry>, ProviderError> {
    registry().lock().map_err(|_| {
        ProviderError::new(
            FailureCode::InternalError,
            "uia observation registry is unavailable",
        )
    })
}

/// Dispatch the SG-000027 UIA observation shapes. Returns `Ok(None)` for
/// non-UIA shapes so the caller falls through to the browser, trust, Git,
/// and legacy dispatchers.
pub fn dispatch_uia(
    workspace: &Workspace,
    request: &RequestEnvelope,
) -> Result<Option<Value>, ProviderError> {
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

fn required_string(request: &RequestEnvelope, name: &str) -> Result<String, ProviderError> {
    request
        .arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("uia observation requires arguments.{name} as a string"),
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
                format!("uia observation requires arguments.{name} as an unsigned integer"),
            )
        })
}

fn optional_u64(request: &RequestEnvelope, name: &str) -> Result<Option<u64>, ProviderError> {
    match request.arguments.get(name) {
        None => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            ProviderError::new(
                FailureCode::InvalidRequest,
                format!("uia observation requires arguments.{name} as an unsigned integer"),
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
