//! SG-000074 error vocabulary: typed unavailability and fail-closed errors.
//!
//! A missing or unsupported browser engine is never an exception and never
//! a fallback: it is [`HostError::Unavailable`], and callers must surface
//! it as typed unavailable.

use std::fmt;

/// Why a browser engine cannot be used. Variants are exhaustive so new
/// failure modes force an explicit decision at every match site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    /// No supported engine executable was discovered.
    NoEngine,
    /// An engine-like executable failed version, publisher, or kind checks.
    UnsupportedEngine,
    /// A supervised launch or handshake step failed.
    LaunchFailed,
    /// The host channel carried malformed or unexpected frames.
    ProtocolViolation,
}

impl fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEngine => write!(f, "no supported browser engine is installed"),
            Self::UnsupportedEngine => write!(f, "browser engine is unsupported or untrusted"),
            Self::LaunchFailed => write!(f, "browser host launch failed"),
            Self::ProtocolViolation => write!(f, "browser host protocol violation"),
        }
    }
}

/// Every failure of the browser host. No variant widens authority.
#[derive(Debug)]
pub enum HostError {
    Unavailable(UnavailableReason),
    Invalid(String),
    Platform(String),
    Io(std::io::Error),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "browser unavailable: {reason}"),
            Self::Invalid(detail) => write!(f, "invalid browser host input: {detail}"),
            Self::Platform(detail) => write!(f, "browser host platform failure: {detail}"),
            Self::Io(err) => write!(f, "browser host I/O failure: {err}"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_reasons_render_without_authority() {
        for reason in [
            UnavailableReason::NoEngine,
            UnavailableReason::UnsupportedEngine,
            UnavailableReason::LaunchFailed,
            UnavailableReason::ProtocolViolation,
        ] {
            let text = HostError::Unavailable(reason).to_string();
            assert!(text.starts_with("browser unavailable: "));
        }
    }
}
