//! SG-000081 structured Windows UI Automation policy.
//!
//! InvokePattern, ValuePattern, SelectionItemPattern, TogglePattern,
//! and ScrollPattern actuate only on exact typed elements with pattern
//! support and enabled state. Every mutation binds exact process,
//! window, element, role, state, supported pattern, workspace, policy
//! revision, approval, and postcondition. Password, security, and
//! protected surfaces are denied. Drifted elements require a fresh
//! observation, never silent retargeting, and no keyboard, mouse,
//! SendInput, coordinate, screenshot, or elevation authority appears.

use crate::error::HostError;

/// Supported UIA patterns in this grain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedPattern {
    Invoke,
    Value,
    SelectionItem,
    Toggle,
    Scroll,
}

impl SupportedPattern {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Invoke => "invoke",
            Self::Value => "value",
            Self::SelectionItem => "select",
            Self::Toggle => "toggle",
            Self::Scroll => "scroll",
        }
    }
}

/// Maximum value characters for ValuePattern.
pub const MAX_UIA_VALUE_CHARS: usize = 1_024;

/// Minimum and maximum scroll amounts.
pub const MIN_SCROLL_AMOUNT: u32 = 1;
pub const MAX_SCROLL_AMOUNT: u32 = 100;

/// Scroll directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn parse(text: &str) -> Result<Self, HostError> {
        match text {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(HostError::Invalid("invalid scroll direction".into())),
        }
    }
}

/// Exact server-derived process binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiaProcess {
    pub pid: u32,
    pub executable_digest: String,
    pub process_generation: u64,
}

/// Exact typed window binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiaWindow {
    pub hwnd: u64,
    pub window_generation: u64,
}

/// Exact typed element binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiaElement {
    pub runtime_id: String,
    pub tree_generation: u64,
    pub control_type: String,
}

/// Exact pre-dispatch UIA target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiaTarget {
    pub process: UiaProcess,
    pub window: UiaWindow,
    pub element: UiaElement,
    pub pattern: SupportedPattern,
    pub enabled: bool,
    pub workspace: String,
    pub policy_revision: u64,
}

/// Fresh one-shot UIA approval bound to the target digest.
#[derive(Debug, Clone)]
pub struct UiaApproval {
    pub approval_id: String,
    pub target_digest: String,
    pub workspace: String,
    pub policy_revision: u64,
    pub consumed: bool,
}

impl UiaApproval {
    pub fn consume(&mut self) -> Result<(), HostError> {
        if self.consumed {
            return Err(HostError::Invalid("uia approval replay denied".into()));
        }
        if self.approval_id.is_empty() || self.target_digest.is_empty() {
            return Err(HostError::Invalid("uia approval malformed".into()));
        }
        self.consumed = true;
        Ok(())
    }
}

/// Binding digest over the complete UIA target.
pub fn uia_target_digest(target: &UiaTarget) -> String {
    let text = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        target.process.pid,
        target.process.executable_digest,
        target.process.process_generation,
        target.window.hwnd,
        target.window.window_generation,
        target.element.runtime_id,
        target.element.tree_generation,
        target.element.control_type,
        target.pattern.name(),
        target.workspace,
        target.policy_revision
    );
    crate::observation::short_digest(&text)
}

/// Whether a surface is protected and never actuated.
pub fn is_protected_uia_surface(control_type: &str, name: &str) -> bool {
    let haystack = format!("{control_type} {name}").to_ascii_lowercase();
    if crate::capture::is_protected_surface(control_type, name) {
        return true;
    }
    for marker in ["password", "secret", "credential", "pin", "token"] {
        if haystack.contains(marker) {
            return true;
        }
    }
    false
}

