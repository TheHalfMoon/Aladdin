//! SG-000027 read-only Windows UI Automation observation provider.
//!
//! This crate implements the narrow P09 observation authority authorized by
//! the SG-000027 SpecGrain: server-derived process identity, typed window
//! identity, typed element identity, bounded read-only tree observation,
//! stale-identity fail-closed behavior, protected Cotra approval-surface
//! exclusion, and password and secret redaction.
//!
//! No actuation authority exists in this crate. Invoke, click, value setting,
//! text entry, select, toggle, scroll, focus, keyboard input, mouse input,
//! `SendInput`, coordinate requests, screenshots, clipboard access, network
//! egress, and elevation have no function here and must fail closed in the
//! policy and dispatch layers. A missing UIA element is reported as stale or
//! denied and must never become coordinate authority; coordinate fallback is
//! P10 work.
//!
//! Identity model:
//! - process identities are server-allocated as `uia-proc-` plus 16 lowercase
//!   hex characters derived from the PID and the executable digest, and they
//!   carry the process generation so PID reuse and restarts fail closed;
//! - window identities are server-allocated as `uia-win-` plus 16 lowercase
//!   hex characters derived from the owning process identity and the window
//!   handle, and they carry the window generation so destroyed or reused
//!   window handles fail closed;
//! - element identities are server-allocated as `uia-el-` plus 16 lowercase
//!   hex characters derived from the owning window identity and the UIA
//!   runtime identity, and they carry the tree generation so disappeared,
//!   replaced, or role-changed elements fail closed.
//!
//! Windows reality: the native adapter proves real process identity against
//! Windows APIs (process creation time as the start generation and the real
//! session identity) on Windows. Live desktop window and tree enumeration
//! requires an interactive session broker, which is successor work, so the
//! native adapter reports the desktop as unavailable instead of fabricating
//! windows. Deterministic observation, stale-protection, redaction, and
//! protected-surface behavior is proven through the injected fake adapter.
//! These limits are recorded honestly and must not be read as interactive
//! desktop evidence.

use cotra_contracts::FailureCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const UIA_SCHEMA: &str = "cotra-uia-observation-v1";
pub const INVOKE_SCHEMA: &str = "cotra-uia-invoke-v1";
pub const VALUE_SCHEMA: &str = "cotra-uia-value-v1";
pub const SELECT_SCHEMA: &str = "cotra-uia-select-v1";
pub const PROCESS_ID_PREFIX: &str = "uia-proc-";
pub const WINDOW_ID_PREFIX: &str = "uia-win-";
pub const ELEMENT_ID_PREFIX: &str = "uia-el-";
pub const POLICY_REVISION: &str = "sg-000027-v1";
pub const INVOKE_POLICY_REVISION: &str = "sg-000028-v1";

pub const MAX_WINDOWS: usize = 64;
pub const MAX_TREE_DEPTH: usize = 8;
pub const MAX_TREE_NODES: usize = 256;
pub const MAX_STRING_CHARS: usize = 256;
pub const MAX_RESPONSE_BYTES: usize = 64 * 1024;

const ID_HEX_CHARS: usize = 16;

/// Window classes and titles that identify protected Cotra approval and
/// security surfaces. Observation must never target these surfaces, because
/// the agent must not be able to inspect trusted approval material in a way
/// that undermines STRONG presence, approval decisions, trust changes,
/// emergency revoke, or protected credential handling.
const PROTECTED_WINDOW_MARKERS: &[&str] = &[
    "cotraapprove",
    "cotra approval",
    "cotra trust",
    "cotra emergency revoke",
];

/// Substrings that mark a control as password or secret bearing. Matching is
/// ASCII case-insensitive and applies to the control type, the automation id,
/// and the accessible name.
const PASSWORD_MARKERS: &[&str] = &["password", "passwd", "secret", "token"];

/// The only UIA capability and operation shapes SG-000027 authorizes. Every
/// other UIA-like shape must fail closed.
pub fn is_allowed_uia_shape(capability: &str, operation: &str) -> bool {
    matches!(
        (capability, operation),
        ("uia.process", "observe")
            | ("uia.window", "list")
            | ("uia.window", "observe")
            | ("uia.tree", "observe")
            | ("uia.element", "observe")
    )
}

/// Typed denial catalog for UIA shapes that remain unauthorized. Actuation,
/// synthetic input, coordinates, screenshots, clipboard, network, and
/// elevation shapes are denied here so policy can map them to the STRONG
/// gate and dispatch can fail closed without reaching the registry.
///
/// NOTE (SG-000028 successor): `uia.element/invoke` is lawfully authorized
/// by the SG-000028 successor grain and is therefore no longer in this
/// denied set for current-tree authority. The frozen SG-000027 qualified
/// head recorded this shape as denied; current-tree authority records the
/// successor delta.
///
/// NOTE (SG-000029 successor): `uia.element/set_value` is lawfully
/// authorized by the SG-000029 successor grain and is therefore no longer
/// in this denied set for current-tree authority. Select, toggle, scroll,
/// focus, keyboard, mouse, coordinates, screenshots, clipboard, network,
/// and elevation remain denied in every successor grain.
///
/// NOTE (SG-000030 successor): `uia.element/select` is lawfully authorized
/// by the SG-000030 successor grain and is therefore no longer in this
/// denied set for current-tree authority. The frozen SG-000027 qualified
/// head recorded this shape as denied; current-tree authority records the
/// successor delta. Toggle, scroll, focus, keyboard, mouse, coordinates,
/// screenshots, clipboard, network, and elevation remain denied in every
/// successor grain.
pub const DENIED_UIA_SHAPES: &[(&str, &str)] = &[
    ("uia.element", "click"),
    ("uia.element", "toggle"),
    ("uia.element", "scroll"),
    ("uia.element", "focus"),
    ("uia.window", "focus"),
    ("uia.window", "close"),
    ("uia.input", "keyboard"),
    ("uia.input", "mouse"),
    ("uia.input", "sendinput"),
    ("uia.coordinates", "request"),
    ("uia.screenshot", "capture"),
    ("uia.clipboard", "read"),
    ("uia.clipboard", "write"),
    ("uia.network", "fetch"),
    ("uia.process", "spawn"),
    ("uia.process", "inject"),
    ("uia.process", "terminate"),
    ("uia.elevation", "request"),
];

/// The single SG-000028 actuation shape. Observation shapes remain authorized
/// through `is_allowed_uia_shape`; invoke is authorized separately so the
/// SG-000027 frozen policy meaning is preserved while current-tree authority
/// records the successor delta.
pub fn is_invoke_shape(capability: &str, operation: &str) -> bool {
    matches!((capability, operation), ("uia.element", "invoke"))
}

/// Control types eligible for InvokePattern actuation. All other control
/// types must fail closed as denied, including value-capable, selection
/// capable, toggle capable, scroll capable, password, and secret controls.
pub const INVOKE_ELIGIBLE_CONTROL_TYPES: &[&str] =
    &["Button", "Hyperlink", "MenuItem", "SplitButton"];

/// The required UIA pattern name for invoke targets.
pub const INVOKE_PATTERN_NAME: &str = "Invoke";

