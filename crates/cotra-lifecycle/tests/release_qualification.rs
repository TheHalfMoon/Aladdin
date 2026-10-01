//! Release-artifact qualification. Runs only when `COTRA_RELEASE_DIR` names a
//! release directory produced by `scripts/package-release.mjs`, as the
//! release-qualification CI job does. It installs the packaged artifact (the
//! real `cotra-mcp` app with its production dependencies), drives MCP through
//! the installed runtime, and exercises update, downgrade refusal, rollback,
//! failed-update recovery, and uninstall. The `fake_tunnel_client` example
//! stands in for the official OpenAI tunnel client only.
#![cfg(windows)]

use cotra_lifecycle::manifest::{sha256_bytes, MANIFEST_FILE};
use cotra_lifecycle::platform::find_node_on_path;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const SECRET: &str = "sk-release-qualification-key-0123456789";
const TUNNEL_ID: &str = "tunnel_0123456789abcdefghijklmnopqrstuv";

/// The closed MCP tool set registered by `apps/cotra-mcp`.
const CANONICAL_TOOLS: &[&str] = &[
    "system_status",
    "workspace_get",
    "fs_stat",
    "fs_list",
    "fs_read",
    "fs_search",
    "fs_write_preview",
    "fs_write",
    "process_spawn",
    "git_status",
    "git_diff",
    "git_log",
    "git_branch_create",
    "git_stage",
    "git_unstage",
    "git_commit",
    "git_fetch_preview",
    "git_fetch",
    "git_push_preview",
    "git_push",
];

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cotra-rq-{label}-{}", cotra_lifecycle::nonce()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fake_tunnel() -> PathBuf {
    let deps = std::env::current_exe().unwrap();
    let profile = deps.parent().unwrap().parent().unwrap();
    let path = profile.join("examples").join("fake_tunnel_client.exe");
    assert!(
        path.is_file(),
        "build the fake_tunnel_client example first: {}",
        path.display()
    );
    path
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Copies a release under a new version, optionally replacing one payload
/// file, and rewrites the manifest to match.
fn derived_release(source: &Path, version: &str, replace: Option<(&str, &[u8])>) -> PathBuf {
    let dir = temp_dir(&format!("release-{version}"));
    copy_tree(source, &dir);
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(dir.join(MANIFEST_FILE)).unwrap()).unwrap();
    manifest["version"] = Value::from(version);
    if let Some((path, bytes)) = replace {
        fs::write(cotra_lifecycle::manifest::join_relative(&dir, path), bytes).unwrap();
        for file in manifest["files"].as_array_mut().unwrap() {
            if file["path"] == path {
                file["size"] = Value::from(bytes.len() as u64);
                file["sha256"] = Value::from(sha256_bytes(bytes));
            }
        }
    }
    fs::write(
        dir.join(MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    dir
}

fn cotra(exe: &Path, local: &Path, args: &[&str]) -> Output {
    let output = Command::new(exe)
        .args(args)
        .env("LOCALAPPDATA", local)
        .output()
        .unwrap();
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !all.contains(SECRET),
        "runtime key leaked into `cotra {args:?}` output"
    );
    output
}

