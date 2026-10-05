//! SG-000080 exact-window capture binding and leases.
//!
//! Windows-first read-only capture bound to exact process identity,
//! HWND, window generation, session, geometry, DPI, policy revision,
//! and capture generation. Authority surfaces, credential brokers,
//! consent dialogs, logon UI, secure desktop, foreign sessions, stale
//! and reused handles, and whole-screen capture are denied.
//! CaptureLease is local-issued, exact-window scoped, bounded by time,
//! frames, rate, bytes, and pixels, revocable, and never minted by
//! model or remote callers.

use crate::error::HostError;

/// Maximum capture width in pixels.
pub const MAX_CAPTURE_WIDTH: u32 = 3_840;

/// Maximum capture height in pixels.
pub const MAX_CAPTURE_HEIGHT: u32 = 2_160;

/// Maximum capture payload bytes (width * height * 4 RGBA8).
pub const MAX_CAPTURE_BYTES: usize = 33_177_600;

/// Maximum frames per lease.
pub const MAX_CAPTURE_FRAMES: u32 = 30;

/// Maximum frames per minute per lease (rate bound).
pub const MAX_CAPTURE_RATE_PER_MINUTE: u32 = 20;

/// Maximum lease lifetime in milliseconds.
pub const MAX_CAPTURE_LEASE_MS: u64 = 60_000;

/// Minimum and maximum accepted DPI.
pub const MIN_CAPTURE_DPI: u32 = 48;
pub const MAX_CAPTURE_DPI: u32 = 576;

/// Exact process identity for capture binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureProcess {
    pub pid: u32,
    pub executable_digest: String,
    pub process_generation: u64,
}

/// Exact window identity for capture binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureWindow {
    pub hwnd: u64,
    pub window_generation: u64,
    pub session: String,
}

/// Exact capture geometry with DPI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
}

/// Exact capture target binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureTarget {
    pub process: CaptureProcess,
    pub window: CaptureWindow,
    pub geometry: CaptureGeometry,
    pub policy_revision: u64,
    pub capture_generation: u64,
}

/// Capture scope. Only exact windows are admissible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureScope {
    ExactWindow,
    WholeScreen,
}

/// Protected surface markers that are never captured.
const PROTECTED_MARKERS: &[&str] = &[
    "credentialuibroker",
    "consent",
    "logonui",
    "secure desktop",
    "deskal approval",
    "deskal trust",
    "deskal revoke",
    "windows security",
    "user account control",
];

/// Whether a window class or title names a protected surface.
pub fn is_protected_surface(class: &str, title: &str) -> bool {
    let haystack = format!("{class} {title}").to_ascii_lowercase();
    PROTECTED_MARKERS
        .iter()
        .any(|marker| haystack.contains(marker))
}

/// Validate an exact-window capture target. Fails closed on scope,
/// protected surfaces, stale generations, foreign sessions, geometry
/// and DPI bounds, and malformed identities.
pub fn validate_capture_target(
    target: &CaptureTarget,
    scope: CaptureScope,
    class: &str,
    title: &str,
    actual_window_generation: u64,
    actual_session: &str,
    actual_policy_revision: u64,
) -> Result<(), HostError> {
    if scope != CaptureScope::ExactWindow {
        return Err(HostError::Invalid("whole-screen capture denied".into()));
    }
    if is_protected_surface(class, title) {
        return Err(HostError::Invalid(
            "protected surface capture denied".into(),
        ));
    }
    if target.window.hwnd == 0 {
        return Err(HostError::Invalid("capture window handle malformed".into()));
    }
    if target.window.window_generation != actual_window_generation {
        return Err(HostError::Invalid("stale window generation".into()));
    }
    if target.window.session != actual_session || target.window.session.is_empty() {
        return Err(HostError::Invalid("capture session drift denied".into()));
    }
    if target.policy_revision != actual_policy_revision {
        return Err(HostError::Invalid("capture policy drift denied".into()));
    }
    if target.process.pid == 0 || target.process.executable_digest.is_empty() {
        return Err(HostError::Invalid(
            "capture process identity malformed".into(),
        ));
    }
    crate::observation::validate_identity_field(&target.window.session)?;
    let geometry = &target.geometry;
    if geometry.width == 0
        || geometry.height == 0
        || geometry.width > MAX_CAPTURE_WIDTH
        || geometry.height > MAX_CAPTURE_HEIGHT
    {
        return Err(HostError::Invalid("capture geometry out of bounds".into()));
    }
    let pixels = geometry.width as u64 * geometry.height as u64;
    if pixels * 4 > MAX_CAPTURE_BYTES as u64 {
        return Err(HostError::Invalid("capture pixel bound exceeded".into()));
    }
    if geometry.dpi < MIN_CAPTURE_DPI || geometry.dpi > MAX_CAPTURE_DPI {
        return Err(HostError::Invalid("capture dpi out of bounds".into()));
    }
    Ok(())
}

