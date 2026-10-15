//! SG-000079 explicit browser tool exposure contract.
//!
//! The live host exposes zero MCP tools in this grain. The explicit
//! `browser_structured` profile names the maximum admissible set: exactly
//! the twelve already-qualified closed-registry browser shapes, nothing
//! more. Existing profiles are not widened, remote browser mapping stays
//! disabled, and the denied set (evaluation, arbitrary JavaScript,
//! generic CDP, personal profiles, raw process control, generic
//! filesystem authority) stays unreachable. Discovery, documentation,
//! profile tables, OAuth scope tables, and runtime agree mechanically:
//! all of them resolve to the same empty live set.

use crate::error::HostError;

/// The explicit live browser profile. No caller reaches it implicitly.
pub const BROWSER_STRUCTURED_PROFILE: &str = "browser_structured";

/// Profiles that existed before this grain. Neither may gain browser
/// shapes implicitly through this grain.
pub const PREEXISTING_PROFILES: &[&str] = &["core", "desktop_structured"];

/// Maximum admissible set: exactly the twelve already-qualified
/// closed-registry browser shapes. Admission here is not exposure: the
/// live set below is what MCP callers can actually reach.
pub const QUALIFIED_BROWSER_SHAPES: &[(&str, &str)] = &[
    ("browser.profile", "status"),
    ("browser.destination", "validate"),
    ("browser.page", "open"),
    ("browser.navigation", "preview"),
    ("browser.navigation", "navigate"),
    ("browser.snapshot", "observe"),
    ("browser.dom", "click"),
    ("browser.dom", "fill"),
    ("browser.download", "preview"),
    ("browser.download", "download"),
    ("browser.upload", "preview"),
    ("browser.upload", "submit"),
];

/// Shapes that must never appear under any profile.
pub const DENIED_BROWSER_SHAPES: &[&str] = &[
    "browser_evaluate",
    "browser.evaluate",
    "browser.script",
    "browser.cdp",
    "browser.devtools",
    "browser.profile.use_personal",
    "browser.profile.attach",
    "browser.process.launch",
    "browser.process.attach",
    "browser.fs.read",
    "browser.fs.list",
    "browser.remote.map",
];

/// Live MCP tools served by the low-authority host in this grain: none.
/// The host vocabulary (hello, ping, shutdown) is a private piped
/// channel, not an MCP surface.
pub const LIVE_BROWSER_MCP_TOOLS: &[&str] = &[];

/// Whether a capability and operation pair is within the maximum
/// admissible set. Admission is necessary but not sufficient for live
/// exposure.
pub fn is_qualified_shape(capability: &str, operation: &str) -> bool {
    QUALIFIED_BROWSER_SHAPES
        .iter()
        .any(|(cap, op)| *cap == capability && *op == operation)
}

/// Whether a tool name is denied under every profile.
pub fn is_denied_shape(tool: &str) -> bool {
    DENIED_BROWSER_SHAPES.contains(&tool)
}

/// Whether a tool name is live-reachable through MCP in this grain.
/// Always false: nothing in the admissible set is wired to MCP yet.
pub fn is_live_exposed(tool: &str) -> bool {
    LIVE_BROWSER_MCP_TOOLS.contains(&tool)
}

/// Remote browser mapping stays disabled in this grain.
pub fn remote_browser_mapping_enabled() -> bool {
    false
}

/// Resolve the tool set for a profile name. Unknown profiles, including
/// future ones, fail closed instead of inheriting a default set.
pub fn profile_tools(profile: &str) -> Result<&'static [&'static str], HostError> {
    if profile == BROWSER_STRUCTURED_PROFILE {
        return Ok(LIVE_BROWSER_MCP_TOOLS);
    }
    Err(HostError::Invalid(
        "tool surface profile is not mapped for live browser".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn qualified_set_is_exactly_twelve_closed_shapes() {
        assert_eq!(QUALIFIED_BROWSER_SHAPES.len(), 12);
        let unique: HashSet<(&str, &str)> = QUALIFIED_BROWSER_SHAPES.iter().copied().collect();
        assert_eq!(unique.len(), 12);
        assert!(is_qualified_shape("browser.snapshot", "observe"));
        assert!(is_qualified_shape("browser.dom", "click"));
        assert!(!is_qualified_shape("browser.dom", "evaluate"));
        assert!(!is_qualified_shape("browser.cdp", "command"));
    }

    #[test]
    fn live_set_is_empty_and_denied_shapes_stay_absent() {
        assert!(LIVE_BROWSER_MCP_TOOLS.is_empty());
        for denied in DENIED_BROWSER_SHAPES {
            assert!(!is_live_exposed(denied), "{denied}");
        }
        for (capability, operation) in QUALIFIED_BROWSER_SHAPES {
            let tool = format!("{capability}/{operation}");
            assert!(!is_live_exposed(&tool), "{tool}");
        }
        assert!(!is_live_exposed("browser_evaluate"));
    }

    #[test]
    fn preexisting_profiles_gain_no_browser_shapes() {
        assert_eq!(PREEXISTING_PROFILES, &["core", "desktop_structured"]);
        assert!(!PREEXISTING_PROFILES.contains(&BROWSER_STRUCTURED_PROFILE));
    }

    #[test]
    fn remote_mapping_stays_disabled_and_unknown_profiles_fail_closed() {
        assert!(!remote_browser_mapping_enabled());
        assert!(profile_tools(BROWSER_STRUCTURED_PROFILE)
            .unwrap()
            .is_empty());
        assert!(profile_tools("core").is_err());
        assert!(profile_tools("browser_evaluate").is_err());
        assert!(profile_tools("").is_err());
    }
}
