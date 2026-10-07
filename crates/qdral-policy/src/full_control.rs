//! SG-000093 full-control authority and profile model.
//!
//! This module defines Deskal-owned authority leases only. It does not expose
//! desktop input, shell/process widening, expanded filesystem access, browser
//! or web executors, an elevation broker, persistent-admin service, or remote
//! full-control execution. Donor runtimes remain absent from this grain.

use qdral_contracts::FailureCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

use crate::PolicyError;

pub const FULL_CONTROL_LEASE_SCHEMA: &str = "deskal-full-control-lease/1";
pub const ADMIN_LEASE_SCHEMA: &str = "deskal-admin-lease/1";
pub const MIN_FULL_CONTROL_LEASE_MS: u64 = 60_000;
pub const MAX_FULL_CONTROL_LEASE_MS: u64 = 12 * 60 * 60 * 1_000;
pub const MAX_AUTHORITY_FIELD_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityMode {
    Safe,
    FullUser,
    FullAdmin,
    PersistentAdmin,
    RemoteFullControl,
}

impl AuthorityMode {
    pub const ALL: [Self; 5] = [
        Self::Safe,
        Self::FullUser,
        Self::FullAdmin,
        Self::PersistentAdmin,
        Self::RemoteFullControl,
    ];

    pub fn is_default(self) -> bool {
        self == Self::Safe
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantOrigin {
    LocalUserControl,
    McpCaller,
    Agent,
    Donor,
    Relay,
    RemotePrincipal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElevationState {
    Standard,
    ElevatedAdministrator,
    System,
    TrustedInstaller,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorClass {
    DesktopObservation,
    DesktopInput,
    ShellProcess,
    Filesystem,
    Browser,
    Web,
    VisualAgent,
    Admin,
    Remote,
}

const FULL_USER_CEILING: &[ExecutorClass] = &[
    ExecutorClass::DesktopObservation,
    ExecutorClass::DesktopInput,
    ExecutorClass::ShellProcess,
    ExecutorClass::Filesystem,
    ExecutorClass::Browser,
    ExecutorClass::Web,
    ExecutorClass::VisualAgent,
];

const FULL_ADMIN_CEILING: &[ExecutorClass] = &[
    ExecutorClass::DesktopObservation,
    ExecutorClass::DesktopInput,
    ExecutorClass::ShellProcess,
    ExecutorClass::Filesystem,
    ExecutorClass::Browser,
    ExecutorClass::Web,
    ExecutorClass::VisualAgent,
    ExecutorClass::Admin,
];

pub fn profile_ceiling(mode: AuthorityMode) -> &'static [ExecutorClass] {
    match mode {
        AuthorityMode::FullUser => FULL_USER_CEILING,
        AuthorityMode::FullAdmin => FULL_ADMIN_CEILING,
        AuthorityMode::Safe
        | AuthorityMode::PersistentAdmin
        | AuthorityMode::RemoteFullControl => &[],
    }
}

pub fn mode_selectable(origin: GrantOrigin, mode: AuthorityMode) -> Result<(), PolicyError> {
    if origin != GrantOrigin::LocalUserControl {
        return Err(denied(
            "full-control authority can only be selected by the authenticated local user control path",
        ));
    }
    match mode {
        AuthorityMode::Safe | AuthorityMode::FullUser | AuthorityMode::FullAdmin => Ok(()),
        AuthorityMode::PersistentAdmin => Err(denied(
            "persistent admin is unavailable until its dedicated P20 successor closes canonically",
        )),
        AuthorityMode::RemoteFullControl => Err(denied(
            "remote full control is unavailable until its dedicated P20 successor closes canonically",
        )),
    }
}

pub fn broader_fallback_allowed() -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullControlLease {
    pub schema: String,
    pub lease_id: String,
    pub mode: AuthorityMode,
    pub windows_user_sid: String,
    pub logon_session_id: u64,
    pub device_id: String,
    pub deskal_session_id: String,
    pub policy_revision: String,
    pub authority_epoch: u64,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullControlLeaseRequest {
    pub lease_id: String,
    pub origin: GrantOrigin,
    pub mode: AuthorityMode,
    pub windows_user_sid: String,
    pub logon_session_id: u64,
    pub device_id: String,
    pub deskal_session_id: String,
    pub policy_revision: String,
    pub authority_epoch: u64,
    pub issued_at_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthorityContext<'a> {
    pub windows_user_sid: &'a str,
    pub logon_session_id: u64,
    pub device_id: &'a str,
    pub deskal_session_id: &'a str,
    pub policy_revision: &'a str,
    pub authority_epoch: u64,
    pub now_ms: u64,
    pub elevation_state: ElevationState,
    pub elevation_proof_id: Option<&'a str>,
}

pub fn issue_full_control(
    request: FullControlLeaseRequest,
) -> Result<FullControlLease, PolicyError> {
    mode_selectable(request.origin, request.mode)?;
    if !matches!(request.mode, AuthorityMode::FullUser | AuthorityMode::FullAdmin) {
        return Err(invalid(
            "safe mode does not require a lease and successor-only modes cannot be issued here",
        ));
    }
    validate_id(&request.lease_id, "fc-", 64, "full-control lease id")?;
    validate_sid(&request.windows_user_sid)?;
    validate_field("device id", &request.device_id)?;
    validate_field("Deskal session id", &request.deskal_session_id)?;
    validate_field("policy revision", &request.policy_revision)?;
    if request.logon_session_id == 0 {
        return Err(invalid("logon session id must be non-zero"));
    }
    if request.authority_epoch == 0 {
        return Err(invalid("authority epoch must be non-zero"));
    }
    if !(MIN_FULL_CONTROL_LEASE_MS..=MAX_FULL_CONTROL_LEASE_MS).contains(&request.duration_ms) {
        return Err(invalid("full-control lease duration is out of bounds"));
    }
    let expires_at_ms = request
        .issued_at_ms
        .checked_add(request.duration_ms)
        .ok_or_else(|| invalid("full-control lease expiry overflow"))?;
    Ok(FullControlLease {
        schema: FULL_CONTROL_LEASE_SCHEMA.to_owned(),
        lease_id: request.lease_id,
        mode: request.mode,
        windows_user_sid: request.windows_user_sid,
        logon_session_id: request.logon_session_id,
        device_id: request.device_id,
        deskal_session_id: request.deskal_session_id,
        policy_revision: request.policy_revision,
        authority_epoch: request.authority_epoch,
        issued_at_ms: request.issued_at_ms,
        expires_at_ms,
        revoked: false,
    })
}

pub fn check_full_control(
    lease: &FullControlLease,
    context: &AuthorityContext<'_>,
) -> Result<(), PolicyError> {
    if lease.schema != FULL_CONTROL_LEASE_SCHEMA {
        return Err(denied("unknown full-control lease schema"));
    }
    if lease.revoked {
        return Err(denied("full-control lease revoked"));
    }
    if context.now_ms < lease.issued_at_ms || context.now_ms >= lease.expires_at_ms {
        return Err(denied("full-control lease inactive at the current time"));
    }
    if lease.windows_user_sid != context.windows_user_sid {
        return Err(denied("full-control lease Windows user binding mismatch"));
    }
    if lease.logon_session_id != context.logon_session_id {
        return Err(denied("full-control lease logon-session binding mismatch"));
    }
    if lease.device_id != context.device_id {
        return Err(denied("full-control lease device binding mismatch"));
    }
    if lease.deskal_session_id != context.deskal_session_id {
        return Err(denied("full-control lease Deskal-session binding mismatch"));
    }
    if lease.policy_revision != context.policy_revision {
        return Err(denied("full-control lease policy revision mismatch"));
    }
    if lease.authority_epoch != context.authority_epoch {
        return Err(denied("full-control lease authority epoch mismatch"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminLease {
    pub schema: String,
    pub lease_id: String,
    pub full_control_lease_id: String,
    pub windows_user_sid: String,
    pub logon_session_id: u64,
    pub device_id: String,
    pub deskal_session_id: String,
    pub policy_revision: String,
    pub authority_epoch: u64,
    pub elevation_proof_id: String,
    pub elevation_state: ElevationState,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminLeaseRequest {
    pub lease_id: String,
    pub origin: GrantOrigin,
    pub elevation_proof_id: String,
    pub elevation_state: ElevationState,
    pub issued_at_ms: u64,
    pub duration_ms: u64,
}

pub fn issue_admin_lease(
    request: AdminLeaseRequest,
    full_control: &FullControlLease,
    context: &AuthorityContext<'_>,
) -> Result<AdminLease, PolicyError> {
    if request.origin != GrantOrigin::LocalUserControl {
        return Err(denied(
            "admin authority can only be issued by the authenticated local user control path",
        ));
    }
    check_full_control(full_control, context)?;
    if full_control.mode != AuthorityMode::FullAdmin {
        return Err(denied("admin lease requires Full Admin intent"));
    }
    validate_id(&request.lease_id, "admin-", 64, "admin lease id")?;
    validate_id(
        &request.elevation_proof_id,
        "win-elev-",
        64,
        "Windows elevation proof id",
    )?;
    if request.elevation_state != ElevationState::ElevatedAdministrator
        || context.elevation_state != ElevationState::ElevatedAdministrator
    {
        return Err(denied(
            "admin lease requires a legitimate elevated administrator Windows authority binding",
        ));
    }
    if context.elevation_proof_id != Some(request.elevation_proof_id.as_str()) {
        return Err(denied("Windows elevation proof binding mismatch"));
    }
    if request.issued_at_ms < full_control.issued_at_ms
        || request.issued_at_ms >= full_control.expires_at_ms
    {
        return Err(invalid("admin lease issue time is outside the full-control lease"));
    }
    if !(MIN_FULL_CONTROL_LEASE_MS..=MAX_FULL_CONTROL_LEASE_MS).contains(&request.duration_ms) {
        return Err(invalid("admin lease duration is out of bounds"));
    }
    let expires_at_ms = request
        .issued_at_ms
        .checked_add(request.duration_ms)
        .ok_or_else(|| invalid("admin lease expiry overflow"))?;
    if expires_at_ms > full_control.expires_at_ms {
        return Err(denied(
            "admin lease cannot outlive the Full Admin full-control lease",
        ));
    }
    Ok(AdminLease {
        schema: ADMIN_LEASE_SCHEMA.to_owned(),
        lease_id: request.lease_id,
        full_control_lease_id: full_control.lease_id.clone(),
        windows_user_sid: full_control.windows_user_sid.clone(),
        logon_session_id: full_control.logon_session_id,
        device_id: full_control.device_id.clone(),
        deskal_session_id: full_control.deskal_session_id.clone(),
        policy_revision: full_control.policy_revision.clone(),
        authority_epoch: full_control.authority_epoch,
        elevation_proof_id: request.elevation_proof_id,
        elevation_state: request.elevation_state,
        issued_at_ms: request.issued_at_ms,
        expires_at_ms,
        revoked: false,
    })
}

pub fn check_admin_lease(
    admin: &AdminLease,
    full_control: &FullControlLease,
    context: &AuthorityContext<'_>,
) -> Result<(), PolicyError> {
    check_full_control(full_control, context)?;
    if full_control.mode != AuthorityMode::FullAdmin {
        return Err(denied("admin authority requires an active Full Admin lease"));
    }
    if admin.schema != ADMIN_LEASE_SCHEMA || admin.revoked {
        return Err(denied("admin lease inactive"));
    }
    if admin.full_control_lease_id != full_control.lease_id {
        return Err(denied("admin lease is bound to a different full-control lease"));
    }
    if context.now_ms < admin.issued_at_ms || context.now_ms >= admin.expires_at_ms {
        return Err(denied("admin lease inactive at the current time"));
    }
    if admin.windows_user_sid != context.windows_user_sid
        || admin.logon_session_id != context.logon_session_id
        || admin.device_id != context.device_id
        || admin.deskal_session_id != context.deskal_session_id
        || admin.policy_revision != context.policy_revision
        || admin.authority_epoch != context.authority_epoch
    {
        return Err(denied("admin lease authority binding mismatch"));
    }
    if admin.elevation_state != ElevationState::ElevatedAdministrator
        || context.elevation_state != ElevationState::ElevatedAdministrator
        || context.elevation_proof_id != Some(admin.elevation_proof_id.as_str())
    {
        return Err(denied("admin lease Windows elevation binding mismatch"));
    }
    Ok(())
}

pub fn revoke_full_control(lease: &mut FullControlLease) {
    lease.revoked = true;
}

pub fn revoke_admin(lease: &mut AdminLease) {
    lease.revoked = true;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityEpoch {
    value: u64,
}

impl AuthorityEpoch {
    pub fn new(value: u64) -> Result<Self, PolicyError> {
        if value == 0 {
            return Err(invalid("authority epoch must be non-zero"));
        }
        Ok(Self { value })
    }

    pub fn value(self) -> u64 {
        self.value
    }

    /// Emergency revoke is authority-reducing: advancing the epoch invalidates
    /// every lease issued under the previous value before new dispatch.
    pub fn emergency_revoke(&mut self) -> Result<u64, PolicyError> {
        self.value = self
            .value
            .checked_add(1)
            .ok_or_else(|| invalid("authority epoch overflow"))?;
        Ok(self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    NotStarted,
    Dispatched,
    Completed,
    Cancelled,
    OutcomeUnknown,
}

impl ExecutionState {
    pub const ALL: [Self; 5] = [
        Self::NotStarted,
        Self::Dispatched,
        Self::Completed,
        Self::Cancelled,
        Self::OutcomeUnknown,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    Granted,
    Revoked,
    Expired,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityAuditEvent {
    pub kind: AuditKind,
    pub lease_id: String,
    pub mode: AuthorityMode,
    pub binding_digest: String,
    pub authority_epoch: u64,
    pub at_ms: u64,
}

pub fn full_control_audit_event(
    kind: AuditKind,
    lease: &FullControlLease,
    at_ms: u64,
) -> AuthorityAuditEvent {
    AuthorityAuditEvent {
        kind,
        lease_id: lease.lease_id.clone(),
        mode: lease.mode,
        binding_digest: binding_digest(
            &lease.windows_user_sid,
            lease.logon_session_id,
            &lease.device_id,
            &lease.deskal_session_id,
            &lease.policy_revision,
            lease.authority_epoch,
            None,
        ),
        authority_epoch: lease.authority_epoch,
        at_ms,
    }
}

pub fn admin_audit_event(
    kind: AuditKind,
    lease: &AdminLease,
    at_ms: u64,
) -> AuthorityAuditEvent {
    AuthorityAuditEvent {
        kind,
        lease_id: lease.lease_id.clone(),
        mode: AuthorityMode::FullAdmin,
        binding_digest: binding_digest(
            &lease.windows_user_sid,
            lease.logon_session_id,
            &lease.device_id,
            &lease.deskal_session_id,
            &lease.policy_revision,
            lease.authority_epoch,
            Some(&lease.elevation_proof_id),
        ),
        authority_epoch: lease.authority_epoch,
        at_ms,
    }
}

fn binding_digest(
    windows_user_sid: &str,
    logon_session_id: u64,
    device_id: &str,
    deskal_session_id: &str,
    policy_revision: &str,
    authority_epoch: u64,
    elevation_proof_id: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    let logon_session_id = logon_session_id.to_string();
    let authority_epoch = authority_epoch.to_string();
    for part in [
        windows_user_sid,
        logon_session_id.as_str(),
        device_id,
        deskal_session_id,
        policy_revision,
        authority_epoch.as_str(),
        elevation_proof_id.unwrap_or(""),
    ] {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn validate_sid(value: &str) -> Result<(), PolicyError> {
    validate_field("Windows user SID", value)?;
    if !value.starts_with("S-1-") {
        return Err(invalid("Windows user SID has an invalid shape"));
    }
    Ok(())
}

fn validate_field(name: &str, value: &str) -> Result<(), PolicyError> {
    if value.is_empty() || value.len() > MAX_AUTHORITY_FIELD_BYTES {
        return Err(invalid(format!("{name} malformed")));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(format!("{name} contains a control character")));
    }
    Ok(())
}

fn validate_id(
    value: &str,
    prefix: &str,
    hex_chars: usize,
    name: &str,
) -> Result<(), PolicyError> {
    if value.len() != prefix.len() + hex_chars || !value.starts_with(prefix) {
        return Err(invalid(format!("{name} malformed")));
    }
    if !value[prefix.len()..]
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid(format!("{name} malformed")));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> PolicyError {
    PolicyError {
        code: FailureCode::InvalidRequest,
        message: message.into(),
    }
}

fn denied(message: impl Into<String>) -> PolicyError {
    PolicyError {
        code: FailureCode::CapabilityDenied,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000_000;
    const SID: &str = "S-1-5-21-1000-2000-3000-1001";
    const DEVICE: &str = "device-local-1";
    const DESKAL_SESSION: &str = "deskal-session-1";
    const POLICY: &str = "sg-000093-v1";

    fn lease_id(prefix: &str, ch: char) -> String {
        format!("{prefix}{}", ch.to_string().repeat(64))
    }

    fn request(mode: AuthorityMode) -> FullControlLeaseRequest {
        FullControlLeaseRequest {
            lease_id: lease_id("fc-", 'a'),
            origin: GrantOrigin::LocalUserControl,
            mode,
            windows_user_sid: SID.into(),
            logon_session_id: 42,
            device_id: DEVICE.into(),
            deskal_session_id: DESKAL_SESSION.into(),
            policy_revision: POLICY.into(),
            authority_epoch: 7,
            issued_at_ms: NOW,
            duration_ms: 60 * 60 * 1_000,
        }
    }

    fn context<'a>(
        sid: &'a str,
        device: &'a str,
        session: &'a str,
        policy: &'a str,
    ) -> AuthorityContext<'a> {
        AuthorityContext {
            windows_user_sid: sid,
            logon_session_id: 42,
            device_id: device,
            deskal_session_id: session,
            policy_revision: policy,
            authority_epoch: 7,
            now_ms: NOW + 1_000,
            elevation_state: ElevationState::Standard,
            elevation_proof_id: None,
        }
    }

    #[test]
    fn authority_modes_and_profile_ceilings_are_exact() {
        assert_eq!(AuthorityMode::ALL.len(), 5);
        assert_eq!(
            AuthorityMode::ALL.iter().filter(|mode| mode.is_default()).count(),
            1
        );
        assert!(AuthorityMode::Safe.is_default());
        assert!(mode_selectable(
            GrantOrigin::LocalUserControl,
            AuthorityMode::FullUser
        )
        .is_ok());
        assert!(mode_selectable(
            GrantOrigin::LocalUserControl,
            AuthorityMode::FullAdmin
        )
        .is_ok());
        assert!(mode_selectable(
            GrantOrigin::LocalUserControl,
            AuthorityMode::PersistentAdmin
        )
        .is_err());
        assert!(mode_selectable(
            GrantOrigin::LocalUserControl,
            AuthorityMode::RemoteFullControl
        )
        .is_err());
        assert!(profile_ceiling(AuthorityMode::FullUser)
            .contains(&ExecutorClass::DesktopInput));
        assert!(!profile_ceiling(AuthorityMode::FullUser).contains(&ExecutorClass::Admin));
        assert!(profile_ceiling(AuthorityMode::FullAdmin).contains(&ExecutorClass::Admin));
        assert!(profile_ceiling(AuthorityMode::PersistentAdmin).is_empty());
        assert!(profile_ceiling(AuthorityMode::RemoteFullControl).is_empty());
        assert!(!broader_fallback_allowed());
    }

    #[test]
    fn exact_local_full_control_lease_checks_and_drift_fails_closed() {
        let lease = issue_full_control(request(AuthorityMode::FullUser)).unwrap();
        let base = context(SID, DEVICE, DESKAL_SESSION, POLICY);
        assert!(check_full_control(&lease, &base).is_ok());

        let other_sid = "S-1-5-21-1000-2000-3000-1002";
        let mut cases = vec![
            context(other_sid, DEVICE, DESKAL_SESSION, POLICY),
            context(SID, "device-local-2", DESKAL_SESSION, POLICY),
            context(SID, DEVICE, "deskal-session-2", POLICY),
            context(SID, DEVICE, DESKAL_SESSION, "sg-000093-v2"),
        ];
        cases[0].logon_session_id = 43;
        for case in cases {
            assert!(check_full_control(&lease, &case).is_err());
        }
        let mut epoch = base;
        epoch.authority_epoch = 8;
        assert!(check_full_control(&lease, &epoch).is_err());
        let mut expired = base;
        expired.now_ms = lease.expires_at_ms;
        assert!(check_full_control(&lease, &expired).is_err());
        let mut revoked = lease.clone();
        revoke_full_control(&mut revoked);
        assert!(check_full_control(&revoked, &base).is_err());
    }

    #[test]
    fn agent_donor_relay_mcp_and_remote_origins_cannot_mint_authority() {
        for origin in [
            GrantOrigin::McpCaller,
            GrantOrigin::Agent,
            GrantOrigin::Donor,
            GrantOrigin::Relay,
            GrantOrigin::RemotePrincipal,
        ] {
            let mut req = request(AuthorityMode::FullUser);
            req.origin = origin;
            assert!(issue_full_control(req).is_err());
            assert!(mode_selectable(origin, AuthorityMode::FullAdmin).is_err());
        }
    }

    #[test]
    fn admin_lease_requires_exact_legitimate_windows_elevation_binding() {
        let full = issue_full_control(request(AuthorityMode::FullAdmin)).unwrap();
        let proof = lease_id("win-elev-", 'b');
        let mut elevated = context(SID, DEVICE, DESKAL_SESSION, POLICY);
        elevated.elevation_state = ElevationState::ElevatedAdministrator;
        elevated.elevation_proof_id = Some(&proof);
        let admin = issue_admin_lease(
            AdminLeaseRequest {
                lease_id: lease_id("admin-", 'c'),
                origin: GrantOrigin::LocalUserControl,
                elevation_proof_id: proof.clone(),
                elevation_state: ElevationState::ElevatedAdministrator,
                issued_at_ms: NOW + 1_000,
                duration_ms: 30 * 60 * 1_000,
            },
            &full,
            &elevated,
        )
        .unwrap();
        assert!(check_admin_lease(&admin, &full, &elevated).is_ok());

        for state in [
            ElevationState::Standard,
            ElevationState::System,
            ElevationState::TrustedInstaller,
        ] {
            let mut ctx = elevated;
            ctx.elevation_state = state;
            assert!(check_admin_lease(&admin, &full, &ctx).is_err());
        }

        let mut wrong_proof = elevated;
        wrong_proof.elevation_proof_id = Some("win-elev-dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        assert!(check_admin_lease(&admin, &full, &wrong_proof).is_err());

        let full_user = issue_full_control(request(AuthorityMode::FullUser)).unwrap();
        assert!(issue_admin_lease(
            AdminLeaseRequest {
                lease_id: lease_id("admin-", 'e'),
                origin: GrantOrigin::LocalUserControl,
                elevation_proof_id: proof,
                elevation_state: ElevationState::ElevatedAdministrator,
                issued_at_ms: NOW + 1_000,
                duration_ms: 30 * 60 * 1_000,
            },
            &full_user,
            &elevated,
        )
        .is_err());
    }

    #[test]
    fn revoke_and_epoch_change_invalidate_before_new_dispatch() {
        let mut lease = issue_full_control(request(AuthorityMode::FullUser)).unwrap();
        let mut epoch = AuthorityEpoch::new(lease.authority_epoch).unwrap();
        let mut ctx = context(SID, DEVICE, DESKAL_SESSION, POLICY);
        assert!(check_full_control(&lease, &ctx).is_ok());

        ctx.authority_epoch = epoch.emergency_revoke().unwrap();
        assert!(check_full_control(&lease, &ctx).is_err());

        revoke_full_control(&mut lease);
        assert!(check_full_control(&lease, &ctx).is_err());
    }

    #[test]
    fn audit_projection_hashes_identity_and_elevation_binding_material() {
        let full = issue_full_control(request(AuthorityMode::FullAdmin)).unwrap();
        let event = full_control_audit_event(AuditKind::Granted, &full, NOW + 1);
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains(SID));
        assert!(!json.contains(DEVICE));
        assert!(!json.contains(DESKAL_SESSION));
        assert_eq!(event.binding_digest.len(), 64);

        let proof = lease_id("win-elev-", 'f');
        let mut elevated = context(SID, DEVICE, DESKAL_SESSION, POLICY);
        elevated.elevation_state = ElevationState::ElevatedAdministrator;
        elevated.elevation_proof_id = Some(&proof);
        let admin = issue_admin_lease(
            AdminLeaseRequest {
                lease_id: lease_id("admin-", 'd'),
                origin: GrantOrigin::LocalUserControl,
                elevation_proof_id: proof.clone(),
                elevation_state: ElevationState::ElevatedAdministrator,
                issued_at_ms: NOW + 1,
                duration_ms: 60_000,
            },
            &full,
            &elevated,
        )
        .unwrap();
        let admin_event = admin_audit_event(AuditKind::Granted, &admin, NOW + 2);
        let json = serde_json::to_string(&admin_event).unwrap();
        assert!(!json.contains(&proof));
        assert!(!json.contains(SID));
        assert_eq!(admin_event.binding_digest.len(), 64);
    }

    #[test]
    fn execution_state_vocabulary_stays_exact_and_no_retry_semantics_are_added() {
        assert_eq!(
            ExecutionState::ALL,
            [
                ExecutionState::NotStarted,
                ExecutionState::Dispatched,
                ExecutionState::Completed,
                ExecutionState::Cancelled,
                ExecutionState::OutcomeUnknown,
            ]
        );
        assert!(!broader_fallback_allowed());
    }
}