pub fn is_invoke_eligible_control_type(control_type: &str) -> bool {
    INVOKE_ELIGIBLE_CONTROL_TYPES.contains(&control_type)
}

/// The single SG-000029 value actuation shape. Invoke remains authorized
/// through `is_invoke_shape`; value is authorized separately so frozen
/// policy meanings are preserved while current-tree authority records the
/// successor delta.
pub fn is_value_shape(capability: &str, operation: &str) -> bool {
    matches!((capability, operation), ("uia.element", "set_value"))
}

/// Control types eligible for ValuePattern actuation. All other control
/// types must fail closed as denied, including invoke-capable, selection
/// capable, toggle capable, scroll capable, password, and secret controls.
pub const VALUE_ELIGIBLE_CONTROL_TYPES: &[&str] = &["Edit", "Document", "ComboBox"];

/// The required UIA pattern name for value targets.
pub const VALUE_PATTERN_NAME: &str = "Value";

/// Maximum value length in characters for structured set_value. Oversized
/// values fail closed without silent truncation.
pub const MAX_VALUE_CHARS: usize = 1024;

pub fn is_value_eligible_control_type(control_type: &str) -> bool {
    VALUE_ELIGIBLE_CONTROL_TYPES.contains(&control_type)
}

/// The single SG-000030 select actuation shape. Invoke and value remain
/// authorized through their own predicates; select is authorized separately
/// so frozen policy meanings are preserved while current-tree authority
/// records the successor delta.
pub fn is_select_shape(capability: &str, operation: &str) -> bool {
    matches!((capability, operation), ("uia.element", "select"))
}

/// Control types eligible for SelectionItem actuation. All other control
/// types must fail closed as denied, including invoke-capable, value
/// capable, toggle capable, scroll capable, password, and secret controls.
pub const SELECT_ELIGIBLE_CONTROL_TYPES: &[&str] = &["ListItem", "TreeItem", "TabItem"];

/// The required UIA pattern name for select targets.
pub const SELECT_PATTERN_NAME: &str = "SelectionItem";

pub fn is_select_eligible_control_type(control_type: &str) -> bool {
    SELECT_ELIGIBLE_CONTROL_TYPES.contains(&control_type)
}

/// SHA-256 hex digest of one requested value, used in approval digests and
/// evidence so raw value bytes never enter approval prompts beyond the
/// bounded request itself and never enter evidence at all.
pub fn value_content_digest(value: &str) -> String {
    digest_hex(&format!("cotra-uia-value-v1|{value}"), 64)
}

pub fn is_denied_uia_shape(capability: &str, operation: &str) -> bool {
    DENIED_UIA_SHAPES
        .iter()
        .any(|(denied_capability, denied_operation)| {
            *denied_capability == capability && *denied_operation == operation
        })
}

#[derive(Debug, Clone)]
pub struct UiaError {
    pub code: FailureCode,
    pub message: String,
}

impl UiaError {
    pub fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub fn is_well_formed_process_id(value: &str) -> bool {
    is_well_formed_typed_id(value, PROCESS_ID_PREFIX)
}

pub fn is_well_formed_window_id(value: &str) -> bool {
    is_well_formed_typed_id(value, WINDOW_ID_PREFIX)
}

pub fn is_well_formed_element_id(value: &str) -> bool {
    is_well_formed_typed_id(value, ELEMENT_ID_PREFIX)
}

fn is_well_formed_typed_id(value: &str, prefix: &str) -> bool {
    value.len() == prefix.len() + ID_HEX_CHARS
        && value.starts_with(prefix)
        && value[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn digest_hex(material: &str, chars: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    hex.chars().take(chars).collect()
}

fn allocate_process_id(pid: u32, exe_id: &str) -> String {
    format!(
        "{PROCESS_ID_PREFIX}{}",
        digest_hex(&format!("{UIA_SCHEMA}|proc|{pid}|{exe_id}"), ID_HEX_CHARS)
    )
}

fn allocate_window_id(process_id: &str, hwnd: u64) -> String {
    format!(
        "{WINDOW_ID_PREFIX}{}",
        digest_hex(
            &format!("{UIA_SCHEMA}|win|{process_id}|{hwnd}"),
            ID_HEX_CHARS
        )
    )
}

fn allocate_element_id(window_id: &str, runtime_id: &str) -> String {
    format!(
        "{ELEMENT_ID_PREFIX}{}",
        digest_hex(
            &format!("{UIA_SCHEMA}|el|{window_id}|{runtime_id}"),
            ID_HEX_CHARS
        )
    )
}

/// A process as reported by an adapter. Adapters never allocate authority;
/// they only report OS-visible facts that the registry binds into typed
/// identities.
#[derive(Debug, Clone)]
pub struct NativeProcess {
    pub pid: u32,
    pub exe_name: String,
    pub exe_id: String,
    pub session_id: u32,
    pub session_verified: bool,
    pub start_generation: u64,
    pub generation_source: &'static str,
}

/// A top-level window as reported by an adapter.
#[derive(Debug, Clone)]
pub struct NativeWindow {
    pub hwnd: u64,
    pub title: String,
    pub class: String,
    pub visible: bool,
    /// Adapter-provided replacement marker. The fake adapter changes this
    /// value to simulate a destroyed and recreated window behind a reused
    /// handle. The native adapter derives it from stable window metadata.
    pub window_nonce: u64,
}

/// One UIA element as reported by an adapter, with already-nested children.
#[derive(Debug, Clone)]
pub struct NativeElement {
    pub runtime_id: String,
    pub control_type: String,
    pub automation_id: String,
    pub name: String,
    pub enabled: bool,
    pub selected: bool,
    pub toggled: bool,
    pub patterns: Vec<String>,
    pub value: Option<String>,
    pub value_is_password: bool,
    pub children: Vec<NativeElement>,
}

/// The adapter boundary. Policy logic, identity allocation, stale checks,
/// redaction, bounds, and protected-surface exclusion live in the registry,
/// so adapters stay small and deterministic tests never need a live desktop.
pub trait UiaAdapter {
    fn list_processes(&self) -> Result<Vec<NativeProcess>, UiaError>;
    fn list_windows(&self, pid: u32) -> Result<Vec<NativeWindow>, UiaError>;
    fn read_tree(&self, hwnd: u64) -> Result<Vec<NativeElement>, UiaError>;
    /// Perform one structured InvokePattern actuation against the live
    /// element identified by window handle and UIA runtime identity. The
    /// registry revalidates every binding before calling this method, so
    /// adapters must not retarget, fall back to coordinates, or synthesize
    /// generic input. The default implementation fails closed as
    /// unavailable, which the native adapter uses until an interactive
    /// session broker exists.
    fn invoke_element(&self, _hwnd: u64, _runtime_id: &str) -> Result<(), UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "structured UIA invoke requires an interactive session broker and is unavailable in this context",
        ))
    }
    /// Perform one structured ValuePattern write against the live element
    /// identified by window handle and UIA runtime identity. The registry
    /// revalidates every binding before calling this method, so adapters
    /// must not retarget, fall back to keyboard, clipboard, or coordinates,
    /// or synthesize generic input. The default implementation fails closed
    /// as unavailable, which the native adapter uses until an interactive
    /// session broker exists.
    fn set_value_element(
        &self,
        _hwnd: u64,
        _runtime_id: &str,
        _value: &str,
    ) -> Result<(), UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "structured UIA set_value requires an interactive session broker and is unavailable in this context",
        ))
    }
    /// Perform one structured SelectionItem actuation against the live
    /// element identified by window handle and UIA runtime identity. The
    /// registry revalidates every binding before calling this method, so
    /// adapters must not retarget, fall back to mouse, keyboard, or
    /// coordinates, or synthesize generic input. The default implementation
    /// fails closed as unavailable, which the native adapter uses until an
    /// interactive session broker exists.
    fn select_element(
        &self,
        _hwnd: u64,
        _runtime_id: &str,
        _selected: bool,
    ) -> Result<(), UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "structured UIA select requires an interactive session broker and is unavailable in this context",
        ))
    }
}

