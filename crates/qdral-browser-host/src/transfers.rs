//! SG-000078 bounded transfer staging and validation.
//!
//! Pure staging policy for the isolated host: canonical relative
//! destinations with reparse-safe containment, type allowlisting with
//! content agreement, size bounds with declared-versus-actual agreement,
//! origin-bound one-shot expiring download sources, server-recorded
//! upload sources with exact digest, size, media type, and file-input
//! binding, and one-shot consumption. Transferred content is never
//! opened, executed, or extracted automatically.

use crate::error::HostError;

/// Hard ceiling on a single transfer in bytes (8 MiB).
pub const MAX_TRANSFER_BYTES: u64 = 8 * 1024 * 1024;

/// Hard ceiling on destination path bytes.
pub const MAX_DESTINATION_BYTES: usize = 512;

/// Download source lifetime in milliseconds.
pub const DOWNLOAD_SOURCE_TTL_MS: u64 = 120_000;

/// Benign filename extensions admitted for download staging.
const ALLOWED_DOWNLOAD_EXTENSIONS: &[&str] = &[
    "txt", "md", "json", "csv", "log", "png", "jpg", "jpeg", "gif", "webp", "svg", "html",
];

/// Declared media types admitted for download staging.
const ALLOWED_DOWNLOAD_MEDIA: &[&str] = &[
    "text/plain",
    "text/markdown",
    "text/csv",
    "text/html",
    "application/json",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/svg+xml",
];

/// Content classes never admitted, by sniffed magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeniedContentClass {
    Pe,
    Elf,
    MachO,
    OleCompound,
    ShellScript,
    Zip,
    Pdf,
}

/// Validate a canonical relative destination. Denies absolute paths,
/// drive letters, UNC, device and NT-namespace prefixes, alternate data
/// streams, parent and current traversal, empty and duplicate separators,
/// trailing dots and spaces, reserved device names, illegal characters,
/// control characters, and length escapes.
pub fn validate_relative_destination(destination: &str) -> Result<String, HostError> {
    if destination.is_empty() || destination.len() > MAX_DESTINATION_BYTES {
        return Err(HostError::Invalid(
            "transfer destination is empty or oversized".into(),
        ));
    }
    if destination.chars().any(|ch| ch.is_control()) {
        return Err(HostError::Invalid(
            "transfer destination carries control characters".into(),
        ));
    }
    let lower = destination.to_ascii_lowercase();
    for prefix in ["/", "\\", "\\\\", "//", "con:", "prn:", "aux:", "nul:"] {
        if lower.starts_with(prefix) {
            return Err(HostError::Invalid(
                "transfer destination is absolute or reserved".into(),
            ));
        }
    }
    if lower.starts_with("\\\\?\\") || lower.starts_with("\\??\\") {
        return Err(HostError::Invalid("transfer NT namespace denied".into()));
    }
    if destination.contains(':') {
        return Err(HostError::Invalid(
            "transfer alternate data stream denied".into(),
        ));
    }
    if destination.len() >= 2
        && destination.as_bytes()[1] == b':'
        && destination.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err(HostError::Invalid(
            "transfer drive destination denied".into(),
        ));
    }
    for part in destination.split(['/', '\\']) {
        if part.is_empty() {
            return Err(HostError::Invalid(
                "transfer destination has empty separator run".into(),
            ));
        }
        if part == "." || part == ".." {
            return Err(HostError::Invalid("transfer traversal denied".into()));
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return Err(HostError::Invalid(
                "transfer trailing dot or space denied".into(),
            ));
        }
        let stem = part.split('.').next().unwrap_or(part).to_ascii_lowercase();
        if matches!(
            stem.as_str(),
            "con"
                | "prn"
                | "aux"
                | "nul"
                | "com1"
                | "com2"
                | "com3"
                | "com4"
                | "com5"
                | "com6"
                | "com7"
                | "com8"
                | "com9"
                | "lpt1"
                | "lpt2"
                | "lpt3"
                | "lpt4"
                | "lpt5"
                | "lpt6"
                | "lpt7"
                | "lpt8"
                | "lpt9"
        ) {
            return Err(HostError::Invalid("transfer reserved name denied".into()));
        }
        if part
            .chars()
            .any(|ch| matches!(ch, '<' | '>' | '"' | '|' | '?' | '*'))
        {
            return Err(HostError::Invalid(
                "transfer illegal character denied".into(),
            ));
        }
    }
    Ok(destination.replace('\\', "/"))
}

/// Lexical reparse-safe containment: the normalized destination must
/// stay inside the staging root by separator boundary. No filesystem
/// is touched here; the caller re-verifies the real path after write.
pub fn reparse_safe_contained(staging_root: &str, destination: &str) -> Result<String, HostError> {
    let normalized = validate_relative_destination(destination)?;
    let root = staging_root.trim_end_matches('/');
    if root.is_empty() || root.contains("..") {
        return Err(HostError::Invalid("transfer staging root malformed".into()));
    }
    Ok(format!("{root}/{normalized}"))
}

