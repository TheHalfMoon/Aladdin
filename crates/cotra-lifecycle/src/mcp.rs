//! Supported local stdio entrypoint (`cotra mcp stdio`).
//!
//! This command launches the authoritative Cotra MCP server for local AI
//! clients without requiring any tunnel. It resolves the active verified
//! install, enforces the same per-session checks as `cotra-mcp-host`
//! (payload verification, Node.js floor, workspace policy), and spawns the
//! identical verified launch (recorded Node.js runtime plus
//! `app/cotra-mcp/dist/index.js` with the sanitized environment and the
//! matching `cotrad`). Standard input and output are inherited untouched
//! because stdout is the MCP channel. No tool, schema, capability,
//! approval, or network authority is added here.

use crate::config::Config;
use crate::install::Installer;
use crate::layout::Layout;
use crate::mcp_host::HostLaunch;
use crate::platform::MIN_NODE_MAJOR;
use crate::LifecycleError;
use std::path::PathBuf;
use std::process::Command;

/// Resolves the stdio launch for the active verified install.
pub fn resolve_stdio_launch(
    layout: &Layout,
    platform: &dyn crate::platform::Platform,
) -> Result<(HostLaunch, Config), LifecycleError> {
    let state = Installer::new(layout.clone(), platform).verify()?;
    let version_dir = layout.version_dir(&state.active.version);
    let node = PathBuf::from(&state.record.node_path);
    let node_version = platform.node_version(&node)?;
    if node_version.major < MIN_NODE_MAJOR {
        return Err(LifecycleError::prerequisite(format!(
            "Node.js {node_version} is older than the required {MIN_NODE_MAJOR}"
        )));
    }
    let config = Config::load(layout)?;
    config.check_workspaces_with_policy()?;
    Ok((
        HostLaunch {
            node,
            script: version_dir
                .join("app")
                .join("cotra-mcp")
                .join("dist")
                .join("index.js"),
            cotrad: version_dir.join("cotrad.exe"),
        },
        config,
    ))
}

/// Runs one stdio MCP session over inherited standard I/O and returns the
/// child exit code. This function does not return normally with output;
/// callers must exit the process with the returned code.
pub fn run_stdio_session(launch: &HostLaunch, config: &Config) -> i32 {
    let mut env = crate::ipc::child_environment(config);
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
            eprintln!("cotra mcp stdio: start Node.js: {error}");
            2
        }
    }
}

/// Handles the `stdio` action: resolves the active verified launch and runs
/// one session over inherited standard I/O. Returns the child exit code.
/// Unknown actions fail closed as usage errors without touching stdio.
pub fn run_action(action: &str) -> Result<i32, LifecycleError> {
    match action {
        "stdio" => {
            let layout = Layout::for_current_user()?;
            let platform = crate::host_platform();
            let (launch, config) =
                resolve_stdio_launch(&layout, platform.as_ref()).map_err(|error| {
                    LifecycleError::state(format!("cotra mcp stdio: {}", error.message))
                })?;
            Ok(run_stdio_session(&launch, &config))
        }
        _ => Err(LifecycleError::usage(
            "unknown mcp action; run `cotra help`",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkspaceEntry;
    use crate::install::InstallOptions;
    use crate::test_support::{release_dir, temp_dir, FakePlatform};

    fn install(version: &str, platform: &FakePlatform) -> Layout {
        let base = temp_dir("mcp-stdio");
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
        layout
    }

    #[test]
    fn stdio_launch_matches_the_tunnel_host_launch() {
        let platform = FakePlatform::default();
        let layout = install("0.2.0", &platform);
        let (launch, config) = resolve_stdio_launch(&layout, &platform).unwrap();
        let version_dir = layout.version_dir("0.2.0");
        assert_eq!(
            launch.script,
            version_dir
                .join("app")
                .join("cotra-mcp")
                .join("dist")
                .join("index.js")
        );
        assert_eq!(launch.cotrad, version_dir.join("cotrad.exe"));
        assert_eq!(config.default_workspace(), Some("default"));
        let exe = version_dir.join("cotra-mcp-host.exe");
        let (host_launch, _) = crate::mcp_host::resolve(&exe, &platform).unwrap();
        assert_eq!(launch, host_launch);
    }

    #[test]
    fn tampered_payload_fails_closed() {
        let platform = FakePlatform::default();
        let layout = install("0.2.0", &platform);
        std::fs::write(
            layout
                .version_dir("0.2.0")
                .join("app/cotra-mcp/dist/index.js"),
            b"replaced",
        )
        .unwrap();
        assert!(resolve_stdio_launch(&layout, &platform).is_err());
    }

    #[test]
    fn missing_workspaces_and_old_node_fail_closed() {
        let platform = FakePlatform::default();
        let layout = install("0.2.0", &platform);
        Config::default().save(&layout).unwrap();
        assert!(resolve_stdio_launch(&layout, &platform).is_err());
        let layout = install("0.2.0", &platform);
        let old_node = FakePlatform {
            node_major: 18,
            ..FakePlatform::default()
        };
        assert!(resolve_stdio_launch(&layout, &old_node).is_err());
    }

    #[test]
    fn unknown_action_fails_closed_as_usage() {
        let error = run_action("http").unwrap_err();
        assert_eq!(error.kind, crate::ErrorKind::Usage);
    }
}