/// The native adapter. On Windows it reports the real current process
/// identity against Windows APIs: the real PID, the real executable digest,
/// the real session identity, and the real process creation time as the
/// start generation. Live desktop enumeration requires an interactive
/// session broker and is reported as unavailable instead of fabricated.
pub struct NativeAdapter;

impl NativeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn current_process() -> NativeProcess {
        let pid = std::process::id();
        let (exe_name, exe_id) = match std::env::current_exe() {
            Ok(path) => {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown")
                    .to_owned();
                let normalized = path.to_string_lossy().to_lowercase();
                let digest = digest_hex(&format!("{UIA_SCHEMA}|exe|{normalized}"), ID_HEX_CHARS);
                (name, digest)
            }
            Err(_) => ("unknown".to_owned(), "0".repeat(ID_HEX_CHARS)),
        };
        #[cfg(windows)]
        let (session_id, session_verified, start_generation, generation_source) =
            windows_process_facts(pid);
        #[cfg(not(windows))]
        let (session_id, session_verified, start_generation, generation_source) =
            (0u32, false, 0u64, "std-fallback");
        NativeProcess {
            pid,
            exe_name,
            exe_id,
            session_id,
            session_verified,
            start_generation,
            generation_source,
        }
    }
}

impl Default for NativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl UiaAdapter for NativeAdapter {
    fn list_processes(&self) -> Result<Vec<NativeProcess>, UiaError> {
        Ok(vec![Self::current_process()])
    }

    fn list_windows(&self, _pid: u32) -> Result<Vec<NativeWindow>, UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "live desktop window enumeration requires an interactive session broker and is unavailable in this context",
        ))
    }

    fn read_tree(&self, _hwnd: u64) -> Result<Vec<NativeElement>, UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "live desktop tree enumeration requires an interactive session broker and is unavailable in this context",
        ))
    }
}

/// Read real process facts against Windows APIs. Creation time becomes the
/// start generation so PID reuse and restarts fail closed. Any failure
/// degrades to explicit unverified markers rather than fabricated facts.
#[cfg(windows)]
fn windows_process_facts(pid: u32) -> (u32, bool, u64, &'static str) {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let (session_id, session_verified) = {
        let mut session = 0u32;
        let ok = unsafe { ProcessIdToSessionId(pid, &mut session) }.is_ok();
        if ok {
            (session, true)
        } else {
            (0u32, false)
        }
    };
    let (start_generation, generation_source) = unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => {
                let handle: HANDLE = handle;
                let mut creation = Default::default();
                let mut exit = Default::default();
                let mut kernel = Default::default();
                let mut user = Default::default();
                let ok = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user)
                    .is_ok();
                let _ = CloseHandle(handle);
                if ok {
                    let file_time =
                        ((creation.dwHighDateTime as u64) << 32) | (creation.dwLowDateTime as u64);
                    (file_time, "win32-creation-time")
                } else {
                    (0u64, "win32-unavailable")
                }
            }
            Err(_) => (0u64, "win32-unavailable"),
        }
    };
    (
        session_id,
        session_verified,
        start_generation,
        generation_source,
    )
}

#[derive(Debug, Clone)]
struct ProcessRecord {
    process_id: String,
    pid: u32,
    exe_name: String,
    exe_id: String,
    session_id: u32,
    session_verified: bool,
    process_generation: u64,
    generation_source: &'static str,
    workspace_id: String,
    policy_revision: String,
    superseded: bool,
}

#[derive(Debug, Clone)]
struct WindowRecord {
    window_id: String,
    process_id: String,
    hwnd: u64,
    title: String,
    class: String,
    visible: bool,
    window_nonce: u64,
    window_generation: u64,
    tree_generation: u64,
    workspace_id: String,
    policy_revision: String,
}

#[derive(Debug, Clone)]
struct ElementRecord {
    element_id: String,
    window_id: String,
    runtime_id: String,
    control_type: String,
    automation_id: String,
    name: String,
    enabled: bool,
    selected: bool,
    toggled: bool,
    patterns: Vec<String>,
    value: Option<String>,
    value_is_password: bool,
    redacted: bool,
    tree_generation: u64,
}

/// Server-side typed identity registry. Identities are allocated here and
/// can never be named by callers; every observation revalidates the exact
/// binding set and fails closed on drift.
#[derive(Debug, Default)]
pub struct UiaRegistry {
    processes: HashMap<String, ProcessRecord>,
    windows: HashMap<String, WindowRecord>,
    elements: HashMap<String, ElementRecord>,
    window_counter: u64,
    tree_counter: u64,
}

