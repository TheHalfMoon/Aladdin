//! Windows-native lifecycle qualification with real binaries: `cotra`,
//! `cotra-mcp-host`, `cotrad`, and Node.js. The tunnel client is the
//! `fake_tunnel_client` example, which stands in for the official OpenAI
//! client only; real ChatGPT connectivity is outside this test.
#![cfg(windows)]

use cotra_lifecycle::manifest::{
    sha256_bytes, ConfigSchemaRange, Manifest, ManifestFile, MANIFEST_FILE, MANIFEST_SCHEMA,
};
use cotra_lifecycle::platform::find_node_on_path;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const SECRET: &str = "sk-e2e-runtime-key-0123456789abcdef";
const TUNNEL_ID: &str = "tunnel_0123456789abcdefghijklmnopqrstuv";

const FAKE_APP: &str = r#"import { spawn } from "node:child_process";
import readline from "node:readline";
const rl = readline.createInterface({ input: process.stdin });
rl.on("line", () => {
  const child = spawn(process.env.COTRA_DAEMON, [], { stdio: ["pipe", "pipe", "ignore"], env: process.env });
  let buffer = "";
  child.stdout.on("data", (chunk) => {
    buffer += chunk.toString();
    const newline = buffer.indexOf("\n");
    if (newline < 0) return;
    const response = JSON.parse(buffer.slice(0, newline));
    const secretish = Object.keys(process.env).filter((name) => /KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL/i.test(name));
    process.stdout.write(JSON.stringify({ cotrad_ok: response.ok, name: response.result && response.result.name, secretish }) + "\n");
    child.stdin.end();
  });
  child.stdin.write(JSON.stringify({ version: 1, request_id: "e2e", client_session_id: "e2e", workspace_id: process.env.COTRA_DEFAULT_WORKSPACE, capability: "system.status", operation: "get", target: null, arguments: {} }) + "\n");
});
"#;

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cotra-rt-{label}-{}", cotra_lifecycle::nonce()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn built(name: &str) -> PathBuf {
    let cli = PathBuf::from(env!("CARGO_BIN_EXE_cotra"));
    let dir = cli.parent().unwrap();
    for candidate in [dir.join(name), dir.join("examples").join(name)] {
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "{name} is not built next to {}; run the workspace test suite",
        cli.display()
    );
}

