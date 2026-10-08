//! SG-000096 T01 contract tests. No process is ever spawned.
use qdral_policy::full_control::{
    AuthorityContext, AuthorityMode, ElevationState, FullControlLease, FULL_CONTROL_LEASE_SCHEMA,
};
use qdral_policy::shell_session::{
    authorize_shell_session_t01, shell_intent_approval_digest, validate_shell_session_intent,
    verify_shell_intent_binding, ExpectedShellBinding, ShellIntentPhase, ShellKind, ShellOperation,
    ShellSessionIntent, ShellSessionLimits, SHELL_SESSION_INTENT_SCHEMA,
};
use std::collections::BTreeMap;

fn intent() -> ShellSessionIntent {
    ShellSessionIntent {
        schema: SHELL_SESSION_INTENT_SCHEMA.into(),
        phase: ShellIntentPhase::Proposed,
        operation: ShellOperation::Spawn,
        session_handle: "sh_0123456789abcdef0123456789abcdef".into(),
        shell_kind: ShellKind::Direct,
        executable_path: "C:\\Windows\\System32\\cmd.exe".into(),
        executable_sha256: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
            .into(),
        argv: vec!["/c".into(), "echo".into(), "hello".into()],
        command: String::new(),
        cwd: "C:\\work\\repo".into(),
        environment_value_digests: BTreeMap::from([(
            "TEMP".into(),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        )]),
        limits: ShellSessionLimits {
            timeout_ms: 30_000,
            stdout_bytes: 2 * 1024 * 1024,
            stderr_bytes: 256 * 1024,
            stdin_bytes: 64 * 1024,
        },
        workspace_id: "workspace".into(),
        policy_revision: "policy1".into(),
        windows_user_sid: "S-1-5-21-1000".into(),
        logon_session_id: 42,
        device_id: "device".into(),
        deskal_session_id: "session".into(),
        authority_epoch: 1,
        owner_process_generation: 123,
        session_generation: 4,
        expected_process_generation: 14,
        approval_nonce: "0123456789abcdef0123456789abcdef".into(),
    }
}
fn binding<'a>(i: &'a ShellSessionIntent, digest: &'a str) -> ExpectedShellBinding<'a> {
    ExpectedShellBinding {
        session_handle: &i.session_handle,
        workspace_id: &i.workspace_id,
        policy_revision: &i.policy_revision,
        windows_user_sid: &i.windows_user_sid,
        logon_session_id: i.logon_session_id,
        device_id: &i.device_id,
        deskal_session_id: &i.deskal_session_id,
        authority_epoch: i.authority_epoch,
        owner_process_generation: i.owner_process_generation,
        session_generation: i.session_generation,
        expected_process_generation: i.expected_process_generation,
        approval_nonce: &i.approval_nonce,
        operation: i.operation,
        approval_digest: digest,
    }
}
fn lease(mode: AuthorityMode) -> FullControlLease {
    FullControlLease {
        schema: FULL_CONTROL_LEASE_SCHEMA.into(),
        lease_id: "opaque-lease".into(),
        mode,
        windows_user_sid: "S-1-5-21-1000".into(),
        logon_session_id: 42,
        device_id: "device".into(),
        deskal_session_id: "session".into(),
        policy_revision: "policy1".into(),
        authority_epoch: 1,
        issued_at_ms: 1_000,
        expires_at_ms: 100_000,
        revoked: false,
    }
}
fn ctx<'a>() -> AuthorityContext<'a> {
    AuthorityContext {
        windows_user_sid: "S-1-5-21-1000",
        logon_session_id: 42,
        device_id: "device",
        deskal_session_id: "session",
        policy_revision: "policy1",
        authority_epoch: 1,
        now_ms: 5_000,
        elevation_state: ElevationState::Standard,
        elevation_proof_id: None,
    }
}