impl UiaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Register or refresh one native process. A changed start generation
    /// for a known PID marks the previous record superseded so the old
    /// identity fails closed instead of silently retargeting. Returns the
    /// server-allocated process identity and its generation.
    pub fn register_process(
        &mut self,
        native: &NativeProcess,
        workspace_id: &str,
        policy_revision: &str,
    ) -> (String, u64) {
        let process_id = allocate_process_id(native.pid, &native.exe_id);
        match self.processes.get(&process_id) {
            Some(existing)
                if existing.process_generation == native.start_generation
                    && existing.exe_id == native.exe_id =>
            {
                (process_id, existing.process_generation)
            }
            Some(_) => {
                if let Some(record) = self.processes.get_mut(&process_id) {
                    record.superseded = true;
                }
                let record = ProcessRecord {
                    process_id: process_id.clone(),
                    pid: native.pid,
                    exe_name: truncate_owned(&native.exe_name, MAX_STRING_CHARS).0,
                    exe_id: native.exe_id.clone(),
                    session_id: native.session_id,
                    session_verified: native.session_verified,
                    process_generation: native.start_generation,
                    generation_source: native.generation_source,
                    workspace_id: workspace_id.to_owned(),
                    policy_revision: policy_revision.to_owned(),
                    superseded: false,
                };
                self.processes.insert(process_id.clone(), record);
                (process_id, native.start_generation)
            }
            None => {
                let record = ProcessRecord {
                    process_id: process_id.clone(),
                    pid: native.pid,
                    exe_name: truncate_owned(&native.exe_name, MAX_STRING_CHARS).0,
                    exe_id: native.exe_id.clone(),
                    session_id: native.session_id,
                    session_verified: native.session_verified,
                    process_generation: native.start_generation,
                    generation_source: native.generation_source,
                    workspace_id: workspace_id.to_owned(),
                    policy_revision: policy_revision.to_owned(),
                    superseded: false,
                };
                self.processes.insert(process_id.clone(), record);
                (process_id, native.start_generation)
            }
        }
    }

    fn require_live_process(
        &self,
        process_id: &str,
        expected_generation: u64,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<ProcessRecord, UiaError> {
        let record = self.processes.get(process_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia process identity is unknown; it may have exited or never existed",
            )
        })?;
        if record.superseded {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia process identity was superseded by a restart and fails closed",
            ));
        }
        if record.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia process identity belongs to another workspace",
            ));
        }
        if record.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia process identity was issued under another policy revision",
            ));
        }
        if record.process_generation != expected_generation {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia process generation drifted; the process may have restarted or its PID may have been reused",
            ));
        }
        Ok(record.clone())
    }

    fn require_live_window(
        &self,
        window_id: &str,
        expected_window_generation: u64,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<WindowRecord, UiaError> {
        let record = self.windows.get(window_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia window identity is unknown; it may have been destroyed or never existed",
            )
        })?;
        if record.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia window identity belongs to another workspace",
            ));
        }
        if record.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia window identity was issued under another policy revision",
            ));
        }
        if record.window_generation != expected_window_generation {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia window generation drifted; the window may have been destroyed and its handle reused",
            ));
        }
        match self.processes.get(&record.process_id) {
            Some(process) if !process.superseded => Ok(record.clone()),
            _ => Err(UiaError::new(
                FailureCode::TargetStale,
                "uia window identity lost its live owning process",
            )),
        }
    }

    /// Observe one registered process. This is a local registry read with no
    /// adapter access and no OS mutation.
    pub fn observe_process(
        &self,
        process_id: &str,
        expected_generation: u64,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        if !is_well_formed_process_id(process_id) {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia process identity is malformed",
            ));
        }
        let record = self.require_live_process(
            process_id,
            expected_generation,
            workspace_id,
            policy_revision,
        )?;
        Ok(json!({
            "schema": UIA_SCHEMA,
            "process_id": record.process_id,
            "pid": record.pid,
            "exe_name": record.exe_name,
            "exe_id": record.exe_id,
            "session_id": record.session_id,
            "session_verified": record.session_verified,
            "process_generation": record.process_generation,
            "generation_source": record.generation_source,
            "workspace_id": record.workspace_id,
            "policy_revision": record.policy_revision,
        }))
    }

    /// Discover windows through the adapter and bind them into typed
    /// identities. Protected Cotra surfaces are omitted and counted, never
    /// returned. Unknown adapter failures fail closed as unavailable.
    pub fn list_windows(
        &mut self,
        adapter: &impl UiaAdapter,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        let natives = adapter.list_processes().map_err(|error| {
            UiaError::new(
                FailureCode::ProviderUnavailable,
                format!("uia process enumeration is unavailable: {}", error.message),
            )
        })?;
        let mut entries: Vec<Value> = Vec::new();
        let mut protected_omitted = 0u64;
        for native_process in natives.iter().take(MAX_WINDOWS) {
            let (owner_id, owner_generation) =
                self.register_process(native_process, workspace_id, policy_revision);
            let native_windows = adapter.list_windows(native_process.pid).map_err(|error| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    format!("uia window enumeration is unavailable: {}", error.message),
                )
            })?;
            for native_window in native_windows.iter().take(MAX_WINDOWS) {
                let record =
                    self.register_window(&owner_id, native_window, workspace_id, policy_revision);
                if is_protected_window(&native_window.title, &native_window.class) {
                    protected_omitted += 1;
                    continue;
                }
                entries.push(json!({
                    "process_id": record.process_id,
                    "process_generation": owner_generation,
                    "window_id": record.window_id,
                    "window_generation": record.window_generation,
                }));
                if entries.len() >= MAX_WINDOWS {
                    break;
                }
            }
            if entries.len() >= MAX_WINDOWS {
                break;
            }
        }
        let result = json!({
            "schema": UIA_SCHEMA,
            "workspace_id": workspace_id,
            "policy_revision": policy_revision,
            "windows": entries,
            "window_count": entries.len(),
            "protected_omitted": protected_omitted,
        });
        enforce_response_bytes(&result)
    }

    fn register_window(
        &mut self,
        process_id: &str,
        native: &NativeWindow,
        workspace_id: &str,
        policy_revision: &str,
    ) -> WindowRecord {
        let window_id = allocate_window_id(process_id, native.hwnd);
        self.window_counter += 1;
        match self.windows.get(&window_id) {
            Some(existing)
                if existing.process_id == process_id
                    && existing.window_nonce == native.window_nonce
                    && existing.title == truncate_owned(&native.title, MAX_STRING_CHARS).0
                    && existing.class == truncate_owned(&native.class, MAX_STRING_CHARS).0 =>
            {
                existing.clone()
            }
            Some(_) => {
                self.tree_counter += 1;
                let record = WindowRecord {
                    window_id: window_id.clone(),
                    process_id: process_id.to_owned(),
                    hwnd: native.hwnd,
                    title: truncate_owned(&native.title, MAX_STRING_CHARS).0,
                    class: truncate_owned(&native.class, MAX_STRING_CHARS).0,
                    visible: native.visible,
                    window_nonce: native.window_nonce,
                    window_generation: self.window_counter,
                    tree_generation: self.tree_counter,
                    workspace_id: workspace_id.to_owned(),
                    policy_revision: policy_revision.to_owned(),
                };
                self.remove_window_elements(&window_id);
                self.windows.insert(window_id, record.clone());
                record
            }
            None => {
                self.tree_counter += 1;
                let record = WindowRecord {
                    window_id: window_id.clone(),
                    process_id: process_id.to_owned(),
                    hwnd: native.hwnd,
                    title: truncate_owned(&native.title, MAX_STRING_CHARS).0,
                    class: truncate_owned(&native.class, MAX_STRING_CHARS).0,
                    visible: native.visible,
                    window_nonce: native.window_nonce,
                    window_generation: self.window_counter,
                    tree_generation: self.tree_counter,
                    workspace_id: workspace_id.to_owned(),
                    policy_revision: policy_revision.to_owned(),
                };
                self.windows.insert(window_id, record.clone());
                record
            }
        }
    }

    fn remove_window_elements(&mut self, window_id: &str) {
        let stale: Vec<String> = self
            .elements
            .iter()
            .filter(|(_, element)| element.window_id == window_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in stale {
            self.elements.remove(&id);
        }
    }

    /// Observe one typed window without reading its tree.
    pub fn observe_window(
        &self,
        window_id: &str,
        expected_window_generation: u64,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        if !is_well_formed_window_id(window_id) {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia window identity is malformed",
            ));
        }
        let record = self.require_live_window(
            window_id,
            expected_window_generation,
            workspace_id,
            policy_revision,
        )?;
        if is_protected_window(&record.title, &record.class) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia observation of a protected Cotra surface is denied",
            ));
        }
        let process = self.processes.get(&record.process_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia window identity lost its owning process",
            )
        })?;
        let result = json!({
            "schema": UIA_SCHEMA,
            "window_id": record.window_id,
            "process_id": record.process_id,
            "process_generation": process.process_generation,
            "window_generation": record.window_generation,
            "tree_generation": record.tree_generation,
            "title": record.title,
            "class": record.class,
            "visible": record.visible,
            "workspace_id": record.workspace_id,
            "policy_revision": record.policy_revision,
        });
        enforce_response_bytes(&result)
    }

    /// Observe the bounded tree of one typed window. The adapter supplies
    /// the live tree, which is re-registered every call so disappeared,
    /// replaced, and role-changed elements fail closed on the next
    /// observation instead of serving stale snapshots.
    #[allow(clippy::too_many_arguments)]
    pub fn observe_tree(
        &mut self,
        adapter: &impl UiaAdapter,
        window_id: &str,
        expected_window_generation: u64,
        max_depth: Option<u64>,
        max_nodes: Option<u64>,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        if !is_well_formed_window_id(window_id) {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia window identity is malformed",
            ));
        }
        let record = self.require_live_window(
            window_id,
            expected_window_generation,
            workspace_id,
            policy_revision,
        )?;
        if is_protected_window(&record.title, &record.class) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia observation of a protected Cotra surface is denied",
            ));
        }
        let depth_cap = max_depth
            .unwrap_or(MAX_TREE_DEPTH as u64)
            .min(MAX_TREE_DEPTH as u64);
        let node_cap = max_nodes
            .unwrap_or(MAX_TREE_NODES as u64)
            .min(MAX_TREE_NODES as u64);
        if node_cap == 0 {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia tree observation requires at least one node",
            ));
        }
        let natives = adapter.read_tree(record.hwnd).map_err(|error| {
            UiaError::new(
                FailureCode::ProviderUnavailable,
                format!("uia tree enumeration is unavailable: {}", error.message),
            )
        })?;
        self.tree_counter += 1;
        let tree_generation = self.tree_counter;
        if let Some(stored) = self.windows.get_mut(window_id) {
            stored.tree_generation = tree_generation;
        }
        self.remove_window_elements(window_id);
        let mut rendered: Vec<Value> = Vec::new();
        let mut truncated = false;
        let mut used_bytes = 0usize;
        let mut stack: Vec<(&NativeElement, u64)> = natives
            .iter()
            .rev()
            .map(|element| (element, 0u64))
            .collect();
        while let Some((native, depth)) = stack.pop() {
            if rendered.len() as u64 >= node_cap {
                truncated = true;
                break;
            }
            if depth > depth_cap {
                truncated = true;
                continue;
            }
            let stored = self.store_element(window_id, native, tree_generation);
            let node = render_element(&stored);
            used_bytes += serde_json::to_vec(&node)
                .map(|bytes| bytes.len())
                .unwrap_or(0);
            if used_bytes > MAX_RESPONSE_BYTES {
                truncated = true;
                self.elements.remove(&stored.element_id);
                break;
            }
            rendered.push(node);
            for child in native.children.iter().rev() {
                stack.push((child, depth + 1));
            }
        }
        let result = json!({
            "schema": UIA_SCHEMA,
            "window_id": record.window_id,
            "window_generation": record.window_generation,
            "tree_generation": tree_generation,
            "nodes": rendered,
            "node_count": rendered.len(),
            "truncated": truncated,
            "workspace_id": workspace_id,
            "policy_revision": policy_revision,
        });
        enforce_response_bytes(&result)
    }

    fn store_element(
        &mut self,
        window_id: &str,
        native: &NativeElement,
        tree_generation: u64,
    ) -> ElementRecord {
        let element_id = allocate_element_id(window_id, &native.runtime_id);
        let redacted = is_password_field(&native.control_type, &native.automation_id, &native.name)
            || native.value_is_password;
        let (automation_id, automation_truncated) =
            truncate_owned(&native.automation_id, MAX_STRING_CHARS);
        let (name, name_truncated) = truncate_owned(&native.name, MAX_STRING_CHARS);
        let value = if redacted {
            None
        } else {
            native
                .value
                .as_ref()
                .map(|raw| truncate_owned(raw, MAX_STRING_CHARS).0)
        };
        let _ = automation_truncated;
        let _ = name_truncated;
        let record = ElementRecord {
            element_id: element_id.clone(),
            window_id: window_id.to_owned(),
            runtime_id: truncate_owned(&native.runtime_id, MAX_STRING_CHARS).0,
            control_type: truncate_owned(&native.control_type, MAX_STRING_CHARS).0,
            automation_id,
            name,
            enabled: native.enabled,
            selected: native.selected,
            toggled: native.toggled,
            patterns: native.patterns.iter().take(16).cloned().collect(),
            value,
            value_is_password: native.value_is_password,
            redacted,
            tree_generation,
        };
        self.elements.insert(element_id, record.clone());
        record
    }

    /// Observe one typed element. The element must belong to the current
    /// tree generation of its window; older identities fail closed.
    pub fn observe_element(
        &self,
        element_id: &str,
        expected_tree_generation: u64,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        if !is_well_formed_element_id(element_id) {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia element identity is malformed",
            ));
        }
        let record = self.elements.get(element_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is unknown; it may have disappeared or never existed",
            )
        })?;
        let window = self.windows.get(&record.window_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity lost its owning window",
            )
        })?;
        if window.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity belongs to another workspace",
            ));
        }
        if window.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity was issued under another policy revision",
            ));
        }
        if record.tree_generation != expected_tree_generation
            || record.tree_generation != window.tree_generation
        {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is stale; the UI tree regenerated and the target must be re-observed",
            ));
        }
        let result = render_element(record);
        enforce_response_bytes(&result)
    }

    /// Test hook: read the current window and tree generations of a window.
    pub fn window_generations(&self, window_id: &str) -> Option<(u64, u64)> {
        self.windows
            .get(window_id)
            .map(|record| (record.window_generation, record.tree_generation))
    }

    /// Test hook: simulate a window replacement behind a reused handle.
    pub fn bump_window_generation(&mut self, window_id: &str) -> bool {
        self.window_counter += 1;
        self.tree_counter += 1;
        let window_counter = self.window_counter;
        let tree_counter = self.tree_counter;
        match self.windows.get_mut(window_id) {
            Some(record) => {
                record.window_generation = window_counter;
                record.tree_generation = tree_counter;
                true
            }
            None => false,
        }
    }

    /// Test hook: simulate a tree regeneration without replacing the window.
    pub fn invalidate_tree(&mut self, window_id: &str) -> bool {
        self.tree_counter += 1;
        let tree_counter = self.tree_counter;
        match self.windows.get_mut(window_id) {
            Some(record) => {
                record.tree_generation = tree_counter;
                true
            }
            None => false,
        }
    }

    /// Test hook: simulate an element disappearing from the live tree.
    pub fn remove_element(&mut self, element_id: &str) -> bool {
        self.elements.remove(element_id).is_some()
    }

    /// Test hook: mark a process record superseded to simulate a restart
    /// behind a reused PID.
    pub fn mark_process_superseded(&mut self, process_id: &str) -> bool {
        match self.processes.get_mut(process_id) {
            Some(record) => {
                record.superseded = true;
                true
            }
            None => false,
        }
    }

    /// Resolve the current invoke binding for approval digest computation.
    /// This performs the same fail-closed revalidation as `invoke_element`
    /// but performs no adapter mutation, so dispatch can bind approval
    /// before actuation and then revalidate again immediately before the
    /// adapter call.
    pub fn invoke_binding(
        &self,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<InvokeBinding, UiaError> {
        let resolved = self.require_invoke_target(
            element_id,
            expected_tree_generation,
            expected_control_type,
            workspace_id,
            policy_revision,
        )?;
        Ok(resolved)
    }

    /// Perform one structured InvokePattern actuation against the injected
    /// adapter after immediate pre-actuation revalidation. Any drift fails
    /// closed without silent retargeting and without fallback to mouse,
    /// keyboard, SendInput, coordinates, screenshots, or elevation. On
    /// success the owning window tree generation is advanced and its
    /// elements are removed so stale identities cannot be replayed.
    pub fn invoke_element(
        &mut self,
        adapter: &impl UiaAdapter,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        let binding = self.require_invoke_target(
            element_id,
            expected_tree_generation,
            expected_control_type,
            workspace_id,
            policy_revision,
        )?;
        adapter
            .invoke_element(binding.hwnd, &binding.runtime_id)
            .map_err(|error| {
                UiaError::new(
                    error.code,
                    format!("uia invoke actuation failed: {}", error.message),
                )
            })?;
        let prior_tree_generation = binding.tree_generation;
        self.tree_counter += 1;
        let new_tree_generation = self.tree_counter;
        if let Some(stored) = self.windows.get_mut(&binding.window_id) {
            stored.tree_generation = new_tree_generation;
        }
        self.remove_window_elements(&binding.window_id);
        let result = json!({
            "schema": INVOKE_SCHEMA,
            "action": "invoke",
            "element_id": binding.element_id,
            "window_id": binding.window_id,
            "process_id": binding.process_id,
            "process_generation": binding.process_generation,
            "window_generation": binding.window_generation,
            "prior_tree_generation": prior_tree_generation,
            "new_tree_generation": new_tree_generation,
            "control_type": binding.control_type,
            "pattern": INVOKE_PATTERN_NAME,
            "workspace_id": workspace_id,
            "policy_revision": policy_revision,
        });
        enforce_response_bytes(&result)
    }

    /// Resolve the current value binding for approval digest computation.
    /// This performs the same fail-closed revalidation as `set_value_element`
    /// but performs no adapter mutation, so dispatch can bind approval
    /// before actuation and then revalidate again immediately before the
    /// adapter call.
    pub fn value_binding(
        &self,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        value: &str,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<ValueBinding, UiaError> {
        let resolved = self.require_value_target(
            element_id,
            expected_tree_generation,
            expected_control_type,
            value,
            workspace_id,
            policy_revision,
        )?;
        Ok(resolved)
    }

    /// Perform one structured ValuePattern write against the injected
    /// adapter after immediate pre-actuation revalidation. Any drift fails
    /// closed without silent retargeting and without fallback to keyboard,
    /// clipboard, mouse, SendInput, coordinates, screenshots, or elevation.
    /// On success the owning window tree generation is advanced and its
    /// elements are removed so stale identities cannot be replayed. Evidence
    /// carries only the value digest, never raw value bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn set_value_element(
        &mut self,
        adapter: &impl UiaAdapter,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        value: &str,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        let binding = self.require_value_target(
            element_id,
            expected_tree_generation,
            expected_control_type,
            value,
            workspace_id,
            policy_revision,
        )?;
        adapter
            .set_value_element(binding.hwnd, &binding.runtime_id, value)
            .map_err(|error| {
                UiaError::new(
                    error.code,
                    format!("uia set_value actuation failed: {}", error.message),
                )
            })?;
        let prior_tree_generation = binding.tree_generation;
        self.tree_counter += 1;
        let new_tree_generation = self.tree_counter;
        if let Some(stored) = self.windows.get_mut(&binding.window_id) {
            stored.tree_generation = new_tree_generation;
        }
        self.remove_window_elements(&binding.window_id);
        let result = json!({
            "schema": VALUE_SCHEMA,
            "action": "set_value",
            "element_id": binding.element_id,
            "window_id": binding.window_id,
            "process_id": binding.process_id,
            "process_generation": binding.process_generation,
            "window_generation": binding.window_generation,
            "prior_tree_generation": prior_tree_generation,
            "new_tree_generation": new_tree_generation,
            "control_type": binding.control_type,
            "pattern": VALUE_PATTERN_NAME,
            "value_digest": binding.value_digest,
            "workspace_id": workspace_id,
            "policy_revision": policy_revision,
        });
        enforce_response_bytes(&result)
    }

    /// Resolve the current select binding for approval digest computation.
    /// This performs the same fail-closed revalidation as `select_element`
    /// but performs no adapter mutation, so dispatch can bind approval
    /// before actuation and then revalidate again immediately before the
    /// adapter call.
    #[allow(clippy::too_many_arguments)]
    pub fn select_binding(
        &self,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        expected_selected: bool,
        selected: bool,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<SelectBinding, UiaError> {
        let resolved = self.require_select_target(
            element_id,
            expected_tree_generation,
            expected_control_type,
            expected_selected,
            selected,
            workspace_id,
            policy_revision,
        )?;
        Ok(resolved)
    }

    /// Perform one structured SelectionItem actuation against the injected
    /// adapter after immediate pre-actuation revalidation. Any drift fails
    /// closed without silent retargeting and without fallback to mouse,
    /// keyboard, SendInput, coordinates, screenshots, or elevation. On
    /// success the owning window tree generation is advanced and its
    /// elements are removed so stale identities cannot be replayed.
    #[allow(clippy::too_many_arguments)]
    pub fn select_element(
        &mut self,
        adapter: &impl UiaAdapter,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        expected_selected: bool,
        selected: bool,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<Value, UiaError> {
        let binding = self.require_select_target(
            element_id,
            expected_tree_generation,
            expected_control_type,
            expected_selected,
            selected,
            workspace_id,
            policy_revision,
        )?;
        adapter
            .select_element(binding.hwnd, &binding.runtime_id, selected)
            .map_err(|error| {
                UiaError::new(
                    error.code,
                    format!("uia select actuation failed: {}", error.message),
                )
            })?;
        let prior_tree_generation = binding.tree_generation;
        self.tree_counter += 1;
        let new_tree_generation = self.tree_counter;
        if let Some(stored) = self.windows.get_mut(&binding.window_id) {
            stored.tree_generation = new_tree_generation;
        }
        self.remove_window_elements(&binding.window_id);
        let result = json!({
            "schema": SELECT_SCHEMA,
            "action": "select",
            "element_id": binding.element_id,
            "window_id": binding.window_id,
            "process_id": binding.process_id,
            "process_generation": binding.process_generation,
            "window_generation": binding.window_generation,
            "prior_tree_generation": prior_tree_generation,
            "new_tree_generation": new_tree_generation,
            "control_type": binding.control_type,
            "pattern": SELECT_PATTERN_NAME,
            "expected_selected": binding.expected_selected,
            "selected": binding.selected,
            "workspace_id": workspace_id,
            "policy_revision": policy_revision,
        });
        enforce_response_bytes(&result)
    }

    #[allow(clippy::too_many_arguments)]
    fn require_select_target(
        &self,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        expected_selected: bool,
        _selected: bool,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<SelectBinding, UiaError> {
        if !is_well_formed_element_id(element_id) {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia element identity is malformed",
            ));
        }
        if expected_control_type.is_empty() || expected_control_type.len() > MAX_STRING_CHARS {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia select expected_control_type is empty or too large",
            ));
        }
        if !is_select_eligible_control_type(expected_control_type) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia select actuates only ListItem, TreeItem, and TabItem elements",
            ));
        }
        let record = self.elements.get(element_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is unknown; it may have disappeared or never existed",
            )
        })?;
        if record.control_type != expected_control_type {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element control type changed since observation; stale targets fail closed",
            ));
        }
        if !is_select_eligible_control_type(&record.control_type) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia select target control type is not eligible for SelectionItem actuation",
            ));
        }
        if !record
            .patterns
            .iter()
            .any(|pattern| pattern == SELECT_PATTERN_NAME)
        {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia select target does not support the SelectionItem pattern; no mouse or keyboard fallback is permitted",
            ));
        }
        if !record.enabled {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia select targets only enabled elements; disabled elements are denied",
            ));
        }
        if record.value_is_password
            || record.redacted
            || is_password_field(&record.control_type, &record.automation_id, &record.name)
        {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia select of a password or secret bearing element is denied",
            ));
        }
        if record.selected != expected_selected {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia select expected selection state drifted; the target must be re-observed",
            ));
        }
        let window = self.windows.get(&record.window_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity lost its owning window",
            )
        })?;
        if window.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity belongs to another workspace",
            ));
        }
        if window.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity was issued under another policy revision",
            ));
        }
        if record.tree_generation != expected_tree_generation
            || record.tree_generation != window.tree_generation
        {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is stale; the UI tree regenerated and the target must be re-observed",
            ));
        }
        if is_protected_window(&window.title, &window.class) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia select of a protected Cotra surface is denied",
            ));
        }
        let process = self.processes.get(&window.process_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia select target lost its owning process",
            )
        })?;
        if process.superseded {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia select target process was superseded by a restart and fails closed",
            ));
        }
        if process.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia select target process belongs to another workspace",
            ));
        }
        if process.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia select target process was issued under another policy revision",
            ));
        }
        Ok(SelectBinding {
            process_id: process.process_id.clone(),
            process_generation: process.process_generation,
            window_id: window.window_id.clone(),
            window_generation: window.window_generation,
            element_id: record.element_id.clone(),
            tree_generation: record.tree_generation,
            control_type: record.control_type.clone(),
            expected_selected,
            selected: _selected,
            hwnd: window.hwnd,
            runtime_id: record.runtime_id.clone(),
        })
    }

    fn require_value_target(
        &self,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        value: &str,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<ValueBinding, UiaError> {
        if !is_well_formed_element_id(element_id) {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia element identity is malformed",
            ));
        }
        if expected_control_type.is_empty() || expected_control_type.len() > MAX_STRING_CHARS {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia set_value expected_control_type is empty or too large",
            ));
        }
        if !is_value_eligible_control_type(expected_control_type) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia set_value actuates only Edit, Document, and ComboBox elements",
            ));
        }
        if value.chars().count() > MAX_VALUE_CHARS {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia set_value value exceeds the bounded length",
            ));
        }
        if value.contains('\0') {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia set_value value contains a NUL byte",
            ));
        }
        let record = self.elements.get(element_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is unknown; it may have disappeared or never existed",
            )
        })?;
        if record.control_type != expected_control_type {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element control type changed since observation; stale targets fail closed",
            ));
        }
        if !is_value_eligible_control_type(&record.control_type) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia set_value target control type is not eligible for ValuePattern actuation",
            ));
        }
        if !record
            .patterns
            .iter()
            .any(|pattern| pattern == VALUE_PATTERN_NAME)
        {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia set_value target does not support the Value pattern; no keyboard or clipboard fallback is permitted",
            ));
        }
        if !record.enabled {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia set_value targets only enabled elements; disabled elements are denied",
            ));
        }
        if record.value_is_password
            || record.redacted
            || is_password_field(&record.control_type, &record.automation_id, &record.name)
        {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia set_value of a password or secret bearing element is denied",
            ));
        }
        let window = self.windows.get(&record.window_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity lost its owning window",
            )
        })?;
        if window.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity belongs to another workspace",
            ));
        }
        if window.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity was issued under another policy revision",
            ));
        }
        if record.tree_generation != expected_tree_generation
            || record.tree_generation != window.tree_generation
        {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is stale; the UI tree regenerated and the target must be re-observed",
            ));
        }
        if is_protected_window(&window.title, &window.class) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia set_value of a protected Cotra surface is denied",
            ));
        }
        let process = self.processes.get(&window.process_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia set_value target lost its owning process",
            )
        })?;
        if process.superseded {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia set_value target process was superseded by a restart and fails closed",
            ));
        }
        if process.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia set_value target process belongs to another workspace",
            ));
        }
        if process.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia set_value target process was issued under another policy revision",
            ));
        }
        Ok(ValueBinding {
            process_id: process.process_id.clone(),
            process_generation: process.process_generation,
            window_id: window.window_id.clone(),
            window_generation: window.window_generation,
            element_id: record.element_id.clone(),
            tree_generation: record.tree_generation,
            control_type: record.control_type.clone(),
            value_digest: value_content_digest(value),
            hwnd: window.hwnd,
            runtime_id: record.runtime_id.clone(),
        })
    }

    fn require_invoke_target(
        &self,
        element_id: &str,
        expected_tree_generation: u64,
        expected_control_type: &str,
        workspace_id: &str,
        policy_revision: &str,
    ) -> Result<InvokeBinding, UiaError> {
        if !is_well_formed_element_id(element_id) {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia element identity is malformed",
            ));
        }
        if expected_control_type.is_empty() || expected_control_type.len() > MAX_STRING_CHARS {
            return Err(UiaError::new(
                FailureCode::InvalidRequest,
                "uia invoke expected_control_type is empty or too large",
            ));
        }
        if !is_invoke_eligible_control_type(expected_control_type) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia invoke actuates only Button, Hyperlink, MenuItem, and SplitButton elements",
            ));
        }
        let record = self.elements.get(element_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is unknown; it may have disappeared or never existed",
            )
        })?;
        if record.control_type != expected_control_type {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element control type changed since observation; stale targets fail closed",
            ));
        }
        if !is_invoke_eligible_control_type(&record.control_type) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia invoke target control type is not eligible for InvokePattern actuation",
            ));
        }
        if !record
            .patterns
            .iter()
            .any(|pattern| pattern == INVOKE_PATTERN_NAME)
        {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia invoke target does not support the Invoke pattern; no mouse or keyboard fallback is permitted",
            ));
        }
        if !record.enabled {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia invoke targets only enabled elements; disabled elements are denied",
            ));
        }
        if record.value_is_password
            || record.redacted
            || is_password_field(&record.control_type, &record.automation_id, &record.name)
        {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia invoke of a password or secret bearing element is denied",
            ));
        }
        let window = self.windows.get(&record.window_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia element identity lost its owning window",
            )
        })?;
        if window.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity belongs to another workspace",
            ));
        }
        if window.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity was issued under another policy revision",
            ));
        }
        if record.tree_generation != expected_tree_generation
            || record.tree_generation != window.tree_generation
        {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia element identity is stale; the UI tree regenerated and the target must be re-observed",
            ));
        }
        if is_protected_window(&window.title, &window.class) {
            return Err(UiaError::new(
                FailureCode::CapabilityDenied,
                "uia invoke of a protected Cotra surface is denied",
            ));
        }
        let process = self.processes.get(&window.process_id).ok_or_else(|| {
            UiaError::new(
                FailureCode::TargetStale,
                "uia invoke target lost its owning process",
            )
        })?;
        if process.superseded {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia invoke target process was superseded by a restart and fails closed",
            ));
        }
        if process.workspace_id != workspace_id {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia invoke target process belongs to another workspace",
            ));
        }
        if process.policy_revision != policy_revision {
            return Err(UiaError::new(
                FailureCode::TargetStale,
                "uia invoke target process was issued under another policy revision",
            ));
        }
        Ok(InvokeBinding {
            process_id: process.process_id.clone(),
            process_generation: process.process_generation,
            window_id: window.window_id.clone(),
            window_generation: window.window_generation,
            element_id: record.element_id.clone(),
            tree_generation: record.tree_generation,
            control_type: record.control_type.clone(),
            hwnd: window.hwnd,
            runtime_id: record.runtime_id.clone(),
        })
    }
}

