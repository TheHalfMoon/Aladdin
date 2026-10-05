//! SG-000084 remote computer-use lease isolation policy.
//!
//! Remote computer use stays disabled until lease isolation is proven
//! here. RemoteComputerUseLease binds provider, principal, exact
//! device, route, session and route epochs, workspace, allowed
//! capability set, allowed targets and windows, expiry, and policy
//! revision, with local issuance and revocation only. Remote callers
//! cannot create, widen, silently renew, self-approve, or resurrect
//! authority. Background screen streams, offline mutation queues,
//! surprise delayed execution, and cross-provider, cross-principal,
//! cross-device, and cross-session reuse are denied. Compromised-relay
//! route confusion fails closed locally.

use crate::error::HostError;

/// Maximum lease lifetime in milliseconds (15 minutes).
pub const MAX_REMOTE_LEASE_MS: u64 = 900_000;

/// Maximum capabilities per lease.
pub const MAX_LEASE_CAPABILITIES: usize = 8;

/// Maximum targets per lease.
pub const MAX_LEASE_TARGETS: usize = 16;

/// Maximum identity field bytes.
pub const MAX_LEASE_FIELD_BYTES: usize = 256;

/// Closed capability set admissible in a remote lease.
pub const LEASE_CAPABILITIES: &[&str] =
    &["observe", "navigate", "click", "fill", "capture", "scroll"];

/// A locally issued remote computer-use lease. Bindings only: no
/// execution right travels with this value, and remote use itself
/// stays disabled (see `remote_execution_allowed`).
#[derive(Debug, Clone)]
pub struct RemoteLease {
    pub lease_id: String,
    pub provider: String,
    pub principal: String,
    pub device_id: String,
    pub route: String,
    pub session_epoch: u64,
    pub route_epoch: u64,
    pub workspace: String,
    pub capabilities: Vec<String>,
    pub targets: Vec<String>,
    pub issued_ms: u64,
    pub expiry_ms: u64,
    pub policy_revision: u64,
    pub revoked: bool,
}

fn check_field(name: &str, value: &str) -> Result<(), HostError> {
    if value.is_empty() || value.len() > MAX_LEASE_FIELD_BYTES {
        return Err(HostError::Invalid(format!("remote lease {name} malformed")));
    }
    if value.chars().any(|ch| ch.is_control()) {
        return Err(HostError::Invalid(format!(
            "remote lease {name} control denied"
        )));
    }
    Ok(())
}

/// Issue a lease locally. Every binding is validated; capabilities
/// must come from the closed set within the count bound; targets
/// must be bounded opaque identities; expiry must sit inside the
/// maximum lifetime. There is no remote issuance path.
pub fn issue_local(lease: RemoteLease) -> Result<RemoteLease, HostError> {
    check_field("lease_id", &lease.lease_id)?;
    check_field("provider", &lease.provider)?;
    check_field("principal", &lease.principal)?;
    check_field("device_id", &lease.device_id)?;
    check_field("route", &lease.route)?;
    check_field("workspace", &lease.workspace)?;
    if lease.capabilities.is_empty() || lease.capabilities.len() > MAX_LEASE_CAPABILITIES {
        return Err(HostError::Invalid(
            "remote lease capability set malformed".into(),
        ));
    }
    for capability in &lease.capabilities {
        if !LEASE_CAPABILITIES.contains(&capability.as_str()) {
            return Err(HostError::Invalid("remote lease capability denied".into()));
        }
    }
    if lease.targets.len() > MAX_LEASE_TARGETS {
        return Err(HostError::Invalid(
            "remote lease target set oversized".into(),
        ));
    }
    for target in &lease.targets {
        crate::observation::validate_identity_field(target)?;
    }
    if lease.expiry_ms <= lease.issued_ms || lease.expiry_ms - lease.issued_ms > MAX_REMOTE_LEASE_MS
    {
        return Err(HostError::Invalid(
            "remote lease lifetime out of bounds".into(),
        ));
    }
    if lease.revoked {
        return Err(HostError::Invalid("remote lease issued revoked".into()));
    }
    Ok(lease)
}