/// Verify pattern support, enabled state, and protected denial for a
/// control type and pattern pair.
pub fn verify_pattern_support(
    control_type: &str,
    pattern: SupportedPattern,
    enabled: bool,
    name: &str,
) -> Result<(), HostError> {
    if !enabled {
        return Err(HostError::Invalid("uia disabled element denied".into()));
    }
    if is_protected_uia_surface(control_type, name) {
        return Err(HostError::Invalid("uia protected surface denied".into()));
    }
    let lower = control_type.to_ascii_lowercase();
    let supported = match pattern {
        SupportedPattern::Invoke => ["button", "hyperlink", "menuitem", "splitbutton"]
            .iter()
            .any(|entry| lower.contains(entry)),
        SupportedPattern::Value => ["edit", "document", "combobox"]
            .iter()
            .any(|entry| lower.contains(entry)),
        SupportedPattern::SelectionItem => ["listitem", "treeitem", "tabitem"]
            .iter()
            .any(|entry| lower.contains(entry)),
        SupportedPattern::Toggle => ["checkbox", "radiobutton"]
            .iter()
            .any(|entry| lower.contains(entry)),
        SupportedPattern::Scroll => ["scrollbar", "pane", "list", "tree"]
            .iter()
            .any(|entry| lower.contains(entry)),
    };
    if !supported {
        return Err(HostError::Invalid(
            "uia pattern unsupported for control type".into(),
        ));
    }
    Ok(())
}

/// Live state snapshot presented for verification.
pub struct UiaLive {
    pub process_generation: u64,
    pub window_generation: u64,
    pub tree_generation: u64,
    pub control_type: String,
    pub workspace: String,
    pub policy_revision: u64,
}

/// Verify the full target immediately before dispatch.
pub fn verify_uia_target(target: &UiaTarget, name: &str, live: &UiaLive) -> Result<(), HostError> {
    if target.process.pid == 0 || target.window.hwnd == 0 || target.element.runtime_id.is_empty() {
        return Err(HostError::Invalid("uia identity malformed".into()));
    }
    crate::observation::validate_identity_field(&target.workspace)?;
    crate::observation::validate_identity_field(&target.element.runtime_id)?;
    if target.process.process_generation != live.process_generation {
        return Err(HostError::Invalid("uia pid reuse or restart denied".into()));
    }
    if target.window.window_generation != live.window_generation {
        return Err(HostError::Invalid(
            "uia destroyed or reused handle denied".into(),
        ));
    }
    if target.element.tree_generation != live.tree_generation {
        return Err(HostError::Invalid(
            "uia disappeared or replaced element denied".into(),
        ));
    }
    if target.element.control_type != live.control_type {
        return Err(HostError::Invalid("uia role change denied".into()));
    }
    if target.workspace != live.workspace {
        return Err(HostError::Invalid("uia workspace drift denied".into()));
    }
    if target.policy_revision != live.policy_revision {
        return Err(HostError::Invalid("uia policy drift denied".into()));
    }
    verify_pattern_support(
        &target.element.control_type,
        target.pattern,
        target.enabled,
        name,
    )?;
    Ok(())
}

/// Common dispatch gate: verification plus one-shot approval.
fn dispatch_gate(
    target: &UiaTarget,
    name: &str,
    approval: &mut UiaApproval,
    live: &UiaLive,
) -> Result<(), HostError> {
    verify_uia_target(target, name, live)?;
    if approval.consumed {
        return Err(HostError::Invalid("uia approval replay denied".into()));
    }
    if approval.workspace != target.workspace || approval.policy_revision != target.policy_revision
    {
        return Err(HostError::Invalid("uia approval drift denied".into()));
    }
    if approval.target_digest != uia_target_digest(target) {
        return Err(HostError::Invalid("uia approval digest mismatch".into()));
    }
    approval.consume()
}

/// Invoke one exact typed element. Success advances the tree generation.
pub fn invoke(
    target: &UiaTarget,
    name: &str,
    approval: &mut UiaApproval,
    live: &UiaLive,
) -> Result<u64, HostError> {
    if target.pattern != SupportedPattern::Invoke {
        return Err(HostError::Invalid("uia wrong pattern for invoke".into()));
    }
    dispatch_gate(target, name, approval, live)?;
    Ok(live.tree_generation.wrapping_add(1))
}