/// Server-resolved invoke binding used for approval digest computation and
/// evidence. Callers never supply these values directly; they are resolved
/// from the typed element identity immediately before actuation.
#[derive(Debug, Clone)]
pub struct InvokeBinding {
    pub process_id: String,
    pub process_generation: u64,
    pub window_id: String,
    pub window_generation: u64,
    pub element_id: String,
    pub tree_generation: u64,
    pub control_type: String,
    pub hwnd: u64,
    pub runtime_id: String,
}

/// Compute the exact approval digest for one structured invoke. Any material
/// drift after approval invalidates the approval because dispatch revalidates
/// the same binding set immediately before the adapter call.
pub fn invoke_approval_digest(
    workspace_id: &str,
    policy_revision: &str,
    binding: &InvokeBinding,
) -> String {
    let material = format!(
        "cotra-uia-invoke-v1|{workspace_id}|{policy_revision}|{}|{}|{}|{}|{}|{}|{}|invoke",
        binding.process_id,
        binding.process_generation,
        binding.window_id,
        binding.window_generation,
        binding.element_id,
        binding.tree_generation,
        binding.control_type,
    );
    digest_hex(&material, 64)
}

/// Server-resolved value binding used for approval digest computation and
/// evidence. Callers never supply these values directly; they are resolved
/// from the typed element identity immediately before actuation.
#[derive(Debug, Clone)]
pub struct ValueBinding {
    pub process_id: String,
    pub process_generation: u64,
    pub window_id: String,
    pub window_generation: u64,
    pub element_id: String,
    pub tree_generation: u64,
    pub control_type: String,
    pub value_digest: String,
    pub hwnd: u64,
    pub runtime_id: String,
}