/// Validate a lease for use at one instant: revocation, expiry,
/// scope match, epoch match, and policy match. Any mismatch fails
/// closed; reconnect never resurrects authority.
/// Presented use context checked against a lease.
pub struct LeaseScope<'a> {
    pub provider: &'a str,
    pub principal: &'a str,
    pub device_id: &'a str,
    pub session_epoch: u64,
    pub route_epoch: u64,
    pub workspace: &'a str,
    pub policy_revision: u64,
    pub now_ms: u64,
}

pub fn check_lease(lease: &RemoteLease, scope: &LeaseScope<'_>) -> Result<(), HostError> {
    let provider = scope.provider;
    let principal = scope.principal;
    let device_id = scope.device_id;
    let session_epoch = scope.session_epoch;
    let route_epoch = scope.route_epoch;
    let workspace = scope.workspace;
    let policy_revision = scope.policy_revision;
    let now_ms = scope.now_ms;
    if lease.revoked {
        return Err(HostError::Invalid("remote lease revoked".into()));
    }
    if now_ms >= lease.expiry_ms {
        return Err(HostError::Invalid("remote lease expired".into()));
    }
    if lease.provider != provider {
        return Err(HostError::Invalid(
            "remote lease cross-provider reuse denied".into(),
        ));
    }
    if lease.principal != principal {
        return Err(HostError::Invalid(
            "remote lease cross-principal reuse denied".into(),
        ));
    }
    if lease.device_id != device_id {
        return Err(HostError::Invalid(
            "remote lease cross-device reuse denied".into(),
        ));
    }
    if lease.session_epoch != session_epoch {
        return Err(HostError::Invalid(
            "remote lease cross-session reuse denied".into(),
        ));
    }
    if lease.route_epoch != route_epoch {
        return Err(HostError::Invalid(
            "remote lease route epoch drift denied".into(),
        ));
    }
    if lease.workspace != workspace {
        return Err(HostError::Invalid(
            "remote lease workspace drift denied".into(),
        ));
    }
    if lease.policy_revision != policy_revision {
        return Err(HostError::Invalid(
            "remote lease policy drift denied".into(),
        ));
    }
    Ok(())
}

/// Validate the route against a relay attestation. A compromised
/// relay claiming a different route never overrides the locally
/// recorded route: confusion fails closed locally.
pub fn validate_route(
    lease: &RemoteLease,
    presented_route: &str,
    relay_trusted: bool,
) -> Result<(), HostError> {
    if !relay_trusted {
        return Err(HostError::Invalid("untrusted relay route denied".into()));
    }
    if presented_route != lease.route {
        return Err(HostError::Invalid("relay route confusion denied".into()));
    }
    Ok(())
}

/// Whether remote execution is allowed in this grain. Always false:
/// isolation is proven here; enablement is a separate decision.
pub fn remote_execution_allowed() -> bool {
    false
}

/// Denied: remote callers cannot issue leases.
pub fn issue_remote() -> Result<RemoteLease, HostError> {
    Err(HostError::Invalid("remote lease issuance denied".into()))
}

/// Denied: leases are never widened after issuance.
pub fn widen_lease(_lease: &RemoteLease) -> Result<(), HostError> {
    Err(HostError::Invalid("remote lease widening denied".into()))
}

/// Denied: leases are never renewed silently.
pub fn renew_silently(_lease: &RemoteLease) -> Result<(), HostError> {
    Err(HostError::Invalid(
        "remote lease silent renewal denied".into(),
    ))
}

/// Denied: remote callers cannot self-approve.
pub fn self_approve() -> Result<(), HostError> {
    Err(HostError::Invalid("remote self-approval denied".into()))
}

/// Denied: expired or revoked leases are never resurrected.
pub fn resurrect(_lease: &RemoteLease) -> Result<(), HostError> {
    Err(HostError::Invalid(
        "remote lease resurrection denied".into(),
    ))
}