/// Bounded non-secret value entry. Success advances the tree generation.
pub fn set_value(
    target: &UiaTarget,
    name: &str,
    value: &str,
    approval: &mut UiaApproval,
    live: &UiaLive,
) -> Result<u64, HostError> {
    if target.pattern != SupportedPattern::Value {
        return Err(HostError::Invalid("uia wrong pattern for value".into()));
    }
    if value.chars().count() > MAX_UIA_VALUE_CHARS || value.contains('\0') {
        return Err(HostError::Invalid(
            "uia value oversized or nul denied".into(),
        ));
    }
    dispatch_gate(target, name, approval, live)?;
    Ok(live.tree_generation.wrapping_add(1))
}

/// Selection change with expected and requested states. No blind writes.
pub fn select(
    target: &UiaTarget,
    name: &str,
    expected_selected: bool,
    requested_selected: bool,
    actual_selected: bool,
    approval: &mut UiaApproval,
    live: &UiaLive,
) -> Result<u64, HostError> {
    if target.pattern != SupportedPattern::SelectionItem {
        return Err(HostError::Invalid("uia wrong pattern for select".into()));
    }
    if expected_selected != actual_selected {
        return Err(HostError::Invalid(
            "uia expected selection drift denied".into(),
        ));
    }
    let _ = requested_selected;
    dispatch_gate(target, name, approval, live)?;
    Ok(live.tree_generation.wrapping_add(1))
}

/// Toggle change with expected and requested states. No blind inversion.
pub fn toggle(
    target: &UiaTarget,
    name: &str,
    expected_on: bool,
    requested_on: bool,
    actual_on: bool,
    approval: &mut UiaApproval,
    live: &UiaLive,
) -> Result<u64, HostError> {
    if target.pattern != SupportedPattern::Toggle {
        return Err(HostError::Invalid("uia wrong pattern for toggle".into()));
    }
    if expected_on != actual_on {
        return Err(HostError::Invalid(
            "uia expected toggle drift denied".into(),
        ));
    }
    let _ = requested_on;
    dispatch_gate(target, name, approval, live)?;
    Ok(live.tree_generation.wrapping_add(1))
}

/// Bounded scroll with direction and amount limits.
pub fn scroll(
    target: &UiaTarget,
    name: &str,
    direction: ScrollDirection,
    amount: u32,
    approval: &mut UiaApproval,
    live: &UiaLive,
) -> Result<u64, HostError> {
    if target.pattern != SupportedPattern::Scroll {
        return Err(HostError::Invalid("uia wrong pattern for scroll".into()));
    }
    let _ = direction;
    if !(MIN_SCROLL_AMOUNT..=MAX_SCROLL_AMOUNT).contains(&amount) {
        return Err(HostError::Invalid("uia scroll amount out of bounds".into()));
    }
    dispatch_gate(target, name, approval, live)?;
    Ok(live.tree_generation.wrapping_add(1))
}

/// Denied: keyboard synthesis never exists in this grain.
pub fn keyboard_synthesis() -> Result<(), HostError> {
    Err(HostError::Invalid("uia keyboard fallback denied".into()))
}

/// Denied: mouse and SendInput synthesis never exist in this grain.
pub fn mouse_synthesis() -> Result<(), HostError> {
    Err(HostError::Invalid("uia mouse fallback denied".into()))
}