/// Compute the exact approval digest for one structured set_value. The
/// digest binds the value digest rather than raw value bytes where
/// practical, and dispatch revalidates the same binding set immediately
/// before the adapter call so any material drift invalidates the approval.
pub fn value_approval_digest(
    workspace_id: &str,
    policy_revision: &str,
    binding: &ValueBinding,
) -> String {
    let material = format!(
        "cotra-uia-value-v1|{workspace_id}|{policy_revision}|{}|{}|{}|{}|{}|{}|{}|{}|set_value",
        binding.process_id,
        binding.process_generation,
        binding.window_id,
        binding.window_generation,
        binding.element_id,
        binding.tree_generation,
        binding.control_type,
        binding.value_digest,
    );
    digest_hex(&material, 64)
}

/// Server-resolved select binding used for approval digest computation and
/// evidence. Callers never supply these values directly; they are resolved
/// from the typed element identity immediately before actuation.
#[derive(Debug, Clone)]
pub struct SelectBinding {
    pub process_id: String,
    pub process_generation: u64,
    pub window_id: String,
    pub window_generation: u64,
    pub element_id: String,
    pub tree_generation: u64,
    pub control_type: String,
    pub expected_selected: bool,
    pub selected: bool,
    pub hwnd: u64,
    pub runtime_id: String,
}