/// Denied: no background screen stream exists in this grain.
pub fn background_stream() -> Result<(), HostError> {
    Err(HostError::Invalid("background screen stream denied".into()))
}

/// Denied: no offline mutation queue exists in this grain.
pub fn offline_queue() -> Result<(), HostError> {
    Err(HostError::Invalid("offline mutation queue denied".into()))
}

/// Denied: no delayed execution exists in this grain.
pub fn delayed_execution() -> Result<(), HostError> {
    Err(HostError::Invalid("delayed execution denied".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lease() -> RemoteLease {
        RemoteLease {
            lease_id: "remote-lease-1".into(),
            provider: "provider-a".into(),
            principal: "principal-1".into(),
            device_id: "device-1".into(),
            route: "route-1".into(),
            session_epoch: 3,
            route_epoch: 1,
            workspace: "workspace-1".into(),
            capabilities: vec!["observe".into(), "click".into()],
            targets: vec!["window-1".into()],
            issued_ms: 1_000,
            expiry_ms: 2_000,
            policy_revision: 9,
            revoked: false,
        }
    }

    fn scope() -> LeaseScope<'static> {
        LeaseScope {
            provider: "provider-a",
            principal: "principal-1",
            device_id: "device-1",
            session_epoch: 3,
            route_epoch: 1,
            workspace: "workspace-1",
            policy_revision: 9,
            now_ms: 1_500,
        }
    }

    #[test]
    fn bound_lease_issues_and_checks() {
        let lease = issue_local(lease()).unwrap();
        assert!(check_lease(&lease, &scope()).is_ok());
        assert!(!remote_execution_allowed());
    }

    #[test]
    fn cross_scope_reuse_and_drift_fail_closed() {
        let lease = issue_local(lease()).unwrap();
        let base = scope();
        let mutated = |mutate: &dyn Fn(&mut LeaseScope<'_>)| {
            let mut scope = LeaseScope {
                provider: base.provider,
                principal: base.principal,
                device_id: base.device_id,
                session_epoch: base.session_epoch,
                route_epoch: base.route_epoch,
                workspace: base.workspace,
                policy_revision: base.policy_revision,
                now_ms: base.now_ms,
            };
            mutate(&mut scope);
            scope
        };
        for scope in [
            mutated(&|s| s.provider = "provider-b"),
            mutated(&|s| s.principal = "principal-2"),
            mutated(&|s| s.device_id = "device-2"),
            mutated(&|s| s.session_epoch = 4),
            mutated(&|s| s.route_epoch = 2),
            mutated(&|s| s.workspace = "workspace-2"),
            mutated(&|s| s.policy_revision = 10),
            mutated(&|s| s.now_ms = 2_000),
        ] {
            assert!(check_lease(&lease, &scope).is_err());
        }
        let mut revoked = lease.clone();
        revoked.revoked = true;
        assert!(check_lease(&revoked, &base).is_err());
    }

    #[test]
    fn remote_privilege_paths_are_denied() {
        let lease = issue_local(lease()).unwrap();
        assert!(issue_remote().is_err());
        assert!(widen_lease(&lease).is_err());
        assert!(renew_silently(&lease).is_err());
        assert!(self_approve().is_err());
        assert!(resurrect(&lease).is_err());
        assert!(background_stream().is_err());
        assert!(offline_queue().is_err());
        assert!(delayed_execution().is_err());
        let mut bad = lease.clone();
        bad.capabilities = vec!["shell".into()];
        assert!(issue_local(bad).is_err());
    }

    #[test]
    fn compromised_relay_confusion_fails_closed() {
        let lease = issue_local(lease()).unwrap();
        assert!(validate_route(&lease, "route-1", true).is_ok());
        assert!(validate_route(&lease, "route-evil", true).is_err());
        assert!(validate_route(&lease, "route-1", false).is_err());
        assert!(validate_route(&lease, "route-evil", false).is_err());
    }
}