#[test]
fn every_authority_mode_denied_even_with_valid_shell_proposal() {
    let i = intent();
    let digest = shell_intent_approval_digest(&i).unwrap();
    verify_shell_intent_binding(&i, &binding(&i, &digest)).unwrap();
    for mode in AuthorityMode::ALL {
        assert!(
            authorize_shell_session_t01(&i, Some(&lease(mode)), &ctx(), &binding(&i, &digest))
                .is_err(),
            "{mode:?} must be denied"
        );
    }
    assert!(authorize_shell_session_t01(&i, None, &ctx(), &binding(&i, &digest)).is_err());
}

#[test]
fn unknown_json_fields_unauthorized_phases_and_bad_utf8_denied() {
    let base = serde_json::to_value(intent()).unwrap();
    for key in [
        "pid",
        "grant_token",
        "authority",
        "remote",
        "elevated",
        "secret",
    ] {
        let mut obj = base.clone();
        obj.as_object_mut()
            .unwrap()
            .insert(key.into(), serde_json::json!(123));
        assert!(
            serde_json::from_value::<ShellSessionIntent>(obj).is_err(),
            "{key}"
        );
    }
    for phase in ["authorized", "executed"] {
        let mut obj = base.clone();
        obj["phase"] = serde_json::json!(phase);
        let i = serde_json::from_value::<ShellSessionIntent>(obj).unwrap();
        assert!(validate_shell_session_intent(&i).is_err(), "{phase}");
    }
    assert!(serde_json::from_slice::<ShellSessionIntent>(b"{\"schema\":\"\\uD800\"}").is_err());
}

