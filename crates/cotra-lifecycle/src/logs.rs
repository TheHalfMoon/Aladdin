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
const TRUNCATED: &str = " [TRUNCATED]";
pub const REDACTED: &str = "[REDACTED]";

/// Credential assignment names, longest first so compound names win.
const ASSIGNMENT_MARKERS: &[&str] = &[
    "refresh_token",
    "access_token",
    "authorization",
    "credential",
    "password",
    "api_key",
    "api-key",
    "session",
    "apikey",
    "passwd",
    "secret",
    "cookie",
    "token",
    "pwd",
    "key",
];

/// Prefixes of well-known key formats and the minimum number of key
/// characters that must follow them.
const KEY_PREFIXES: &[(&str, usize)] =
    &[("github_pat_", 16), ("sess-", 8), ("ghp_", 16), ("sk-", 8)];

/// Redacts one log line and bounds its length. Tokens are split on spaces
/// and tabs; within a token every credential assignment (`name=value`,
/// `name:value`, quoted JSON forms) and every key-like value is replaced, and
/// an assignment whose value follows the separator in the next token (for
/// example `password: secret` or pretty-printed JSON) redacts that token.
pub fn redact_line(line: &str) -> String {
    let mut out = Vec::new();
    let mut pending = false;
    let mut awaiting_separator = false;
    for token in line.split([' ', '\t']) {
        if token.is_empty() {
            out.push(String::new());
            continue;
        }
        if awaiting_separator && matches!(token, "=" | ":" | "=>") {
            out.push(token.to_string());
            awaiting_separator = false;
            pending = true;
            continue;
        }
        awaiting_separator = false;
        let lower = token.to_ascii_lowercase();
        let scheme = lower == "bearer" || lower == "basic";
        if scheme {
            out.push(token.to_string());
            pending = true;
            continue;
        }
        if pending {
            out.push(REDACTED.to_string());
            pending = false;
            continue;
        }
        let (redacted, value_follows) = redact_token(token);
        out.push(redacted);
        pending = value_follows;
        let bare = lower.trim_matches(|c: char| c == '"' || c == '\'');
        // An option such as `--api-key` or `--control-plane.api-key` whose
        // value is the next token.
        let option_name = bare
            .strip_prefix('-')
            .filter(|_| !bare.contains('='))
            .and_then(|name| name.rsplit(['.', '-', '_']).next());
        if !pending
            && option_name.is_some_and(|last| marker_len(last.as_bytes()) == Some(last.len()))
        {
            pending = true;
        }
        awaiting_separator = !pending && marker_len(bare.as_bytes()) == Some(bare.len());
    }
    let joined = out.join(" ");
    if joined.chars().count() > MAX_LINE_CHARS {
        joined
            .chars()
            .take(MAX_LINE_CHARS - TRUNCATED.len())
            .collect::<String>()
            + TRUNCATED
    } else {
        joined
    }
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn is_quote(byte: u8) -> bool {
    byte == b'"' || byte == b'\''
}

fn key_like_len(rest: &[u8]) -> Option<usize> {
    KEY_PREFIXES.iter().find_map(|(prefix, minimum)| {
        let prefix = prefix.as_bytes();
        if !rest.starts_with(prefix) {
            return None;
        }
        let body = rest[prefix.len()..]
            .iter()
            .take_while(|byte| is_word(**byte) || **byte == b'.')
            .count();
        (body >= *minimum).then_some(prefix.len() + body)
    })
}

fn marker_len(rest: &[u8]) -> Option<usize> {
    ASSIGNMENT_MARKERS.iter().find_map(|marker| {
        let marker = marker.as_bytes();
        let bounded =
            rest.starts_with(marker) && rest.get(marker.len()).is_none_or(|next| !is_word(*next));
        bounded.then_some(marker.len())
    })
}

/// Redacts every credential assignment and key-like value in one
/// whitespace-free token. Returns the redacted token and whether its last
/// assignment has no value in this token (so the next token is the value).
fn redact_token(token: &str) -> (String, bool) {
    let lower = token.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut out = String::with_capacity(token.len());
    let mut copied = 0;
    let mut index = 0;
    while index < bytes.len() {
        // `_` and `-` separate name parts (OPENAI_API_KEY, --api-key), so only
        // an alphanumeric predecessor means the match is inside a word.
        let at_boundary = index == 0 || !bytes[index - 1].is_ascii_alphanumeric();
        if at_boundary {
            if let Some(length) = key_like_len(&bytes[index..]) {
                out.push_str(&token[copied..index]);
                out.push_str(REDACTED);
                index += length;
                copied = index;
                continue;
            }
            if let Some(length) = marker_len(&bytes[index..]) {
                let mut separator = index + length;
                while separator < bytes.len() && is_quote(bytes[separator]) {
                    separator += 1;
                }
                if separator < bytes.len() && (bytes[separator] == b'=' || bytes[separator] == b':')
                {
                    let mut start = separator + 1;
                    while start < bytes.len() && is_quote(bytes[start]) {
                        start += 1;
                    }
                    if start >= bytes.len() {
                        out.push_str(&token[copied..]);
                        return (out, true);
                    }
                    let mut end = start;
                    while end < bytes.len()
                        && !matches!(
                            bytes[end],
                            b'&' | b',' | b';' | b'"' | b'\'' | b'}' | b')' | b']'
                        )
                    {
                        end += 1;
                    }
                    out.push_str(&token[copied..start]);
                    out.push_str(REDACTED);
                    index = end;
                    copied = end;
                    continue;
                }
            }
        }
        index += 1;
    }
    out.push_str(&token[copied..]);
    (out, false)
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

    fn assert_hidden(line: &str, secrets: &[&str]) -> String {
        let out = redact_line(line);
        for secret in secrets {
            assert!(
                !out.contains(secret),
                "{secret} survived in {out:?} (from {line:?})"
            );
        }
        out
    }

    #[test]
    fn key_like_tokens_and_assignments_are_redacted() {
        let out = assert_hidden(
            "connect api_key=sk-proj-abcdef123456 token: abc Authorization: Bearer xyz.123 key sk-1234567890ab ok",
            &["sk-proj-abcdef123456", "abc", "xyz.123", "sk-1234567890ab"],
        );
        assert!(out.contains("api_key=[REDACTED]"), "{out}");
        assert!(out.starts_with("connect"));
        assert!(out.ends_with(" ok"));
        assert_hidden(r#"{"password":"hunter22","user":"u"}"#, &["hunter22"]);
    }

    #[test]
    fn spaced_tabbed_pretty_and_later_assignments_are_redacted() {
        assert_hidden("password: hunter22", &["hunter22"]);
        assert_hidden("password = hunter22", &["hunter22"]);
        assert_hidden(r#"  "password": "hunter22","#, &["hunter22"]);
        assert_hidden("value	sk-abcdefghijklmnop", &["sk-abcdefghijklmnop"]);
        assert_hidden("value	sk-abcdefghijklmnop", &["sk-abcdefghijklmnop"]);
        assert_hidden("GET /cb?x=1&token=abc123&y=2", &["abc123"]);
        assert_hidden(r#"{"user":"u","password":"hunter22"}"#, &["hunter22"]);
        assert_hidden("key=sk-proj-abcdef123456", &["sk-proj-abcdef123456"]);
        assert_hidden("session=s3cr3t-value;path=/", &["s3cr3t-value"]);
        assert_hidden("Authorization: Basic dXNlcjpwYXNz", &["dXNlcjpwYXNz"]);
        assert_hidden("cookie: id=abc", &["id=abc"]);
        assert_hidden(
            "OPENAI_API_KEY=sk-proj-abcdef123456",
            &["sk-proj-abcdef123456"],
        );
        assert_hidden("env OPENAI_API_KEY=plainsecret", &["plainsecret"]);
        assert_hidden("--api-key=plainsecret", &["plainsecret"]);
        assert_hidden("--api-key plainsecret", &["plainsecret"]);
        assert_hidden(
            r"--control-plane.api-key file:C:\keys\k",
            &[r"file:C:\keys\k"],
        );
        assert_eq!(
            redact_line(r"--health.url-file C:\run\h.url"),
            r"--health.url-file C:\run\h.url"
        );
        assert_hidden("CONTROL_PLANE_TOKEN: plainsecret", &["plainsecret"]);
    }

    #[test]
    fn ordinary_words_containing_markers_are_not_redacted() {
        for line in [
            "monkey=banana",
            "tokens=5 keys=3",
            "turkey: roasted",
            "tunnel connected to control plane in 120ms",
        ] {
            assert_eq!(redact_line(line), line);
        }
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