/// Compute the exact approval digest for one structured select. The digest
/// binds expected and requested selection state, and dispatch revalidates
/// the same binding set immediately before the adapter call so any material
/// drift invalidates the approval.
pub fn select_approval_digest(
    workspace_id: &str,
    policy_revision: &str,
    binding: &SelectBinding,
) -> String {
    let material = format!(
        "cotra-uia-select-v1|{workspace_id}|{policy_revision}|{}|{}|{}|{}|{}|{}|{}|{}|{}|select",
        binding.process_id,
        binding.process_generation,
        binding.window_id,
        binding.window_generation,
        binding.element_id,
        binding.tree_generation,
        binding.control_type,
        if binding.expected_selected { "1" } else { "0" },
        if binding.selected { "1" } else { "0" },
    );
    digest_hex(&material, 64)
}

fn render_element(record: &ElementRecord) -> Value {
    json!({
        "schema": UIA_SCHEMA,
        "element_id": record.element_id,
        "window_id": record.window_id,
        "runtime_id": record.runtime_id,
        "control_type": record.control_type,
        "automation_id": record.automation_id,
        "name": record.name,
        "enabled": record.enabled,
        "selected": record.selected,
        "toggled": record.toggled,
        "patterns": record.patterns,
        "value": record.value,
        "value_is_password": record.value_is_password,
        "redacted": record.redacted,
        "tree_generation": record.tree_generation,
    })
}