fn release() -> PathBuf {
    let dir = temp_dir("release");
    let payload: Vec<(&str, Vec<u8>)> = vec![
        ("cotra.exe", fs::read(env!("CARGO_BIN_EXE_cotra")).unwrap()),
        (
            "cotra-mcp-host.exe",
            fs::read(env!("CARGO_BIN_EXE_cotra-mcp-host")).unwrap(),
        ),
        ("cotrad.exe", fs::read(built("cotrad.exe")).unwrap()),
        ("LICENSE", b"Apache-2.0 test payload".to_vec()),
        (
            "app/cotra-mcp/package.json",
            br#"{"name":"@cotra/mcp","version":"0.0.0","private":true,"type":"module"}"#.to_vec(),
        ),
        ("app/cotra-mcp/dist/index.js", FAKE_APP.as_bytes().to_vec()),
    ];
    let mut files = Vec::new();
    for (path, bytes) in payload {
        let target = cotra_lifecycle::manifest::join_relative(&dir, path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, &bytes).unwrap();
        files.push(ManifestFile {
            path: path.into(),
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

fn cotra(exe: &Path, local: &Path, args: &[&str]) -> Output {
    Command::new(exe)
        .args(args)
        .env("LOCALAPPDATA", local)
        .output()
        .unwrap()
}

fn json_ok(output: &Output) -> Value {
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "status {:?}\nstdout {text}\nstderr {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!text.contains(SECRET), "secret leaked into CLI output");
    serde_json::from_str(&text).unwrap()
}

fn wait_for_log(path: &Path, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let text = fs::read_to_string(path).unwrap_or_default();
        if text.contains(needle) {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "{needle} not logged; log:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
fn install_configure_start_status_doctor_stop_restart_and_uninstall() {
    let node = find_node_on_path().expect("Node.js is required on PATH for this qualification");
    let fake_tunnel = built("fake_tunnel_client.exe");
    let release = release();
    let local = temp_dir("local");
    let root = local.join("Cotra");
    let project = temp_dir("project");
    let key_file = temp_dir("key").join("runtime.key");
    fs::write(&key_file, format!("{SECRET}\n")).unwrap();
    let exe = release.join("cotra.exe");

    json_ok(&cotra(
        &exe,
        &local,
        &[
            "install",
            "--no-path",
            "--node",
            node.to_str().unwrap(),
            "--json",
        ],
    ));
    let cli = root.join("bin").join("cotra.exe");

    // Starting before configuration fails closed.
    let unconfigured = cotra(&cli, &local, &["start", "--json"]);
    assert_eq!(unconfigured.status.code(), Some(7), "{unconfigured:?}");

    // A workspace overlapping protected state is refused.
    let refused = cotra(
        &cli,
        &local,
        &["workspace", "add", "bad", root.to_str().unwrap(), "--json"],
    );
    assert!(!refused.status.success());
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
            fake_tunnel.to_str().unwrap(),
            "--tunnel-id",
            TUNNEL_ID,
            "--key-file",
            key_file.to_str().unwrap(),
            "--json",
        ],
    ));
    let shown = json_ok(&cotra(&cli, &local, &["tunnel", "show", "--json"]));
    assert_eq!(shown["runtime_key_present"], true);

    let started = json_ok(&cotra(&cli, &local, &["start", "--json"]));
    assert_eq!(started["status"]["state"], "running", "{started}");
    let tunnel_pid = started["status"]["tunnel_client_pid"].as_u64().unwrap() as u32;

    let log = wait_for_log(&root.join("logs").join("tunnel.log"), "mcp-response");
    assert!(log.contains("fake-tunnel: key-file-read=true"), "{log}");
    assert!(log.contains("fake-tunnel: env-clean=true"), "{log}");
    assert!(log.contains("api_key=[REDACTED]"), "{log}");
    assert!(log.contains(r#""cotrad_ok":true"#), "{log}");
    assert!(log.contains(r#""name":"cotrad""#), "{log}");
    assert!(log.contains(r#""secretish":[]"#), "{log}");
    assert!(
        !log.contains(SECRET),
        "runtime key leaked into the tunnel log"
    );

    let already = cotra(&cli, &local, &["start", "--json"]);
    assert_eq!(already.status.code(), Some(6));

    let status = json_ok(&cotra(&cli, &local, &["status", "--json"]));
    assert_eq!(status["runtime"]["state"], "running");
    let doctor = json_ok(&cotra(&cli, &local, &["doctor", "--json"]));
    assert_eq!(doctor["doctor"]["healthy"], true, "{doctor}");
    let ipc = doctor["doctor"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "ipc")
        .unwrap();
    assert_eq!(ipc["status"], "pass", "{ipc}");

    let stopped = json_ok(&cotra(&cli, &local, &["stop", "--json"]));
    assert_eq!(stopped["stop"]["outcome"], "stopped_verified", "{stopped}");
    assert!(cotra_lifecycle::runtime::identify(tunnel_pid)
        .map(|identity| !identity
            .image
            .to_ascii_lowercase()
            .ends_with("fake_tunnel_client.exe"))
        .unwrap_or(true));
    let status = json_ok(&cotra(&cli, &local, &["status", "--json"]));
    assert_eq!(status["runtime"]["state"], "not_running");

    // Restart, then uninstall stops the running instance first.
    let restarted = json_ok(&cotra(&cli, &local, &["start", "--json"]));
    assert_eq!(restarted["status"]["state"], "running");
    let removed = json_ok(&cotra(&exe, &local, &["uninstall", "--json"]));
    assert!(removed["uninstall"]["retained"].as_array().unwrap().len() >= 2);
    assert!(!root.join("run").join("supervisor.json").exists());
    assert!(root
        .join("state")
        .join("secrets")
        .join("tunnel-runtime-key")
        .is_file());

    for path in [
        root.join("logs").join("tunnel.log"),
        root.join("logs").join("supervisor.log"),
        root.join("state").join("config.json"),
    ] {
        let text = fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !text.contains(SECRET),
            "{} contains the runtime key",
            path.display()
        );
    }
    json_ok(&cotra(
        &exe,
        &local,
        &["uninstall", "--purge-data", "--yes", "--json"],
    ));
    assert!(!root.exists());
}
