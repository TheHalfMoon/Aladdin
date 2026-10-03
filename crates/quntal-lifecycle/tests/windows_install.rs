//! Windows-native install qualification: real ACLs, a real registry PATH value
//! under an isolated test key, and the real `quntal` binary installing from a
//! synthetic release into an isolated LOCALAPPDATA.
#![cfg(windows)]

use quntal_lifecycle::manifest::{
    sha256_bytes, ConfigSchemaRange, Manifest, ManifestFile, MANIFEST_FILE, MANIFEST_SCHEMA,
    REQUIRED_FILES,
};
use quntal_lifecycle::platform::{find_node_on_path, Platform, WindowsPlatform};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn temp_dir(label: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("quntal-win-{label}-{}", quntal_lifecycle::nonce()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn icacls(args: &[&str]) {
    let status = Command::new("icacls")
        .args(args)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "icacls {args:?}");
}

#[test]
fn platform_reports_real_build_and_elevation() {
    let platform = WindowsPlatform::default();
    assert!(platform.windows_build().unwrap() >= 17_763);
    platform.is_avoidably_elevated().unwrap();
}

#[test]
fn protect_tree_resets_foreign_access_and_verification_detects_it() {
    let platform = WindowsPlatform::default();
    let root = temp_dir("acl");
    fs::create_dir_all(root.join("state").join("secrets")).unwrap();
    fs::write(root.join("state").join("secrets").join("key"), b"k").unwrap();
    // Grant BUILTIN\Users explicit read on a nested file before protection.
    let key = root.join("state").join("secrets").join("key");
    icacls(&[key.to_str().unwrap(), "/grant", "*S-1-5-32-545:R"]);

    platform.protect_tree(&root).unwrap();
    platform.verify_tree_acl(&root).unwrap();

    // A foreign ACE added after protection must be detected.
    icacls(&[key.to_str().unwrap(), "/grant", "*S-1-1-0:R"]);
    let error = platform.verify_tree_acl(&root).unwrap_err();
    assert!(
        error.message.contains("unexpected principal"),
        "{}",
        error.message
    );

    platform.protect_tree(&root).unwrap();
    platform.verify_tree_acl(&root).unwrap();
    let _ = fs::remove_dir_all(root);
}

#[test]
fn user_path_edits_are_idempotent_and_exact() {
    let key = format!(
        "Software\\Quntal-Test-{}\\Environment",
        quntal_lifecycle::nonce()
    );
    let platform = WindowsPlatform::with_environment_key(&key);
    let dir = Path::new(r"C:\Users\example\AppData\Local\Quntal\bin");
    assert!(platform.add_user_path(dir).unwrap());
    assert!(!platform.add_user_path(dir).unwrap());
    assert!(platform.remove_user_path(dir).unwrap());
    assert!(!platform.remove_user_path(dir).unwrap());
    let parent = key.trim_end_matches("\\Environment");
    let _ = Command::new("reg")
        .args(["delete", &format!("HKCU\\{parent}"), "/f"])
        .stdout(std::process::Stdio::null())
        .status();
}

#[test]
fn junction_inside_install_root_is_refused_before_acl_reset() {
    let node = find_node_on_path().expect("Node.js is required on PATH for this qualification");
    let release = release_with_real_cli();
    let local = temp_dir("junction");
    let root = local.join("Quntal");
    let outside = temp_dir("junction-target");
    fs::create_dir_all(root.join("state")).unwrap();
    let link = root.join("state").join("escape");
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&outside)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let before = fs::metadata(&outside).unwrap().permissions();
    let refused = quntal(
        &release.join("quntal.exe"),
        &local,
        &[
            "install",
            "--no-path",
            "--node",
            node.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(refused.status.code(), Some(7), "{:?}", refused);
    assert!(!root.join("current.json").exists());
    assert_eq!(fs::metadata(&outside).unwrap().permissions(), before);
    let _ = fs::remove_dir(&link);
    let _ = fs::remove_dir_all(local);
    let _ = fs::remove_dir_all(outside);
    let _ = fs::remove_dir_all(release);
}

/// Builds a release whose `quntal.exe` is the real CLI under test.
fn release_with_real_cli() -> PathBuf {
    let dir = temp_dir("release");
    let cli = fs::read(env!("CARGO_BIN_EXE_quntal")).unwrap();
    let mut files = Vec::new();
    for path in REQUIRED_FILES {
        let bytes = if *path == "quntal.exe" {
            cli.clone()
        } else {
            format!("{path} synthetic payload").into_bytes()
        };
        let target = quntal_lifecycle::manifest::join_relative(&dir, path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, &bytes).unwrap();
        files.push(ManifestFile {
            path: (*path).into(),
            size: bytes.len() as u64,
            sha256: sha256_bytes(&bytes),
        });
    }
    let manifest = Manifest {
        schema: MANIFEST_SCHEMA.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        config_schema: ConfigSchemaRange { min: 1, max: 1 },
        files,
    };
    fs::write(
        dir.join(MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    dir
}

fn quntal(exe: &Path, local_app_data: &Path, args: &[&str]) -> Output {
    Command::new(exe)
        .args(args)
        .env("LOCALAPPDATA", local_app_data)
        .output()
        .unwrap()
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn real_cli_installs_verifies_and_uninstalls_from_a_release() {
    let node = find_node_on_path().expect("Node.js is required on PATH for this qualification");
    let release = release_with_real_cli();
    let exe = release.join("quntal.exe");
    let local = temp_dir("localappdata");
    let root = local.join("Quntal");
    let node_arg = node.to_str().unwrap();

    // Tampered release is refused before anything is written.
    let tampered = release_with_real_cli();
    fs::write(tampered.join("quntald.exe"), b"tampered").unwrap();
    let refused = quntal(
        &tampered.join("quntal.exe"),
        &local,
        &["install", "--no-path", "--node", node_arg, "--json"],
    );
    assert_eq!(refused.status.code(), Some(4), "{:?}", refused);
    assert!(!root.join("current.json").exists());

    // Install from the extracted release directory (default source).
    let installed = quntal(
        &exe,
        &local,
        &["install", "--no-path", "--node", node_arg, "--json"],
    );
    assert!(installed.status.success(), "{:?}", installed);
    let report = json(&installed);
    assert_eq!(report["install"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["install"]["path_entry_added"], false);
    WindowsPlatform::default().verify_tree_acl(&root).unwrap();

    // The installed CLI runs and reports the install.
    let version = quntal(
        &root.join("bin").join("quntal.exe"),
        &local,
        &["version", "--json"],
    );
    assert!(version.status.success(), "{:?}", version);
    assert_eq!(json(&version)["installed"], env!("CARGO_PKG_VERSION"));

    // Repair after tampering with the installed payload.
    fs::write(
        root.join("versions")
            .join(env!("CARGO_PKG_VERSION"))
            .join("quntald.exe"),
        b"tampered",
    )
    .unwrap();
    let repaired = quntal(
        &exe,
        &local,
        &["install", "--no-path", "--node", node_arg, "--json"],
    );
    assert!(repaired.status.success(), "{:?}", repaired);
    assert_eq!(json(&repaired)["install"]["repaired_existing"], true);

    // User data survives a default uninstall.
    fs::write(root.join("state").join("config.json"), b"{}").unwrap();
    fs::write(root.join("audit.jsonl"), b"{}\n").unwrap();
    let removed = quntal(&exe, &local, &["uninstall", "--json"]);
    assert!(removed.status.success(), "{:?}", removed);
    assert!(!root.join("versions").exists());
    assert!(!root.join("bin").exists());
    assert!(root.join("state").join("config.json").is_file());
    assert!(root.join("audit.jsonl").is_file());
    let again = quntal(&exe, &local, &["uninstall", "--json"]);
    assert_eq!(again.status.code(), Some(5));

    // Purge requires explicit confirmation.
    let unconfirmed = quntal(&exe, &local, &["uninstall", "--purge-data"]);
    assert_eq!(unconfirmed.status.code(), Some(2));
    assert!(root.join("audit.jsonl").is_file());
    let purged = quntal(
        &exe,
        &local,
        &["uninstall", "--purge-data", "--yes", "--json"],
    );
    assert!(purged.status.success(), "{:?}", purged);
    assert!(!root.exists());

    let _ = fs::remove_dir_all(local);
    let _ = fs::remove_dir_all(release);
    let _ = fs::remove_dir_all(tampered);
}
