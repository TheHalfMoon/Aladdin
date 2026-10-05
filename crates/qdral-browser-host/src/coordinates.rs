//! SG-000082 governed coordinate fallback policy.
//!
//! Coordinate control is lower-ceiling authority and never automatic.
//! Every proposal binds exact capture generation, target window,
//! geometry, DPI, action, and coordinates. Every execution requires a
//! fresh InputLease, one approval where applicable, one exact action,
//! and one execution. Human physical input interrupts and revokes
//! active synthetic input authority. Only click, double-click,
//! right-click, bounded scroll, and bounded Unicode text input exist.

use crate::error::HostError;

/// Maximum scroll delta per axis in pixels.
pub const MAX_SCROLL_DELTA: i32 = 500;

/// Maximum text input characters per execution.
pub const MAX_INPUT_TEXT_CHARS: usize = 256;

/// Input lease lifetime in milliseconds.
pub const INPUT_LEASE_TTL_MS: u64 = 60_000;

/// Coordinate actions admitted in this grain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinateAction {
    Click,
    DoubleClick,
    RightClick,
    Scroll { dx: i32, dy: i32 },
    Text { text: String },
}

impl CoordinateAction {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::DoubleClick => "double-click",
            Self::RightClick => "right-click",
            Self::Scroll { .. } => "scroll",
            Self::Text { .. } => "text",
        }
    }
}

/// Exact coordinate proposal bound to a capture generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinateProposal {
    pub capture_generation: u64,
    pub hwnd: u64,
    pub window_generation: u64,
    pub geometry_x: i32,
    pub geometry_y: i32,
    pub geometry_width: u32,
    pub geometry_height: u32,
    pub dpi: u32,
    pub action: CoordinateAction,
    pub x: i32,
    pub y: i32,
}

/// Binding digest over the complete proposal.
pub fn proposal_digest(proposal: &CoordinateProposal) -> String {
    let text = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        proposal.capture_generation,
        proposal.hwnd,
        proposal.window_generation,
        proposal.geometry_x,
        proposal.geometry_y,
        proposal.geometry_width,
        proposal.geometry_height,
        proposal.dpi,
        proposal.action.name(),
        proposal.x,
        proposal.y
    );
    crate::observation::short_digest(&text)
}

/// Validate a proposal against live capture state. Coordinates must
/// fall inside the bound geometry and every identity must match.
pub fn validate_proposal(
    proposal: &CoordinateProposal,
    actual_capture_generation: u64,
    actual_window_generation: u64,
    actual_dpi: u32,
) -> Result<(), HostError> {
    if proposal.hwnd == 0 {
        return Err(HostError::Invalid(
            "coordinate window handle malformed".into(),
        ));
    }
    if proposal.capture_generation != actual_capture_generation {
        return Err(HostError::Invalid("stale capture generation".into()));
    }
    if proposal.window_generation != actual_window_generation {
        return Err(HostError::Invalid("coordinate window drift denied".into()));
    }
    if proposal.dpi != actual_dpi {
        return Err(HostError::Invalid("coordinate dpi drift denied".into()));
    }
    if proposal.geometry_width == 0
        || proposal.geometry_height == 0
        || proposal.geometry_width > crate::capture::MAX_CAPTURE_WIDTH
        || proposal.geometry_height > crate::capture::MAX_CAPTURE_HEIGHT
    {
        return Err(HostError::Invalid(
            "coordinate geometry out of bounds".into(),
        ));
    }
    if proposal.x < proposal.geometry_x
        || proposal.y < proposal.geometry_y
        || proposal.x >= proposal.geometry_x + proposal.geometry_width as i32
        || proposal.y >= proposal.geometry_y + proposal.geometry_height as i32
    {
        return Err(HostError::Invalid(
            "coordinates outside target window".into(),
        ));
    }
    match &proposal.action {
        CoordinateAction::Click | CoordinateAction::DoubleClick | CoordinateAction::RightClick => {
            Ok(())
        }
        CoordinateAction::Scroll { dx, dy } => {
            if dx.abs() > MAX_SCROLL_DELTA || dy.abs() > MAX_SCROLL_DELTA {
                return Err(HostError::Invalid("coordinate scroll out of bounds".into()));
            }
            if *dx == 0 && *dy == 0 {
                return Err(HostError::Invalid("coordinate null scroll denied".into()));
            }
            Ok(())
        }
        CoordinateAction::Text { text } => {
            if text.is_empty() || text.chars().count() > MAX_INPUT_TEXT_CHARS {
                return Err(HostError::Invalid("coordinate text out of bounds".into()));
            }
            if text
                .chars()
                .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
            {
                return Err(HostError::Invalid("coordinate text control denied".into()));
            }
            Ok(())
        }
    }
}

/// Fresh single-use input lease bound to one proposal and one
/// interruption epoch.
#[derive(Debug, Clone)]
pub struct InputLease {
    pub lease_id: String,
    pub proposal_digest: String,
    pub issued_ms: u64,
    pub interruption_epoch: u64,
    pub consumed: bool,
}

impl InputLease {
    pub fn issue(
        lease_id: &str,
        proposal_digest: &str,
        issued_ms: u64,
        interruption_epoch: u64,
    ) -> Result<Self, HostError> {
        if lease_id.is_empty() || proposal_digest.is_empty() {
            return Err(HostError::Invalid("input lease material malformed".into()));
        }
        Ok(Self {
            lease_id: lease_id.to_owned(),
            proposal_digest: proposal_digest.to_owned(),
            issued_ms,
            interruption_epoch,
            consumed: false,
        })
    }
}

/// Monotonic human-interruption epoch. Physical input increments it
/// exactly once per report; synthetic authority bound to an older
/// epoch fails closed.
#[derive(Debug, Clone, Default)]
pub struct InterruptionTracker {
    pub epoch: u64,
}