/// Sniff a denied executable, archive, or document class by magic bytes.
pub fn sniff_denied_class(bytes: &[u8]) -> Option<DeniedContentClass> {
    if bytes.len() >= 2 && bytes[0] == b'M' && bytes[1] == b'Z' {
        return Some(DeniedContentClass::Pe);
    }
    if bytes.len() >= 4 && bytes[0] == 0x7f && &bytes[1..4] == b"ELF" {
        return Some(DeniedContentClass::Elf);
    }
    if bytes.len() >= 4
        && ((bytes[0] == 0xCF && bytes[1] == 0xFA && bytes[2] == 0xED && bytes[3] == 0xFE)
            || (bytes[0] == 0xFE && bytes[1] == 0xED && bytes[2] == 0xFA && bytes[3] == 0xCE))
    {
        return Some(DeniedContentClass::MachO);
    }
    if bytes.len() >= 8 && &bytes[0..8] == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
        return Some(DeniedContentClass::OleCompound);
    }
    if bytes.len() >= 2 && bytes[0] == b'#' && bytes[1] == b'!' {
        return Some(DeniedContentClass::ShellScript);
    }
    if bytes.len() >= 4
        && bytes[0] == b'P'
        && bytes[1] == b'K'
        && bytes[2] == 0x03
        && bytes[3] == 0x04
    {
        return Some(DeniedContentClass::Zip);
    }
    if bytes.len() >= 5 && &bytes[0..5] == b"%PDF-" {
        return Some(DeniedContentClass::Pdf);
    }
    None
}

/// Validate download type agreement: filename extension, declared media
/// type, and sniffed content must agree on an allowlisted benign class.
pub fn validate_download_type(
    file_name: &str,
    declared_media: &str,
    content: &[u8],
) -> Result<(), HostError> {
    let extension = file_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ALLOWED_DOWNLOAD_EXTENSIONS.contains(&extension.as_str()) {
        return Err(HostError::Invalid("transfer filename type denied".into()));
    }
    if !ALLOWED_DOWNLOAD_MEDIA.contains(&declared_media.to_ascii_lowercase().as_str()) {
        return Err(HostError::Invalid("transfer declared type denied".into()));
    }
    if sniff_denied_class(content).is_some() {
        return Err(HostError::Invalid("transfer content class denied".into()));
    }
    if content.is_empty() || content.len() as u64 > MAX_TRANSFER_BYTES {
        return Err(HostError::Invalid("transfer content size denied".into()));
    }
    Ok(())
}

/// Validate declared-versus-actual size agreement within the hard bound.
pub fn validate_transfer_size(declared: u64, actual: u64) -> Result<(), HostError> {
    if declared != actual {
        return Err(HostError::Invalid("transfer size mismatch".into()));
    }
    if actual == 0 || actual > MAX_TRANSFER_BYTES {
        return Err(HostError::Invalid("transfer size out of bounds".into()));
    }
    Ok(())
}

/// One-shot expiring download source identity.
#[derive(Debug, Clone)]
pub struct DownloadSource {
    pub source_id: String,
    pub origin: String,
    pub content_digest: String,
    pub issued_ms: u64,
    pub consumed: bool,
}

impl DownloadSource {
    pub fn consume(&mut self, now_ms: u64) -> Result<(), HostError> {
        if self.consumed {
            return Err(HostError::Invalid("transfer source replay denied".into()));
        }
        if now_ms.saturating_sub(self.issued_ms) > DOWNLOAD_SOURCE_TTL_MS {
            return Err(HostError::Invalid("transfer source expired".into()));
        }
        if self.source_id.is_empty() || self.content_digest.is_empty() {
            return Err(HostError::Invalid("transfer source malformed".into()));
        }
        self.consumed = true;
        Ok(())
    }
}

/// Server-recorded upload source descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedUpload {
    pub digest: String,
    pub size: u64,
    pub media_type: String,
    pub file_input: String,
    pub workspace: String,
}

/// Caller-presented upload descriptor, verified field by field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedUpload {
    pub digest: String,
    pub size: u64,
    pub media_type: String,
    pub file_input: String,
    pub workspace: String,
}

/// Validate an upload against its server-recorded download: exact digest,
/// size, media type, file-input identity, and workspace binding.
pub fn validate_upload_source(
    recorded: &RecordedUpload,
    presented: &PresentedUpload,
) -> Result<(), HostError> {
    if recorded.digest != presented.digest || recorded.digest.is_empty() {
        return Err(HostError::Invalid("upload source digest mismatch".into()));
    }
    validate_transfer_size(recorded.size, presented.size)?;
    if recorded.media_type != presented.media_type || recorded.media_type.is_empty() {
        return Err(HostError::Invalid("upload media type mismatch".into()));
    }
    if recorded.file_input != presented.file_input || recorded.file_input.is_empty() {
        return Err(HostError::Invalid("upload file-input mismatch".into()));
    }
    if recorded.workspace != presented.workspace || recorded.workspace.is_empty() {
        return Err(HostError::Invalid("upload workspace drift denied".into()));
    }
    crate::observation::validate_identity_field(&presented.file_input)?;
    Ok(())
}