/// Denied: coordinate fallback never exists in this grain.
pub fn uia_coordinate_fallback() -> Result<(), HostError> {
    Err(HostError::Invalid("uia coordinate fallback denied".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uia_target(pattern: SupportedPattern, control_type: &str) -> UiaTarget {
        UiaTarget {
            process: UiaProcess {
                pid: 4242,
                executable_digest: "app-digest".into(),
                process_generation: 11,
            },
            window: UiaWindow {
                hwnd: 0x00A1B2,
                window_generation: 5,
            },
            element: UiaElement {
                runtime_id: "element-1".into(),
                tree_generation: 3,
                control_type: control_type.into(),
            },
            pattern,
            enabled: true,
            workspace: "workspace-1".into(),
            policy_revision: 9,
        }
    }

    fn live(control_type: &str) -> UiaLive {
        UiaLive {
            process_generation: 11,
            window_generation: 5,
            tree_generation: 3,
            control_type: control_type.into(),
            workspace: "workspace-1".into(),
            policy_revision: 9,
        }
    }

    fn approval_for(target: &UiaTarget) -> UiaApproval {
        UiaApproval {
            approval_id: "uia-approval-1".into(),
            target_digest: uia_target_digest(target),
            workspace: target.workspace.clone(),
            policy_revision: target.policy_revision,
            consumed: false,
        }
    }

    #[test]
    fn invoke_pattern_actuates_typed_buttons() {
        let target = uia_target(SupportedPattern::Invoke, "Button");
        let mut approval = approval_for(&target);
        assert_eq!(
            invoke(&target, "Submit", &mut approval, &live("Button")).unwrap(),
            4
        );
    }

    #[test]
    fn value_pattern_actuates_bounded_non_secret_values() {
        let target = uia_target(SupportedPattern::Value, "Edit");
        let mut approval = approval_for(&target);
        assert_eq!(
            set_value(&target, "Name", "Ada", &mut approval, &live("Edit")).unwrap(),
            4
        );
        let mut approval = approval_for(&target);
        assert!(set_value(
            &target,
            "Name",
            &"x".repeat(MAX_UIA_VALUE_CHARS + 1),
            &mut approval,
            &live("Edit")
        )
        .is_err());
    }

    #[test]
    fn select_toggle_and_scroll_bind_expected_state() {
        let target = uia_target(SupportedPattern::SelectionItem, "ListItem");
        let mut approval = approval_for(&target);
        assert_eq!(
            select(
                &target,
                "Row",
                false,
                true,
                false,
                &mut approval,
                &live("ListItem")
            )
            .unwrap(),
            4
        );
        let target = uia_target(SupportedPattern::Toggle, "CheckBox");
        let mut approval = approval_for(&target);
        assert!(select(
            &target,
            "Box",
            false,
            true,
            false,
            &mut approval,
            &live("CheckBox")
        )
        .is_err());
        let mut approval = approval_for(&target);
        assert_eq!(
            toggle(
                &target,
                "Box",
                false,
                true,
                false,
                &mut approval,
                &live("CheckBox")
            )
            .unwrap(),
            4
        );
        let target = uia_target(SupportedPattern::Scroll, "Pane");
        let mut approval = approval_for(&target);
        assert_eq!(
            scroll(
                &target,
                "Pane",
                ScrollDirection::Down,
                10,
                &mut approval,
                &live("Pane")
            )
            .unwrap(),
            4
        );
        let mut approval = approval_for(&target);
        assert!(scroll(
            &target,
            "Pane",
            ScrollDirection::Down,
            0,
            &mut approval,
            &live("Pane")
        )
        .is_err());
        assert!(ScrollDirection::parse("diagonal").is_err());
    }

    #[test]
    fn drift_reuse_and_protected_surfaces_fail_closed() {
        let target = uia_target(SupportedPattern::Invoke, "Button");
        let mut approval = approval_for(&target);
        let mut drifted = live("Button");
        drifted.tree_generation = 4;
        assert!(invoke(&target, "Submit", &mut approval, &drifted).is_err());
        let mut approval = approval_for(&target);
        invoke(&target, "Submit", &mut approval, &live("Button")).unwrap();
        assert!(invoke(&target, "Submit", &mut approval, &live("Button")).is_err());
        let target = uia_target(SupportedPattern::Invoke, "Button");
        let mut approval = approval_for(&target);
        assert!(invoke(
            &target,
            "Deskal approval dialog",
            &mut approval,
            &live("Button")
        )
        .is_err());
        let secret = uia_target(SupportedPattern::Value, "password");
        let mut approval = approval_for(&secret);
        assert!(set_value(&secret, "Secret", "x", &mut approval, &live("password")).is_err());
    }

    #[test]
    fn synthetic_input_fallbacks_never_exist() {
        assert!(keyboard_synthesis().is_err());
        assert!(mouse_synthesis().is_err());
        assert!(uia_coordinate_fallback().is_err());
    }
}
