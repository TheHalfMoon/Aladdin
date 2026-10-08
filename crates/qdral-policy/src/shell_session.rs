//! SG-000096 T01: pure shell-session intent and approval-digest contract.
//!
//! This module NEVER executes a command, mints authority, or consumes an approval.
//! Hashes bind proposed intent bytes only; they are not approval tokens. Active
//! ShellProcess execution stays disabled until separately qualified successors.

use crate::full_control::{
    check_full_control, executor_enabled_now, AuthorityContext, AuthorityMode, ExecutorClass,
    FullControlLease,
};
use crate::PolicyError;
use qdral_contracts::FailureCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub const SHELL_SESSION_INTENT_SCHEMA: &str = "deskal-shell-session-intent/1";
const APPROVAL_DOMAIN: &[u8] = b"deskal:sg-000096:shell-session-approval:v1\0";
const MAX_COMMAND_BYTES: usize = 64 * 1024;
const MAX_ARGUMENTS: usize = 64;
const MAX_ARGUMENT_BYTES: usize = 8192;
const MAX_ENV_IDENTIFIERS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellIntentPhase {
    Proposed,
    Authorized,
    Executed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellOperation {
    Spawn,
    WriteStdin,
    Terminate,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellKind {
    Direct,
    PowerShell,
    Cmd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellSessionLimits {
    pub timeout_ms: u64,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdin_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellSessionIntent {
    pub schema: String,
    pub phase: ShellIntentPhase,
    pub operation: ShellOperation,
    /// An opaque, server-issued identity. Possession does not grant authority.
    pub session_handle: String,
    pub shell_kind: ShellKind,
    pub executable_path: String,
    pub executable_sha256: String,
    pub argv: Vec<String>,
    pub command: String,
    /// Lexically absolute; native canonicalization and reparse checks are T02.
    pub cwd: String,
    /// Keys and value digests only, never environment secrets or plaintext.
    pub environment_value_digests: BTreeMap<String, String>,
    pub limits: ShellSessionLimits,
    pub workspace_id: String,
    pub policy_revision: String,
    pub windows_user_sid: String,
    pub logon_session_id: u64,
    pub device_id: String,
    pub deskal_session_id: String,
    pub authority_epoch: u64,
    pub owner_process_generation: u64,
    pub session_generation: u64,
    pub expected_process_generation: u64,
    /// Fresh, server-provided nonce; a digest alone does not consume it.
    pub approval_nonce: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ExpectedShellBinding<'a> {
    pub session_handle: &'a str,
    pub workspace_id: &'a str,
    pub policy_revision: &'a str,
    pub windows_user_sid: &'a str,
    pub logon_session_id: u64,
    pub device_id: &'a str,
    pub deskal_session_id: &'a str,
    pub authority_epoch: u64,
    pub owner_process_generation: u64,
    pub session_generation: u64,
    pub expected_process_generation: u64,
    pub approval_nonce: &'a str,
    pub operation: ShellOperation,
    pub approval_digest: &'a str,
}

fn invalid(message: &'static str) -> PolicyError {
    PolicyError {
        code: FailureCode::InvalidRequest,
        message: message.into(),
    }
}

fn denied(message: &'static str) -> PolicyError {
    PolicyError {
        code: FailureCode::CapabilityDenied,
        message: message.into(),
    }
}

fn field(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && !value
            .chars()
            .any(|c| c.is_control() || unsafe_format_char(c))
}

fn unsafe_format_char(c: char) -> bool {
    matches!(
        c,
        '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200b}'..='\u{200f}' | '\u{feff}'
    )
}

fn valid_nonce(value: &str) -> bool {
    (value.len() == 32 || value.len() == 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn opaque_identity(value: &str) -> bool {
    value.len() == 35
        && value.starts_with("sh_")
        && value.as_bytes()[3..].iter().all(u8::is_ascii_hexdigit)
        && value.as_bytes()[3..]
            .iter()
            .all(|b| !b.is_ascii_uppercase())
}

fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn lexical_windows_absolute(value: &str) -> bool {
    let raw = value.as_bytes();
    if !field(value, 4096)
        || raw.len() < 3
        || !raw[0].is_ascii_alphabetic()
        || raw[1] != b':'
        || raw[2] != b'\\'
        || value.contains('/')
        || value.ends_with(' ')
        || value.contains(':') && value[2..].contains(':')
    {
        return false;
    }
    // Reject drive-relative paths, UNC, dot segments and ambiguous whitespace.
    value.len() == 3
        || value[3..].split('\\').all(|s| {
            !s.is_empty()
                && !matches!(s, "." | "..")
                && !s.ends_with(' ')
                && !s.ends_with('.')
                && !s
                    .chars()
                    .any(|c| matches!(c, '<' | '>' | '"' | '|' | '?' | '*'))
        })
}

pub fn validate_shell_session_intent(intent: &ShellSessionIntent) -> Result<(), PolicyError> {
    if intent.schema != SHELL_SESSION_INTENT_SCHEMA {
        return Err(invalid("unknown shell-session intent schema"));
    }
    if intent.phase != ShellIntentPhase::Proposed {
        return Err(denied(
            "authorized/executed shell phases cannot come from intent",
        ));
    }
    if !opaque_identity(&intent.session_handle)
        || !hex_digest(&intent.executable_sha256)
        || !lexical_windows_absolute(&intent.executable_path)
        || !lexical_windows_absolute(&intent.cwd)
        || !field(&intent.workspace_id, 256)
        || !field(&intent.policy_revision, 256)
        || !field(&intent.windows_user_sid, 256)
        || !intent.windows_user_sid.starts_with("S-1-")
        || !field(&intent.device_id, 256)
        || !field(&intent.deskal_session_id, 256)
        || !valid_nonce(&intent.approval_nonce)
    {
        return Err(invalid("invalid shell-session identity, path or nonce"));
    }
    if intent.logon_session_id == 0
        || intent.authority_epoch == 0
        || intent.owner_process_generation == 0
        || intent.session_generation == 0
        || intent.expected_process_generation == 0
    {
        return Err(invalid(
            "shell-session generation, epoch and logon must be nonzero",
        ));
    }
    if intent.argv.len() > MAX_ARGUMENTS
        || intent
            .argv
            .iter()
            .any(|arg| !field(arg, MAX_ARGUMENT_BYTES))
        || intent.command.len() > MAX_COMMAND_BYTES
        || intent.command.chars().any(|c| {
            c == '\0'
                || unsafe_format_char(c)
                || (c.is_control() && c != '\n' && c != '\r' && c != '\t')
        })
    {
        return Err(invalid("shell argument or command is out of bounds"));
    }
    if matches!(intent.shell_kind, ShellKind::Direct) {
        if !intent.command.is_empty() {
            return Err(invalid(
                "direct execution must bind argv, not an implicit shell command",
            ));
        }
    } else if intent.command.is_empty() || !intent.argv.is_empty() {
        return Err(invalid(
            "shell execution requires exact command and no implicit argv",
        ));
    }
    if intent.environment_value_digests.len() > MAX_ENV_IDENTIFIERS
        || intent.environment_value_digests.iter().any(|(key, value)| {
            !field(key, 128)
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'(' | b')'))
                || !hex_digest(value)
        })
    {
        return Err(invalid("environment identity digests are invalid"));
    }
    let mut env_names = std::collections::BTreeSet::new();
    for name in intent.environment_value_digests.keys() {
        if !env_names.insert(name.to_ascii_lowercase()) {
            return Err(invalid("duplicate case-insensitive environment identity"));
        }
    }
    let limits = &intent.limits;
    if limits.timeout_ms == 0
        || limits.timeout_ms > 30 * 60 * 1000
        || limits.stdout_bytes == 0
        || limits.stdout_bytes > 16 * 1024 * 1024
        || limits.stderr_bytes == 0
        || limits.stderr_bytes > 4 * 1024 * 1024
        || limits.stdin_bytes == 0
        || limits.stdin_bytes > 256 * 1024
    {
        return Err(invalid("shell-session resource limits are out of bounds"));
    }
    Ok(())
}

/// Stable, domain-separated, length-prefixed intent commitment. Not a grant.
pub fn shell_intent_approval_digest(intent: &ShellSessionIntent) -> Result<String, PolicyError> {
    validate_shell_session_intent(intent)?;
    let bytes = serde_json::to_vec(intent).map_err(|_| invalid("cannot serialize shell intent"))?;
    let mut hash = Sha256::new();
    hash.update(APPROVAL_DOMAIN);
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(&bytes);
    let mut result = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(&mut result, "{byte:02x}").expect("write to String");
    }
    Ok(result)
}

/// This only verifies an exact proposal binding. It does not grant execution.
pub fn verify_shell_intent_binding(
    intent: &ShellSessionIntent,
    trusted: &ExpectedShellBinding<'_>,
) -> Result<(), PolicyError> {
    let digest = shell_intent_approval_digest(intent)?;
    if intent.session_handle != trusted.session_handle
        || intent.workspace_id != trusted.workspace_id
        || intent.policy_revision != trusted.policy_revision
        || intent.windows_user_sid != trusted.windows_user_sid
        || intent.logon_session_id != trusted.logon_session_id
        || intent.device_id != trusted.device_id
        || intent.deskal_session_id != trusted.deskal_session_id
        || intent.authority_epoch != trusted.authority_epoch
        || intent.owner_process_generation != trusted.owner_process_generation
        || intent.session_generation != trusted.session_generation
        || intent.expected_process_generation != trusted.expected_process_generation
        || intent.approval_nonce != trusted.approval_nonce
        || intent.operation != trusted.operation
        || digest != trusted.approval_digest
    {
        return Err(denied(
            "shell-session proposal binding changed or nonce replayed",
        ));
    }
    Ok(())
}

/// Reserved for T02+. Valid Full User lease alone is insufficient at T01.
/// Intentionally no success path: ShellProcess is not an enabled executor.
pub fn authorize_shell_session_t01(
    intent: &ShellSessionIntent,
    lease: Option<&FullControlLease>,
    context: &AuthorityContext<'_>,
    trusted: &ExpectedShellBinding<'_>,
) -> Result<(), PolicyError> {
    verify_shell_intent_binding(intent, trusted)?;
    let lease = lease.ok_or_else(|| denied("local full-user shell lease is required"))?;
    check_full_control(lease, context)?;
    if lease.mode != AuthorityMode::FullUser {
        return Err(denied(
            "only local Full User may ever request shell sessions",
        ));
    }
    if executor_enabled_now(lease.mode, ExecutorClass::ShellProcess) {
        // No production executor should route here before a future qualified
        // implementation replaces this always-deny reserved gate.
        return Err(denied("SG-000096 T01 has no executable shell adapter"));
    }
    Err(denied(
        "ShellProcess remains disabled until SG-000096 T02-T04 qualification",
    ))
}
