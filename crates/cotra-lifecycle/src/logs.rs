//! Bounded, redacted lifecycle logs.
//!
//! Tunnel-client output is written line by line through [`redact_line`],
//! which replaces key-like tokens and credential assignments. The redactor
//! never reads the runtime key file, so the key itself cannot be copied into a
//! log by the redaction logic.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MAX_LOG_BYTES: u64 = 1024 * 1024;
const MAX_LINE_CHARS: usize = 4096;
pub const REDACTED: &str = "[REDACTED]";

const ASSIGNMENT_MARKERS: &[&str] = &[
    "api_key",
    "api-key",
    "apikey",
    "access_token",
    "token",
    "secret",
    "password",
    "passwd",
    "authorization",
    "credential",
];

/// Redacts one log line and bounds its length.
pub fn redact_line(line: &str) -> String {
    let mut out = Vec::new();
    let mut redact_next = false;
    for token in line.split(' ') {
        if redact_next && !token.is_empty() {
            out.push(REDACTED.to_string());
            redact_next = false;
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if lower == "bearer" || lower == "basic" {
            redact_next = true;
            out.push(token.to_string());
            continue;
        }
        out.push(redact_token(token, &lower));
    }
    let mut joined = out.join(" ");
    if joined.chars().count() > MAX_LINE_CHARS {
        joined = joined.chars().take(MAX_LINE_CHARS).collect::<String>() + " [TRUNCATED]";
    }
    joined
}

fn redact_token(token: &str, lower: &str) -> String {
    let trimmed = lower.trim_matches(|c: char| c == '"' || c == '\'' || c == ',');
    if looks_like_key(trimmed) {
        return REDACTED.to_string();
    }
    for separator in ['=', ':'] {
        if let Some(index) = lower.find(separator) {
            let name = lower[..index].trim_matches(|c: char| c == '"' || c == '\'' || c == '-');
            if ASSIGNMENT_MARKERS
                .iter()
                .any(|marker| name.ends_with(marker))
                && index + 1 < token.len()
            {
                return format!("{}{REDACTED}", &token[..=index]);
            }
        }
    }
    token.to_string()
}

fn looks_like_key(token: &str) -> bool {
    (token.starts_with("sk-") && token.len() >= 11)
        || (token.starts_with("sess-") && token.len() >= 13)
        || (token.starts_with("ghp_") && token.len() >= 20)
        || (token.starts_with("github_pat_") && token.len() >= 20)
}

/// An append-only log file with one rotated generation.
pub struct BoundedLog {
    path: PathBuf,
    file: File,
    written: u64,
}

impl BoundedLog {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let written = file.metadata()?.len();
        Ok(Self {
            path: path.to_path_buf(),
            file,
            written,
        })
    }

    pub fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        let redacted = redact_line(line);
        let bytes = redacted.len() as u64 + 1;
        if self.written + bytes > MAX_LOG_BYTES {
            self.rotate()?;
        }
        writeln!(self.file, "{redacted}")?;
        self.written += bytes;
        Ok(())
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        let rotated = self.path.with_extension("log.1");
        let _ = fs::remove_file(&rotated);
        fs::rename(&self.path, &rotated)?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.written = 0;
        Ok(())
    }
}

/// Returns up to `max_lines` final lines of a log file.
pub fn tail(path: &Path, max_lines: usize) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines = text.lines().collect::<Vec<_>>();
    lines[lines.len().saturating_sub(max_lines)..]
        .iter()
        .map(|line| line.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn key_like_tokens_and_assignments_are_redacted() {
        let line = "connect api_key=sk-proj-abcdef123456 token: abc Authorization: Bearer xyz.123 key sk-1234567890ab ok";
        let out = redact_line(line);
        assert!(!out.contains("sk-proj-abcdef123456"), "{out}");
        assert!(!out.contains("xyz.123"), "{out}");
        assert!(!out.contains("sk-1234567890ab"), "{out}");
        assert!(out.contains("api_key=[REDACTED]"), "{out}");
        assert!(out.contains("connect"));
        assert!(out.contains(" ok"));
        let json = redact_line(r#"{"password":"hunter22","user":"u"}"#);
        assert!(!json.contains("hunter22"), "{json}");
    }

    #[test]
    fn ordinary_lines_are_unchanged_and_long_lines_are_bounded() {
        let line = "tunnel connected to control plane in 120ms";
        assert_eq!(redact_line(line), line);
        let long = "x".repeat(10_000);
        assert!(redact_line(&long).len() < 4200);
    }

    #[test]
    fn log_rotates_at_the_bound() {
        let dir = temp_dir("logs");
        let path = dir.join("tunnel.log");
        let mut log = BoundedLog::open(&path).unwrap();
        let line = "y".repeat(4000);
        for _ in 0..300 {
            log.write_line(&line).unwrap();
        }
        assert!(fs::metadata(&path).unwrap().len() <= MAX_LOG_BYTES);
        assert!(dir.join("tunnel.log.1").is_file());
        assert_eq!(tail(&path, 2).len(), 2);
    }
}
