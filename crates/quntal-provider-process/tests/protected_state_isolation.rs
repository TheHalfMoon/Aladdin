#![cfg(windows)]

use quntal_provider_process::{build_execution_plan, execute_contained, ExecutionLimits};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const SENTINEL_DIR: &str = "sg-000012-qualification";
const SENTINEL_FILE: &str = "protected-state-sentinel";

fn required_env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("required Windows environment variable is unavailable: {name}"))
}

fn qualification_env(workspace: &std::path::Path) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([
        ("LOCALAPPDATA".to_owned(), required_env("LOCALAPPDATA")),
        ("SystemRoot".to_owned(), required_env("SystemRoot")),
        ("TEMP".to_owned(), required_env("TEMP")),
        ("TMP".to_owned(), workspace.to_string_lossy().into_owned()),
    ]);
    for optional in ["PATH", "ComSpec"] {
        if let Ok(value) = std::env::var(optional) {
            env.insert(optional.to_owned(), value);
        }
    }

    // These values must never reach the child. They are deliberately supplied
    // to the plan builder so the qualification traverses the real sanitizer.
    env.insert(
        "QUNTAL_TUNNEL_KEY_FILE".to_owned(),
        "must-not-reach-contained-child".to_owned(),
    );
    env.insert(
        "OPENAI_API_KEY".to_owned(),
        "must-not-reach-contained-child".to_owned(),
    );
    env
}

fn sentinel_path() -> PathBuf {
    let local_app_data = PathBuf::from(required_env("LOCALAPPDATA"));
    let tmp = PathBuf::from(required_env("TMP"));
    let instance = tmp
        .file_name()
        .expect("qualification TMP has a final component");
    local_app_data
        .join("Quntal")
        .join(SENTINEL_DIR)
        .join(instance)
        .join(SENTINEL_FILE)
}

#[test]
fn protected_state_child() {
    if !std::env::args().any(|argument| argument == "--exact") {
        return;
    }

    assert!(
        std::env::var_os("QUNTAL_TUNNEL_KEY_FILE").is_none(),
        "contained child inherited Quntal tunnel-key authority"
    );
    assert!(
        std::env::var_os("OPENAI_API_KEY").is_none(),
        "contained child inherited a secret-like API key"
    );
    assert!(
        std::env::var_os("QUNTAL_DAEMON").is_none(),
        "contained child inherited Quntal daemon authority"
    );

    // The contained executor binds stdin to NUL. A zero-length read proves the
    // child did not inherit the request-scoped quntal-mcp -> quntald stdin pipe.
    use std::io::Read;
    let mut byte = [0u8; 1];
    let read = std::io::stdin()
        .read(&mut byte)
        .expect("read contained stdin");
    assert_eq!(
        read, 0,
        "contained child inherited a readable stdin authority channel"
    );

    if std::fs::read(sentinel_path()).is_ok() {
        panic!("restricted AppContainer child could read Quntal protected state");
    }
}

#[test]
fn windows_restricted_child_cannot_read_quntal_protected_state() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let workspace = std::env::temp_dir().join(format!(
        "Quntal.SG000012.Workspace.{}.{}",
        std::process::id(),
        suffix
    ));
    std::fs::create_dir_all(&workspace).expect("create qualification workspace");

    let local_app_data = PathBuf::from(required_env("LOCALAPPDATA"));
    let instance = workspace
        .file_name()
        .expect("workspace has a final component");
    let protected_dir = local_app_data
        .join("Quntal")
        .join(SENTINEL_DIR)
        .join(instance);
    std::fs::create_dir_all(&protected_dir).expect("create Quntal-owned qualification state");
    let sentinel = protected_dir.join(SENTINEL_FILE);
    std::fs::write(&sentinel, b"QUNTAL-SG-000012-PROTECTED-STATE")
        .expect("seed protected-state sentinel");
    assert!(
        sentinel.is_file(),
        "parent failed to create protected-state sentinel"
    );

    let executable = std::env::current_exe().expect("integration-test executable");
    let env = qualification_env(&workspace);
    let plan = build_execution_plan(
        &workspace,
        &executable,
        &[
            "--exact".to_owned(),
            "protected_state_child".to_owned(),
            "--nocapture".to_owned(),
        ],
        ".",
        &env,
        ExecutionLimits::default(),
    )
    .expect("protected-state qualification plan");

    assert!(!plan.env.contains_key("QUNTAL_TUNNEL_KEY_FILE"));
    assert!(!plan.env.contains_key("OPENAI_API_KEY"));
    assert!(!plan.env.contains_key("QUNTAL_DAEMON"));

    let profile = format!("Quntal.SG000012.{}.{}", std::process::id(), suffix);
    let result = execute_contained(plan, &profile).expect("protected-state isolation execution");

    assert_eq!(result.exit_code, 0, "negative-access fixture failed");
    assert!(result.appcontainer_verified);
    assert!(result.assigned_to_job_before_resume);
    assert!(result.job_quiescent);

    let _ = std::fs::remove_dir_all(&protected_dir);
    let _ = std::fs::remove_dir_all(&workspace);
}
