//! SG-000072 release-version consistency gate: every package, crate manifest,
//! plugin, release artifact descriptor, and runtime version report must carry
//! the single expected release line. Any drift fails the suite on every OS.
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// The single release line closed by SG-000072. Bumping the release requires
/// updating every site below together with this constant in one change.
const EXPECTED_RELEASE_VERSION: &str = "0.2.0";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn json_version(path: &Path, pointer: &str) -> String {
    let text = read(path);
    let value: Value =
        serde_json::from_str(&text).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()));
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing {pointer} in {}", path.display()))
        .to_owned()
}

fn workspace_cargo_version(root: &Path) -> String {
    let text = read(&root.join("Cargo.toml"));
    let section = text
        .split("[workspace.package]")
        .nth(1)
        .expect("missing [workspace.package] in Cargo.toml");
    let section = section.split('[').next().unwrap_or(section);
    for line in section.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version") {
            let value = rest.trim_start_matches([' ', '=']).trim();
            let value = value.trim_matches('"');
            if !value.is_empty() {
                return value.to_owned();
            }
        }
    }
    panic!("missing version in [workspace.package]");
}

#[test]
fn every_release_surface_reports_the_single_release_line() {
    let root = repo_root();
    let observed: Vec<(&str, String)> = vec![
        (
            "Cargo.toml [workspace.package]",
            workspace_cargo_version(&root),
        ),
        (
            "package.json",
            json_version(&root.join("package.json"), "/version"),
        ),
        (
            "apps/qdral-mcp/package.json",
            json_version(&root.join("apps/qdral-mcp/package.json"), "/version"),
        ),
        (
            "apps/qdral-relay/package.json",
            json_version(&root.join("apps/qdral-relay/package.json"), "/version"),
        ),
        (
            "apps/qdral-relay dependency on @qdral/mcp",
            json_version(
                &root.join("apps/qdral-relay/package.json"),
                "/dependencies/@qdral~1mcp",
            ),
        ),
        (
            "distribution/codex/qdral/plugin.json",
            json_version(
                &root.join("distribution/codex/qdral/plugin.json"),
                "/version",
            ),
        ),
        (
            "examples/mcp-clients/claude-desktop-mcpb-manifest.json",
            json_version(
                &root.join("examples/mcp-clients/claude-desktop-mcpb-manifest.json"),
                "/version",
            ),
        ),
        (
            "package-lock.json root",
            json_version(&root.join("package-lock.json"), "/version"),
        ),
        (
            "package-lock.json packages[\"\"]",
            json_version(&root.join("package-lock.json"), "/packages//version"),
        ),
        (
            "package-lock.json apps/qdral-mcp",
            json_version(
                &root.join("package-lock.json"),
                "/packages/apps~1qdral-mcp/version",
            ),
        ),
        (
            "package-lock.json apps/qdral-relay",
            json_version(
                &root.join("package-lock.json"),
                "/packages/apps~1qdral-relay/version",
            ),
        ),
    ];

    let server = read(&root.join("apps/qdral-mcp/src/server.ts"));
    let marker = format!("version: \"{EXPECTED_RELEASE_VERSION}\"");
    assert!(
        server.contains(&marker),
        "apps/qdral-mcp/src/server.ts must report McpServer {marker}"
    );
    for stale in ["0.1.0", "0.3.0"] {
        assert!(
            !server.contains(&format!("version: \"{stale}\"")),
            "apps/qdral-mcp/src/server.ts carries a stale McpServer version"
        );
    }

    for (site, version) in &observed {
        assert_eq!(
            version, EXPECTED_RELEASE_VERSION,
            "version drift at {site}: expected {EXPECTED_RELEASE_VERSION}, found {version}"
        );
    }
}

#[test]
fn cargo_lock_pins_every_qdral_crate_to_the_release_line() {
    let root = repo_root();
    let text = read(&root.join("Cargo.lock"));
    let mut checked = 0;
    let mut current_name: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(stripped) = line.strip_prefix("name = ") {
            current_name = Some(stripped.trim_matches('"').to_owned());
        } else if let Some(stripped) = line.strip_prefix("version = ") {
            if let Some(name) = current_name.take() {
                if name == "qdrald" || name.starts_with("qdral-") {
                    let version = stripped.trim_matches('"');
                    assert_eq!(
                        version, EXPECTED_RELEASE_VERSION,
                        "Cargo.lock pins {name} at {version}, expected {EXPECTED_RELEASE_VERSION}"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(
        checked >= 14,
        "expected at least 14 qdral workspace crates in Cargo.lock, found {checked}"
    );
}