/// Binding digest over the complete capture target.
pub fn capture_binding_digest(target: &CaptureTarget) -> String {
    let text = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        target.process.pid,
        target.process.executable_digest,
        target.process.process_generation,
        target.window.hwnd,
        target.window.window_generation,
        target.window.session,
        target.geometry.x,
        target.geometry.y,
        target.geometry.width,
        target.geometry.height,
        target.policy_revision
    );
    crate::observation::short_digest(&text)
}

/// Local-issued exact-window capture lease. Issuance requires a
/// server-side binding digest; model and remote callers hold no path
/// to mint one because they cannot produce the server binding.
#[derive(Debug, Clone)]
pub struct CaptureLease {
    pub lease_id: String,
    pub binding_digest: String,
    pub issued_ms: u64,
    pub frames_used: u32,
    pub frame_stamps_ms: Vec<u64>,
    pub revoked: bool,
}

impl CaptureLease {
    /// Issue a lease for a validated binding. Fails closed on empty
    /// binding material.
    pub fn issue(lease_id: &str, binding_digest: &str, issued_ms: u64) -> Result<Self, HostError> {
        if lease_id.is_empty() || binding_digest.is_empty() {
            return Err(HostError::Invalid(
                "capture lease material malformed".into(),
            ));
        }
        Ok(Self {
            lease_id: lease_id.to_owned(),
            binding_digest: binding_digest.to_owned(),
            issued_ms,
            frames_used: 0,
            frame_stamps_ms: Vec::new(),
            revoked: false,
        })
    }

    /// Authorize one frame. Enforces revocation, expiry, frame bound,
    /// byte bound, and rate bound.
    pub fn use_frame(
        &mut self,
        now_ms: u64,
        frame_bytes: usize,
        binding_digest: &str,
    ) -> Result<(), HostError> {
        if self.revoked {
            return Err(HostError::Invalid("capture lease revoked".into()));
        }
        if binding_digest != self.binding_digest {
            return Err(HostError::Invalid("capture lease binding mismatch".into()));
        }
        if now_ms.saturating_sub(self.issued_ms) > MAX_CAPTURE_LEASE_MS {
            return Err(HostError::Invalid("capture lease expired".into()));
        }
        if self.frames_used >= MAX_CAPTURE_FRAMES {
            return Err(HostError::Invalid(
                "capture lease frame bound exceeded".into(),
            ));
        }
        if frame_bytes == 0 || frame_bytes > MAX_CAPTURE_BYTES {
            return Err(HostError::Invalid(
                "capture frame byte bound exceeded".into(),
            ));
        }
        self.frame_stamps_ms
            .retain(|stamp| now_ms.saturating_sub(*stamp) <= 60_000);
        if self.frame_stamps_ms.len() as u32 >= MAX_CAPTURE_RATE_PER_MINUTE {
            return Err(HostError::Invalid(
                "capture lease rate bound exceeded".into(),
            ));
        }
        self.frame_stamps_ms.push(now_ms);
        self.frames_used += 1;
        Ok(())
    }

    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

/// Invalidation events. Every event invalidates volatile capture
/// authority without exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationEvent {
    ProcessReplaced,
    WindowReplaced,
    LockLogoff,
    SessionChange,
    RdpTransition,
    PolicyChange,
    Restart,
}

