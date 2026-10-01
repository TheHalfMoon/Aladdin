//! `cotra-mcp-host`: the stdio command the official tunnel client runs. It
//! resolves the install from its own location, verifies the whole installed
//! payload and the recorded Node.js runtime for every session, refuses to run
//! from an inactive version, and runs the compiled `cotra-mcp` app with the
//! validated Node.js runtime, the matching `cotrad`, the configured
//! workspaces, and the sanitized environment. It never writes to stdout
//! itself, because stdout is the MCP channel.

use crate::config::Config;
use crate::install::Installer;
use crate::layout::Layout;
use crate::platform::{Platform, MIN_NODE_MAJOR};
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

/// Resolves and verifies the launch for a host binary located at `exe`.
pub fn resolve(
    exe: &Path,
    platform: &dyn Platform,
) -> Result<(HostLaunch, Config), LifecycleError> {
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
    let state = Installer::new(layout.clone(), platform).verify()?;
    if layout.version_dir(&state.active.version) != version_dir {
        return Err(LifecycleError::state(
            "cotra-mcp-host belongs to an inactive version; restart Cotra",
        ));
    }
    let node = PathBuf::from(&state.record.node_path);
    let node_version = platform.node_version(&node)?;
    if node_version.major < MIN_NODE_MAJOR {
        return Err(LifecycleError::prerequisite(format!(
            "Node.js {node_version} is older than the required {MIN_NODE_MAJOR}"
        )));
    }
    let config = Config::load(&layout)?;
    config.check_workspaces_with_policy()?;
    let launch = HostLaunch {
        node,
        script: version_dir
            .join("app")
            .join("cotra-mcp")
            .join("dist")
            .join("index.js"),
        cotrad: version_dir.join("cotrad.exe"),
    };
    Ok((launch, config))
}

/// Runs the MCP app for the current process and returns its exit code.
pub fn run() -> i32 {
    let platform = crate::host_platform();
    let outcome = std::env::current_exe()
        .map_err(|error| LifecycleError::io("locate cotra-mcp-host", error))
        .and_then(|exe| resolve(&exe, platform.as_ref()));
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
    use crate::install::InstallOptions;
    use crate::test_support::{release_dir, temp_dir, FakePlatform};

    fn install(version: &str, platform: &FakePlatform) -> (Layout, PathBuf) {
        let base = temp_dir("mcp-host");
        let layout = Layout::new(base.join("Cotra"));
        Installer::new(layout.clone(), platform)
            .install(
                &release_dir(version),
                &InstallOptions {
                    node: Some(PathBuf::from("/fake/node")),
                    add_to_path: false,
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
        let exe = layout.version_dir(version).join("cotra-mcp-host.exe");
        (layout, exe)
    }

    #[test]
    fn resolves_the_active_verified_version_payload() {
        let platform = FakePlatform::default();
        let (layout, exe) = install("0.2.0", &platform);
        let (launch, config) = resolve(&exe, &platform).unwrap();
        assert_eq!(
            launch.cotrad,
            layout.version_dir("0.2.0").join("cotrad.exe")
        );
        assert_eq!(config.default_workspace(), Some("default"));
    }

    #[test]
    fn tampered_payload_fails_closed_for_each_session() {
        let platform = FakePlatform::default();
        let (layout, exe) = install("0.2.0", &platform);
        std::fs::write(
            layout
                .version_dir("0.2.0")
                .join("app/cotra-mcp/dist/index.js"),
            b"replaced",
        )
        .unwrap();
        assert!(resolve(&exe, &platform).is_err());
    }

    #[test]
    fn inactive_version_foreign_location_and_old_node_fail_closed() {
        let platform = FakePlatform::default();
        let (layout, _) = install("0.2.0", &platform);
        let stale = layout.version_dir("0.1.0").join("cotra-mcp-host.exe");
        assert!(resolve(&stale, &platform).is_err());
        assert!(resolve(&std::env::temp_dir().join("cotra-mcp-host.exe"), &platform).is_err());
        let (_, exe) = install("0.2.0", &platform);
        let old_node = FakePlatform {
            node_major: 18,
            ..FakePlatform::default()
        };
        assert!(resolve(&exe, &old_node).is_err());
    }

    #[test]
    fn missing_workspaces_fail_closed() {
        let platform = FakePlatform::default();
        let (layout, exe) = install("0.2.0", &platform);
        Config::default().save(&layout).unwrap();
        assert!(resolve(&exe, &platform).is_err());
    }
}
