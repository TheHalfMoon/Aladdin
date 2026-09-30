//! `cotra-mcp-host`: the stdio command the official tunnel client runs. It
//! resolves the install from its own location, refuses to run from an
//! inactive version, and runs the compiled `cotra-mcp` app with the validated
//! Node.js runtime, the matching `cotrad`, the configured workspaces, and the
//! sanitized environment. It never writes to stdout itself, because stdout is
//! the MCP channel.

use crate::config::Config;
use crate::layout::{read_json, CurrentRecord, InstallRecord, Layout};
use crate::LifecycleError;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The resolved launch for one MCP session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLaunch {
    pub node: PathBuf,
    pub script: PathBuf,
    pub cotrad: PathBuf,
}

/// Resolves the launch for a host binary located at `exe`.
pub fn resolve(exe: &Path) -> Result<(HostLaunch, Config), LifecycleError> {
    let version_dir = exe
        .parent()
        .ok_or_else(|| LifecycleError::state("cotra-mcp-host has no parent directory"))?;
    let versions = version_dir
        .parent()
        .filter(|dir| dir.file_name().is_some_and(|name| name == "versions"))
        .ok_or_else(|| {
            LifecycleError::state("cotra-mcp-host is not inside an installed version")
        })?;
    let root = versions
        .parent()
        .ok_or_else(|| LifecycleError::state("install root not found"))?;
    let layout = Layout::new(root);
    let current: CurrentRecord = read_json(&layout.current_file())?
        .ok_or_else(|| LifecycleError::not_installed("Cotra is not installed"))?;
    if layout.version_dir(&current.version) != version_dir {
        return Err(LifecycleError::state(
            "cotra-mcp-host belongs to an inactive version; restart Cotra",
        ));
    }
    let record: InstallRecord = read_json(&layout.install_file())?
        .ok_or_else(|| LifecycleError::state("install.json is missing"))?;
    let config = Config::load(&layout)?;
    config.check_workspaces_with_policy()?;
    let launch = HostLaunch {
        node: PathBuf::from(record.node_path),
        script: version_dir
            .join("app")
            .join("cotra-mcp")
            .join("dist")
            .join("index.js"),
        cotrad: version_dir.join("cotrad.exe"),
    };
    for (label, path) in [
        ("Node.js runtime", &launch.node),
        ("cotra-mcp app", &launch.script),
        ("cotrad", &launch.cotrad),
    ] {
        if !path.is_file() {
            return Err(LifecycleError::state(format!("{label} is missing")));
        }
    }
    Ok((launch, config))
}

/// Runs the MCP app for the current process and returns its exit code.
pub fn run() -> i32 {
    let outcome = std::env::current_exe()
        .map_err(|error| LifecycleError::io("locate cotra-mcp-host", error))
        .and_then(|exe| resolve(&exe));
    let (launch, config) = match outcome {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("cotra-mcp-host: {}", error.message);
            return 2;
        }
    };
    let mut env = crate::ipc::child_environment(&config);
    env.insert(
        "COTRA_DAEMON".into(),
        launch.cotrad.to_string_lossy().into_owned(),
    );
    let status = Command::new(&launch.node)
        .arg(&launch.script)
        .env_clear()
        .envs(env)
        .status();
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("cotra-mcp-host: start Node.js: {error}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkspaceEntry;
    use crate::layout::{write_json_atomic, CURRENT_SCHEMA, INSTALL_SCHEMA};
    use crate::test_support::temp_dir;

    fn fake_install(active: &str) -> (Layout, PathBuf) {
        let base = temp_dir("mcp-host");
        let layout = Layout::new(base.join("Cotra"));
        let version_dir = layout.version_dir(active);
        std::fs::create_dir_all(version_dir.join("app/cotra-mcp/dist")).unwrap();
        std::fs::write(version_dir.join("app/cotra-mcp/dist/index.js"), b"").unwrap();
        std::fs::write(version_dir.join("cotrad.exe"), b"").unwrap();
        let node = base.join("node.exe");
        std::fs::write(&node, b"").unwrap();
        write_json_atomic(
            &layout.current_file(),
            &CurrentRecord {
                schema: CURRENT_SCHEMA.into(),
                version: active.into(),
                manifest_sha256: "0".repeat(64),
            },
        )
        .unwrap();
        write_json_atomic(
            &layout.install_file(),
            &InstallRecord {
                schema: INSTALL_SCHEMA.into(),
                active: active.into(),
                previous: None,
                node_path: node.to_string_lossy().into_owned(),
                path_entry_added: false,
            },
        )
        .unwrap();
        let project = base.join("project");
        std::fs::create_dir_all(&project).unwrap();
        Config {
            workspaces: vec![WorkspaceEntry {
                id: "default".into(),
                root: project,
            }],
            ..Config::default()
        }
        .save(&layout)
        .unwrap();
        (layout, version_dir.join("cotra-mcp-host.exe"))
    }

    #[test]
    fn resolves_the_active_version_payload() {
        let (layout, exe) = fake_install("0.2.0");
        let (launch, config) = resolve(&exe).unwrap();
        assert_eq!(
            launch.cotrad,
            layout.version_dir("0.2.0").join("cotrad.exe")
        );
        assert_eq!(config.default_workspace(), Some("default"));
    }

    #[test]
    fn inactive_version_and_foreign_locations_fail_closed() {
        let (layout, _) = fake_install("0.2.0");
        let stale = layout.version_dir("0.1.0").join("cotra-mcp-host.exe");
        assert!(resolve(&stale).is_err());
        assert!(resolve(&std::env::temp_dir().join("cotra-mcp-host.exe")).is_err());
    }

    #[test]
    fn missing_workspaces_fail_closed() {
        let (layout, exe) = fake_install("0.2.0");
        Config::default().save(&layout).unwrap();
        assert!(resolve(&exe).is_err());
    }
}