/// Apply invalidation to a lease. Always revokes.
pub fn invalidate_on(lease: &mut CaptureLease, _event: InvalidationEvent) {
    lease.revoked = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> CaptureTarget {
        CaptureTarget {
            process: CaptureProcess {
                pid: 4242,
                executable_digest: "engine-digest".into(),
                process_generation: 11,
            },
            window: CaptureWindow {
                hwnd: 0x00A1B2,
                window_generation: 5,
                session: "session-1".into(),
            },
            geometry: CaptureGeometry {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
                dpi: 96,
            },
            policy_revision: 9,
            capture_generation: 1,
        }
    }

    #[test]
    fn exact_window_binding_validates() {
        let target = target();
        assert!(validate_capture_target(
            &target,
            CaptureScope::ExactWindow,
            "Chrome_WidgetWin_1",
            "Example",
            5,
            "session-1",
            9
        )
        .is_ok());
        assert!(!capture_binding_digest(&target).is_empty());
    }

    #[test]
    fn protected_surfaces_whole_screen_and_drift_are_denied() {
        let target = target();
        assert!(validate_capture_target(
            &target,
            CaptureScope::WholeScreen,
            "Chrome_WidgetWin_1",
            "Example",
            5,
            "session-1",
            9
        )
        .is_err());
        for (class, title) in [
            ("CredentialUIBroker", "Windows Security"),
            ("ConsentUI", "User Account Control"),
            ("LogonUI", "Sign in"),
            ("Chrome_WidgetWin_1", "Deskal approval request"),
            ("Chrome_WidgetWin_1", "Deskal trust revoke"),
        ] {
            assert!(
                validate_capture_target(
                    &target,
                    CaptureScope::ExactWindow,
                    class,
                    title,
                    5,
                    "session-1",
                    9
                )
                .is_err(),
                "{class} {title}"
            );
        }
        assert!(validate_capture_target(
            &target,
            CaptureScope::ExactWindow,
            "Chrome_WidgetWin_1",
            "Example",
            6,
            "session-1",
            9
        )
        .is_err());
        assert!(validate_capture_target(
            &target,
            CaptureScope::ExactWindow,
            "Chrome_WidgetWin_1",
            "Example",
            5,
            "session-2",
            9
        )
        .is_err());
        assert!(validate_capture_target(
            &target,
            CaptureScope::ExactWindow,
            "Chrome_WidgetWin_1",
            "Example",
            5,
            "session-1",
            10
        )
        .is_err());
        let mut oversized = target.clone();
        oversized.geometry.width = MAX_CAPTURE_WIDTH + 1;
        assert!(validate_capture_target(
            &oversized,
            CaptureScope::ExactWindow,
            "Chrome_WidgetWin_1",
            "Example",
            5,
            "session-1",
            9
        )
        .is_err());
        let mut bad_dpi = target.clone();
        bad_dpi.geometry.dpi = 0;
        assert!(validate_capture_target(
            &bad_dpi,
            CaptureScope::ExactWindow,
            "Chrome_WidgetWin_1",
            "Example",
            5,
            "session-1",
            9
        )
        .is_err());
    }

    #[test]
    fn lease_bounds_replay_and_binding_are_enforced() {
        let digest = capture_binding_digest(&target());
        let mut lease = CaptureLease::issue("lease-1", &digest, 1_000).unwrap();
        assert!(lease.use_frame(2_000, 1024, &digest).is_ok());
        assert!(lease.use_frame(2_000, 1024, "forged-binding").is_err());
        assert!(lease
            .use_frame(2_000, MAX_CAPTURE_BYTES + 1, &digest)
            .is_err());
        for _ in 0..MAX_CAPTURE_FRAMES {
            let _ = lease.use_frame(3_000, 64, &digest);
        }
        assert!(lease.use_frame(3_000, 64, &digest).is_err());
        lease.revoke();
        assert!(lease.use_frame(3_000, 64, &digest).is_err());
        let mut expiring = CaptureLease::issue("lease-2", &digest, 0).unwrap();
        assert!(expiring
            .use_frame(MAX_CAPTURE_LEASE_MS + 1, 64, &digest)
            .is_err());
        assert!(CaptureLease::issue("", &digest, 0).is_err());
    }

    #[test]
    fn rate_bound_and_invalidation_events_revoke() {
        let digest = capture_binding_digest(&target());
        let mut lease = CaptureLease::issue("lease-3", &digest, 10_000).unwrap();
        for offset in 0..MAX_CAPTURE_RATE_PER_MINUTE {
            lease
                .use_frame(10_000 + u64::from(offset), 64, &digest)
                .unwrap();
        }
        assert!(lease.use_frame(10_050, 64, &digest).is_err());
        for event in [
            InvalidationEvent::ProcessReplaced,
            InvalidationEvent::WindowReplaced,
            InvalidationEvent::LockLogoff,
            InvalidationEvent::SessionChange,
            InvalidationEvent::RdpTransition,
            InvalidationEvent::PolicyChange,
            InvalidationEvent::Restart,
        ] {
            let mut lease = CaptureLease::issue("lease-x", &digest, 20_000).unwrap();
            invalidate_on(&mut lease, event);
            assert!(lease.use_frame(20_001, 64, &digest).is_err(), "{event:?}");
        }
    }
}