impl InterruptionTracker {
    pub fn report_physical_input(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }
}

/// Execute exactly one action under a fresh lease. Consumes the lease.
/// Fails closed on stale proposals, expired or consumed leases,
/// interruption drift, and digest mismatch.
pub fn execute_one(
    proposal: &CoordinateProposal,
    lease: &mut InputLease,
    actual_capture_generation: u64,
    actual_window_generation: u64,
    actual_dpi: u32,
    now_ms: u64,
    current_epoch: u64,
) -> Result<String, HostError> {
    validate_proposal(
        proposal,
        actual_capture_generation,
        actual_window_generation,
        actual_dpi,
    )?;
    if lease.consumed {
        return Err(HostError::Invalid("input lease replay denied".into()));
    }
    if now_ms.saturating_sub(lease.issued_ms) > INPUT_LEASE_TTL_MS {
        return Err(HostError::Invalid("input lease expired".into()));
    }
    if lease.interruption_epoch != current_epoch {
        return Err(HostError::Invalid(
            "human interruption revoked synthetic authority".into(),
        ));
    }
    if lease.proposal_digest != proposal_digest(proposal) {
        return Err(HostError::Invalid("input lease binding mismatch".into()));
    }
    lease.consumed = true;
    Ok(format!("input-{}", proposal_digest(proposal)))
}

/// Denied: unrestricted hotkeys never exist in this grain.
pub fn hotkey_press() -> Result<(), HostError> {
    Err(HostError::Invalid("unrestricted hotkey denied".into()))
}

/// Denied: arbitrary drag never exists in this grain.
pub fn arbitrary_drag() -> Result<(), HostError> {
    Err(HostError::Invalid("arbitrary drag denied".into()))
}

/// Denied: free-form input streams never exist in this grain.
pub fn input_stream() -> Result<(), HostError> {
    Err(HostError::Invalid("free-form input stream denied".into()))
}

/// Denied: clipboard-driven typing never exists in this grain.
pub fn clipboard_typing() -> Result<(), HostError> {
    Err(HostError::Invalid("clipboard typing denied".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal() -> CoordinateProposal {
        CoordinateProposal {
            capture_generation: 4,
            hwnd: 0x00A1B2,
            window_generation: 5,
            geometry_x: 0,
            geometry_y: 0,
            geometry_width: 800,
            geometry_height: 600,
            dpi: 96,
            action: CoordinateAction::Click,
            x: 100,
            y: 200,
        }
    }

    fn lease_for(proposal: &CoordinateProposal, epoch: u64) -> InputLease {
        InputLease::issue("lease-1", &proposal_digest(proposal), 1_000, epoch).unwrap()
    }

    #[test]
    fn bound_proposal_executes_once() {
        let proposal = proposal();
        let mut lease = lease_for(&proposal, 0);
        let receipt = execute_one(&proposal, &mut lease, 4, 5, 96, 2_000, 0).unwrap();
        assert!(!receipt.is_empty());
        assert!(lease.consumed);
        assert!(execute_one(&proposal, &mut lease, 4, 5, 96, 2_000, 0).is_err());
    }

    #[test]
    fn stale_drift_and_outside_coordinates_fail_closed() {
        let proposal = proposal();
        let mut lease = lease_for(&proposal, 0);
        assert!(execute_one(&proposal, &mut lease, 5, 5, 96, 2_000, 0).is_err());
        let mut lease = lease_for(&proposal, 0);
        assert!(execute_one(&proposal, &mut lease, 4, 6, 96, 2_000, 0).is_err());
        let mut lease = lease_for(&proposal, 0);
        assert!(execute_one(&proposal, &mut lease, 4, 5, 120, 2_000, 0).is_err());
        let mut outside = proposal.clone();
        outside.x = 900;
        let mut lease = lease_for(&outside, 0);
        assert!(execute_one(&outside, &mut lease, 4, 5, 96, 2_000, 0).is_err());
        let mut scrolled = proposal.clone();
        scrolled.action = CoordinateAction::Scroll {
            dx: MAX_SCROLL_DELTA + 1,
            dy: 0,
        };
        let mut lease = lease_for(&scrolled, 0);
        assert!(execute_one(&scrolled, &mut lease, 4, 5, 96, 2_000, 0).is_err());
    }

    #[test]
    fn expiry_and_interruption_revoke_authority() {
        let proposal = proposal();
        let mut lease = lease_for(&proposal, 0);
        assert!(execute_one(
            &proposal,
            &mut lease,
            4,
            5,
            96,
            INPUT_LEASE_TTL_MS + 1_001,
            0
        )
        .is_err());
        let mut tracker = InterruptionTracker::default();
        let mut lease = lease_for(&proposal, 0);
        tracker.report_physical_input();
        assert!(execute_one(&proposal, &mut lease, 4, 5, 96, 2_000, tracker.epoch).is_err());
        let mut lease = lease_for(&proposal, tracker.epoch);
        assert!(execute_one(&proposal, &mut lease, 4, 5, 96, 2_000, tracker.epoch).is_ok());
    }

    #[test]
    fn widened_verbs_never_exist() {
        assert!(hotkey_press().is_err());
        assert!(arbitrary_drag().is_err());
        assert!(input_stream().is_err());
        assert!(clipboard_typing().is_err());
        let mut text = proposal();
        text.action = CoordinateAction::Text {
            text: "x".repeat(MAX_INPUT_TEXT_CHARS + 1),
        };
        let mut lease = lease_for(&text, 0);
        assert!(execute_one(&text, &mut lease, 4, 5, 96, 2_000, 0).is_err());
    }
}