fn json_ok(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "status {:?}\nstdout {}\nstderr {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn wait_for_count(path: &Path, needle: &str, count: usize) -> String {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let text = fs::read_to_string(path).unwrap_or_default();
        if text.matches(needle).count() >= count {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "{needle} x{count} not logged; log:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn bump_patch(version: &str, by: u64) -> String {
    let core = version.split('-').next().unwrap();
    let parts = core
        .split('.')
        .map(|p| p.parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    format!("{}.{}.{}", parts[0], parts[1], parts[2] + by)
}

#[test]
#[ignore = "requires COTRA_RELEASE_DIR; run by the release-qualification CI job"]
fn packaged_release_installs_runs_updates_recovers_and_uninstalls() {
    let release = PathBuf::from(
        std::env::var_os("COTRA_RELEASE_DIR")
            .expect("COTRA_RELEASE_DIR must name a packaged release"),
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(release.join(MANIFEST_FILE)).unwrap()).unwrap();
    let base_version = manifest["version"].as_str().unwrap().to_string();
    let node = find_node_on_path().expect("Node.js is required on PATH");
    let tunnel = fake_tunnel();
    let local = temp_dir("local");
    let root = local.join("Cotra");
    let project = temp_dir("project");
    fs::write(project.join("hello.txt"), b"hello from the workspace").unwrap();
    let key_file = temp_dir("key").join("runtime.key");
    fs::write(&key_file, SECRET).unwrap();
    let tunnel_log = root.join("logs").join("tunnel.log");

    // Install the packaged artifact from its own directory.
    let installed = json_ok(&cotra(
        &release.join("cotra.exe"),
        &local,
        &[
            "install",
            "--no-path",
            "--node",
            node.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(installed["install"]["version"], base_version.as_str());
    let cli = root.join("bin").join("cotra.exe");
    json_ok(&cotra(
        &cli,
        &local,
        &[
            "workspace",
            "add",
            "default",
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    json_ok(&cotra(
        &cli,
        &local,
        &[
            "tunnel",
            "setup",
            "--client",
            tunnel.to_str().unwrap(),
            "--tunnel-id",
            TUNNEL_ID,
            "--key-file",
            key_file.to_str().unwrap(),
            "--json",
        ],
    ));

    // Start and drive MCP through the installed real app.
    let started = json_ok(&cotra(&cli, &local, &["start", "--json"]));
    assert_eq!(started["status"]["state"], "running", "{started}");
    let log = wait_for_count(&tunnel_log, "fake-tunnel: mcp-call", 1);
    assert!(
        log.contains("fake-tunnel: mcp-initialize server=cotra"),
        "{log}"
    );
    let tools_line = log
        .lines()
        .find(|line| line.contains("fake-tunnel: mcp-tools"))
        .unwrap()
        .to_string();
    // The exposed MCP surface must be exactly the canonical closed tool set:
    // any added tool (clipboard, network, browser, UI automation, trust,
    // shell, ...) is exposure drift and fails qualification.
    let listed = tools_line
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(names, _)| names.split(',').map(str::to_string).collect::<Vec<_>>())
        .unwrap();
    let mut listed_sorted = listed.clone();
    listed_sorted.sort();
    let mut canonical = CANONICAL_TOOLS
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    canonical.sort();
    assert_eq!(listed_sorted, canonical, "MCP exposure drift: {tools_line}");
    assert!(
        log.contains("fake-tunnel: mcp-call is_error=false cotrad_answered=true"),
        "{log}"
    );
    assert!(log.contains("fake-tunnel: env-clean=true"), "{log}");
    assert!(log.contains("api_key=[REDACTED]"), "{log}");

    let doctor = json_ok(&cotra(&cli, &local, &["doctor", "--json"]));
    assert_eq!(doctor["doctor"]["healthy"], true, "{doctor}");

    // Update while running: stopped, switched, verified by the new CLI, restarted.
    let next = bump_patch(&base_version, 1);
    let candidate = derived_release(&release, &next, None);
    let checked = json_ok(&cotra(
        &cli,
        &local,
        &[
            "update",
            "--source",
            candidate.to_str().unwrap(),
            "--check",
            "--json",
        ],
    ));
    assert_eq!(checked["update"]["direction"], "upgrade");
    let updated = json_ok(&cotra(
        &cli,
        &local,
        &["update", "--source", candidate.to_str().unwrap(), "--json"],
    ));
    assert_eq!(updated["update"]["to"], next.as_str());
    assert_eq!(updated["update"]["restarted"], true, "{updated}");
    wait_for_count(
        &tunnel_log,
        "fake-tunnel: mcp-call is_error=false cotrad_answered=true",
        2,
    );
    let version = json_ok(&cotra(&cli, &local, &["version", "--json"]));
    assert_eq!(version["installed"], next.as_str());
    assert_eq!(version["previous"], base_version.as_str());

    // Downgrades are refused unless explicitly allowed.
    let downgrade = cotra(
        &cli,
        &local,
        &["update", "--source", release.to_str().unwrap(), "--json"],
    );
    assert_eq!(downgrade.status.code(), Some(6), "{downgrade:?}");

    // Rollback to the retained previous version.
    let rolled = json_ok(&cotra(&cli, &local, &["rollback", "--json"]));
    assert_eq!(rolled["rollback"]["to"], base_version.as_str());
    assert_eq!(rolled["rollback"]["restarted"], true);
    wait_for_count(
        &tunnel_log,
        "fake-tunnel: mcp-call is_error=false cotrad_answered=true",
        3,
    );

    // A release whose CLI cannot verify itself must be rolled back automatically.
    let bad_version = bump_patch(&base_version, 2);
    let broken_cli = fs::read(&tunnel).unwrap();
    let broken = derived_release(&release, &bad_version, Some(("cotra.exe", &broken_cli)));
    let failed = cotra(
        &cli,
        &local,
        &["update", "--source", broken.to_str().unwrap(), "--json"],
    );
    assert_eq!(failed.status.code(), Some(7), "{failed:?}");
    let failure = String::from_utf8_lossy(&failed.stdout);
    assert!(failure.contains("was restored"), "{failure}");
    let version = json_ok(&cotra(&cli, &local, &["version", "--json"]));
    assert_eq!(version["installed"], base_version.as_str());
    assert!(!root.join("versions").join(&bad_version).exists());
    let status = json_ok(&cotra(&cli, &local, &["status", "--json"]));
    assert_eq!(status["runtime"]["state"], "running", "{status}");
    let doctor = cotra(&cli, &local, &["doctor", "--json"]);
    let doctor_text = String::from_utf8_lossy(&doctor.stdout);
    assert!(doctor_text.contains("update_state"), "{doctor_text}");
    assert!(doctor_text.contains("was restored"), "{doctor_text}");

    // Stop and uninstall; user data is retained until purged.
    let stopped = json_ok(&cotra(&cli, &local, &["stop", "--json"]));
    assert_eq!(stopped["stop"]["outcome"], "stopped_verified");
    json_ok(&cotra(
        &release.join("cotra.exe"),
        &local,
        &["uninstall", "--json"],
    ));
    assert!(root.join("state").join("config.json").is_file());
    for log in ["tunnel.log", "supervisor.log", "lifecycle.log"] {
        let text = fs::read_to_string(root.join("logs").join(log)).unwrap_or_default();
        assert!(!text.contains(SECRET), "{log} contains the runtime key");
    }
    let transcript = fs::read_to_string(root.join("logs").join("lifecycle.log")).unwrap();
    assert!(
        transcript.contains("failed and was rolled back"),
        "{transcript}"
    );
    json_ok(&cotra(
        &release.join("cotra.exe"),
        &local,
        &["uninstall", "--purge-data", "--yes", "--json"],
    ));
    assert!(!root.exists());
}