#[test]
fn malformed_shell_identity_path_nul_sizes_and_missing_binding_denied() {
    let mut cases = Vec::new();
    let mut i = intent();
    i.cwd = "relative".into();
    cases.push(i);
    let mut i = intent();
    i.cwd = "C:\\work\\..\\outside".into();
    cases.push(i);
    let mut i = intent();
    i.cwd = "\\\\foreign\\share".into();
    cases.push(i);
    let mut i = intent();
    i.executable_path = "C:relative.exe".into();
    cases.push(i);
    let mut i = intent();
    i.argv[0] = "bad\0arg".into();
    cases.push(i);
    let mut i = intent();
    i.argv[0] = "x".repeat(8193);
    cases.push(i);
    let mut i = intent();
    i.argv = vec!["x".into(); 65];
    cases.push(i);
    let mut i = intent();
    i.command = "unexpected-shell".into();
    cases.push(i);
    let mut i = intent();
    i.executable_sha256 = "not-sha".into();
    cases.push(i);
    let mut i = intent();
    i.session_handle = "foreign-pid".into();
    cases.push(i);
    let mut i = intent();
    i.limits.timeout_ms = 0;
    cases.push(i);
    let mut i = intent();
    i.limits.stdout_bytes = 16 * 1024 * 1024 + 1;
    cases.push(i);
    let mut i = intent();
    i.limits.stdin_bytes = 256 * 1024 + 1;
    cases.push(i);
    let mut i = intent();
    i.logon_session_id = 0;
    cases.push(i);
    let mut i = intent();
    i.authority_epoch = 0;
    cases.push(i);
    let mut i = intent();
    i.approval_nonce = String::new();
    cases.push(i);
    let mut i = intent();
    i.environment_value_digests
        .insert("SECRET".into(), "not-sha".into());
    cases.push(i);
    let mut i = intent();
    i.approval_nonce = "short-guessable-nonce".into();
    cases.push(i);
    let mut i = intent();
    i.cwd = "C:\\work\\\\duplicate".into();
    cases.push(i);
    let mut i = intent();
    i.workspace_id = "workspace\nspoof".into();
    cases.push(i);
    let mut i = intent();
    i.argv[1] = "bad\u{202e}format".into();
    cases.push(i);
    let mut i = intent();
    i.environment_value_digests.insert(
        "temp".into(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    );
    cases.push(i);
    for (n, i) in cases.iter().enumerate() {
        assert!(
            validate_shell_session_intent(i).is_err(),
            "invalid case {n} accepted"
        );
    }
}

#[test]
fn digest_binds_all_material_fields_and_rejects_cross_operation_or_nonce_replay() {
    let base = intent();
    let original = shell_intent_approval_digest(&base).unwrap();
    let mut cases = Vec::new();
    let mut i = base.clone();
    i.operation = ShellOperation::Terminate;
    cases.push(i);
    let mut i = base.clone();
    i.executable_path = "D:\\other\\program.exe".into();
    cases.push(i);
    let mut i = base.clone();
    i.executable_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
    cases.push(i);
    let mut i = base.clone();
    i.argv[1] = "dir".into();
    cases.push(i);
    let mut i = base.clone();
    i.cwd = "C:\\other".into();
    cases.push(i);
    let mut i = base.clone();
    i.environment_value_digests.insert(
        "TEMP".into(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    );
    cases.push(i);
    let mut i = base.clone();
    i.limits.stdin_bytes += 1;
    cases.push(i);
    let mut i = base.clone();
    i.workspace_id = "different".into();
    cases.push(i);
    let mut i = base.clone();
    i.policy_revision = "different".into();
    cases.push(i);
    let mut i = base.clone();
    i.windows_user_sid = "S-1-5-21-222".into();
    cases.push(i);
    let mut i = base.clone();
    i.logon_session_id += 1;
    cases.push(i);
    let mut i = base.clone();
    i.device_id = "other".into();
    cases.push(i);
    let mut i = base.clone();
    i.deskal_session_id = "other".into();
    cases.push(i);
    let mut i = base.clone();
    i.authority_epoch += 1;
    cases.push(i);
    let mut i = base.clone();
    i.owner_process_generation += 1;
    cases.push(i);
    let mut i = base.clone();
    i.session_generation += 1;
    cases.push(i);
    let mut i = base.clone();
    i.expected_process_generation += 1;
    cases.push(i);
    let mut i = base.clone();
    i.approval_nonce = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
    cases.push(i);
    let mut i = base.clone();
    i.session_handle = "sh_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
    cases.push(i);
    for (n, i) in cases.iter().enumerate() {
        assert_ne!(
            original,
            shell_intent_approval_digest(i).unwrap(),
            "digest drift {n}"
        );
        assert!(
            verify_shell_intent_binding(i, &binding(&base, &original)).is_err(),
            "cross-binding {n}"
        );
    }
    let mut i = base.clone();
    i.shell_kind = ShellKind::PowerShell;
    i.argv.clear();
    i.command = "Write-Output hi".into();
    assert_ne!(original, shell_intent_approval_digest(&i).unwrap());
    let mut b = binding(&base, &original);
    b.approval_nonce = "replayed";
    assert!(verify_shell_intent_binding(&base, &b).is_err());
}

#[test]
fn revoked_expired_foreign_lease_and_session_are_denied() {
    let i = intent();
    let digest = shell_intent_approval_digest(&i).unwrap();
    let b = binding(&i, &digest);
    let mut cases = Vec::new();
    let mut l = lease(AuthorityMode::FullUser);
    l.revoked = true;
    cases.push(l);
    let mut l = lease(AuthorityMode::FullUser);
    l.expires_at_ms = 5000;
    cases.push(l);
    let mut l = lease(AuthorityMode::FullUser);
    l.authority_epoch += 1;
    cases.push(l);
    let mut l = lease(AuthorityMode::FullUser);
    l.windows_user_sid = "S-1-5-21-99".into();
    cases.push(l);
    let mut l = lease(AuthorityMode::FullUser);
    l.logon_session_id += 1;
    cases.push(l);
    let mut l = lease(AuthorityMode::FullUser);
    l.device_id = "other".into();
    cases.push(l);
    let mut l = lease(AuthorityMode::FullUser);
    l.deskal_session_id = "other".into();
    cases.push(l);
    let mut l = lease(AuthorityMode::FullUser);
    l.policy_revision = "other".into();
    cases.push(l);
    for (n, l) in cases.iter().enumerate() {
        assert!(
            authorize_shell_session_t01(&i, Some(l), &ctx(), &b).is_err(),
            "lease {n}"
        );
    }
}