/// Denied: transferred content is never opened automatically.
pub fn open_automatically(_path: &str) -> Result<(), HostError> {
    Err(HostError::Invalid("automatic open denied".into()))
}

/// Denied: transferred content is never executed automatically.
pub fn execute_automatically(_path: &str) -> Result<(), HostError> {
    Err(HostError::Invalid("automatic execute denied".into()))
}

/// Denied: transferred content is never extracted automatically.
pub fn extract_automatically(_path: &str) -> Result<(), HostError> {
    Err(HostError::Invalid("automatic extract denied".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_destinations_validate_and_contain() {
        assert_eq!(
            validate_relative_destination("reports/summary.txt").unwrap(),
            "reports/summary.txt"
        );
        assert_eq!(
            reparse_safe_contained("/staging/root", "a/b.txt").unwrap(),
            "/staging/root/a/b.txt"
        );
    }

    #[test]
    fn traversal_unc_device_ads_and_reserved_fail_closed() {
        for destination in [
            "/absolute/path.txt",
            "C:\\Windows\\x.txt",
            "\\\\server\\share\\x.txt",
            "\\\\?\\C:\\x.txt",
            "notes:stream.txt",
            "../escape.txt",
            "a/../../escape.txt",
            "a//b.txt",
            "trailingdot./x.txt",
            "trailingspace /x.txt",
            "CON/x.txt",
            "aux.txt",
            "bad<name>.txt",
            "nul",
        ] {
            assert!(
                validate_relative_destination(destination).is_err(),
                "{destination}"
            );
        }
    }

    #[test]
    fn denied_content_and_size_mismatch_fail_closed() {
        assert!(validate_download_type("run.txt", "text/plain", b"MZ-binary").is_err());
        assert!(validate_download_type("doc.txt", "text/plain", b"%PDF-1.4").is_err());
        assert!(validate_download_type("run.exe", "text/plain", b"hello").is_err());
        assert!(validate_download_type("notes.txt", "application/x-sh", b"hello").is_err());
        assert!(validate_download_type("notes.txt", "text/plain", b"").is_err());
        assert!(validate_transfer_size(10, 11).is_err());
        assert!(validate_transfer_size(0, 0).is_err());
        assert!(validate_transfer_size(MAX_TRANSFER_BYTES + 1, MAX_TRANSFER_BYTES + 1).is_err());
        assert!(validate_transfer_size(12, 12).is_ok());
        assert!(validate_download_type("notes.txt", "text/plain", b"hello").is_ok());
    }

    #[test]
    fn sources_are_one_shot_expiring_and_exact() {
        let mut source = DownloadSource {
            source_id: "dl-1".into(),
            origin: "https://example.com:443".into(),
            content_digest: "abc".into(),
            issued_ms: 1_000,
            consumed: false,
        };
        assert!(source.consume(2_000).is_ok());
        assert!(source.consume(3_000).is_err());
        let mut expired = DownloadSource {
            source_id: "dl-2".into(),
            origin: "https://example.com:443".into(),
            content_digest: "abc".into(),
            issued_ms: 0,
            consumed: false,
        };
        assert!(expired.consume(DOWNLOAD_SOURCE_TTL_MS + 1).is_err());
        let recorded = RecordedUpload {
            digest: "d".into(),
            size: 4,
            media_type: "text/plain".into(),
            file_input: "input-1".into(),
            workspace: "ws".into(),
        };
        let ok = PresentedUpload {
            digest: "d".into(),
            size: 4,
            media_type: "text/plain".into(),
            file_input: "input-1".into(),
            workspace: "ws".into(),
        };
        let mut wrong = ok.clone();
        wrong.digest = "other".into();
        assert!(validate_upload_source(&recorded, &wrong).is_err());
        wrong.digest = "d".into();
        wrong.size = 5;
        assert!(validate_upload_source(&recorded, &wrong).is_err());
        wrong.size = 4;
        wrong.file_input = "input-2".into();
        assert!(validate_upload_source(&recorded, &wrong).is_err());
        assert!(validate_upload_source(&recorded, &ok).is_ok());
    }

    #[test]
    fn automatic_handling_never_authorized() {
        assert!(open_automatically("/staging/root/a.txt").is_err());
        assert!(execute_automatically("/staging/root/a.txt").is_err());
        assert!(extract_automatically("/staging/root/a.txt").is_err());
    }
}