fn enforce_response_bytes(result: &Value) -> Result<Value, UiaError> {
    let bytes = serde_json::to_vec(result)
        .map(|bytes| bytes.len())
        .unwrap_or(0);
    if bytes > MAX_RESPONSE_BYTES {
        return Err(UiaError::new(
            FailureCode::OutputLimit,
            "uia observation exceeded the bounded response size",
        ));
    }
    Ok(result.clone())
}

fn truncate_owned(raw: &str, limit: usize) -> (String, bool) {
    let clipped: String = raw.chars().take(limit).collect();
    let truncated = raw.chars().count() > limit;
    (clipped, truncated)
}

pub fn is_protected_window(title: &str, class: &str) -> bool {
    let haystack = format!("{title} {class}").to_ascii_lowercase();
    PROTECTED_WINDOW_MARKERS
        .iter()
        .any(|marker| haystack.contains(marker))
}

pub fn is_password_field(control_type: &str, automation_id: &str, name: &str) -> bool {
    let haystack = format!("{control_type} {automation_id} {name}").to_ascii_lowercase();
    PASSWORD_MARKERS
        .iter()
        .any(|marker| haystack.contains(marker))
}

#[cfg(test)]
mod sg000027_tests;
#[cfg(test)]
mod sg000028_tests;
#[cfg(test)]
mod sg000029_tests;
#[cfg(test)]
mod sg000030_tests;
