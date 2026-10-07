//! Supported local stdio entrypoint (`qdral mcp stdio`).
//!
//! This command launches the authoritative Qdral MCP server for local AI
//! clients without requiring any tunnel. It resolves the active verified
//! install, enforces the same per-session checks as `qdral-mcp-host`
//! (payload verification, Node.js floor, workspace policy), and spawns the
//! identical verified launch (recorded Node.js runtime plus
//! `app/qdral-mcp/dist/index.js` with the sanitized environment and the
//! matching `qdrald`). Standard input and output are inherited untouched
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
                .join("qdral-mcp")
                .join("dist")
                .join("index.js"),
            qdrald: version_dir.join("qdrald.exe"),
        },
        config,
    ))
}

/// Runs one stdio MCP session over inherited standard I/O and returns the
/// child exit code. This function does not return normally with output;
/// callers must exit the process with the returned code.
pub fn run_stdio_session(launch: &HostLaunch, config: &Config) -> i32 {
    let layout = match Layout::for_current_user() {
        Ok(layout) => layout,
        Err(error) => {
            eprintln!("qdral mcp stdio: {}", error.message);
            return 2;
        }
    };
    let session = match crate::full_control::begin_runtime_session(&layout) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("qdral mcp stdio: {}", error.message);
            return 2;
        }
    };
    let mut env = crate::ipc::child_environment(config);
    env.insert(
        "QDRAL_DAEMON".into(),
        launch.qdrald.to_string_lossy().into_owned(),
    );
    env.insert(
        crate::full_control::RUNTIME_SESSION_ENV.into(),
        session.session_id.clone(),
    );
    let status = Command::new(&launch.node)
        .arg(&launch.script)
        .env_clear()
        .envs(env)
        .status();
    crate::full_control::end_runtime_session(&layout, &session.session_id);
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("qdral mcp stdio: start Node.js: {error}");
            2
        }
    }
}

/// Handles the `stdio` and `serve` actions. `stdio` runs one MCP session
/// over inherited standard I/O; `serve` starts the loopback HTTP listener.
/// Returns the child exit code. Unknown actions fail closed as usage errors
/// without touching stdio or opening any socket.
pub fn run_action(action: &str, port: Option<u16>) -> Result<i32, LifecycleError> {
    match action {
        "stdio" => {
            if port.is_some() {
                return Err(LifecycleError::usage(
                    "qdral mcp stdio takes no --port; run `qdral help`",
                ));
            }
            let layout = Layout::for_current_user()?;
            let platform = crate::host_platform();
            let (launch, config) =
                resolve_stdio_launch(&layout, platform.as_ref()).map_err(|error| {
                    LifecycleError::state(format!("qdral mcp stdio: {}", error.message))
                })?;
            Ok(run_stdio_session(&launch, &config))
        }
        "serve" => {
            let layout = Layout::for_current_user()?;
            let platform = crate::host_platform();
            let token = std::env::var("QDRAL_LOOPBACK_TOKEN").unwrap_or_default();
            let (launch, config) = resolve_serve_launch(&layout, platform.as_ref(), port, &token)
                .map_err(|error| {
                if error.kind == crate::ErrorKind::Usage {
                    error
                } else {
                    LifecycleError::state(format!("qdral mcp serve: {}", error.message))
                }
            })?;
            Ok(run_serve_session(&launch, &config))
        }
        _ => Err(LifecycleError::usage(
            "unknown mcp action; run `qdral help`",
        )),
    }
}

/// The resolved launch for one loopback HTTP listener.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeLaunch {
    pub node: PathBuf,
    pub script: PathBuf,
    pub qdrald: PathBuf,
    pub port: Option<u16>,
    pub token: String,
}

/// Minimum loopback credential length in characters. Mirrors the Node
/// transport floor so both layers fail closed on the same weak secrets.
pub const MIN_LOOPBACK_TOKEN_CHARS: usize = 32;

/// Resolves the loopback listener launch for the active verified install.
/// The credential comes only from the caller's environment and is never
/// generated, stored, or logged here. There is no bind-address option:
/// the Node transport always binds `127.0.0.1`.
pub fn resolve_serve_launch(
    layout: &Layout,
    platform: &dyn crate::platform::Platform,
    port: Option<u16>,
    token: &str,
) -> Result<(ServeLaunch, Config), LifecycleError> {
    if port.is_some_and(|port| port == 0) {
        return Err(LifecycleError::usage(
            "qdral mcp serve --port must be from 1 to 65535",
        ));
    }
    if token.chars().count() < MIN_LOOPBACK_TOKEN_CHARS {
        return Err(LifecycleError::usage(
            "QDRAL_LOOPBACK_TOKEN of at least 32 characters is required to serve loopback MCP",
        ));
    }
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
        ServeLaunch {
            node,
            script: version_dir
                .join("app")
                .join("qdral-mcp")
                .join("dist")
                .join("entrypoints")
                .join("loopback_http.js"),
            qdrald: version_dir.join("qdrald.exe"),
            port,
            token: token.to_string(),
        },
        config,
    ))
}

/// Runs the loopback listener with inherited standard I/O and returns the
/// child exit code. The bound URL is reported on standard error only.
pub fn run_serve_session(launch: &ServeLaunch, config: &Config) -> i32 {
    let layout = match Layout::for_current_user() {
        Ok(layout) => layout,
        Err(error) => {
            eprintln!("qdral mcp serve: {}", error.message);
            return 2;
        }
    };
    let session = match crate::full_control::begin_runtime_session(&layout) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("qdral mcp serve: {}", error.message);
            return 2;
        }
    };
    let mut env = crate::ipc::child_environment(config);
    env.insert(
        "QDRAL_DAEMON".into(),
        launch.qdrald.to_string_lossy().into_owned(),
    );
    env.insert(
        crate::full_control::RUNTIME_SESSION_ENV.into(),
        session.session_id.clone(),
    );
    env.insert("QDRAL_LOOPBACK_TOKEN".into(), launch.token.clone());
    if let Some(port) = launch.port {
        env.insert("QDRAL_LOOPBACK_PORT".into(), port.to_string());
    }
    let status = Command::new(&launch.node)
        .arg(&launch.script)
        .env_clear()
        .envs(env)
        .status();
    crate::full_control::end_runtime_session(&layout, &session.session_id);
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("qdral mcp serve: start Node.js: {error}");
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

    fn install(version: &str, platform: &FakePlatform) -> Layout {
        let base = temp_dir("mcp-stdio");
        let layout = Layout::new(base.join("Qdral"));
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
                .join("qdral-mcp")
                .join("dist")
                .join("index.js")
        );
        assert_eq!(launch.qdrald, version_dir.join("qdrald.exe"));
        assert_eq!(config.default_workspace(), Some("default"));
        let exe = version_dir.join("qdral-mcp-host.exe");
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
                .join("app/qdral-mcp/dist/index.js"),
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
        let error = run_action("http", None).unwrap_err();
        assert_eq!(error.kind, crate::ErrorKind::Usage);
    }

    #[test]
    fn serve_launch_points_at_the_loopback_entrypoint() {
        let platform = FakePlatform::default();
        let layout = install("0.2.0", &platform);
        let token = "0123456789abcdef0123456789abcdef";
        let (launch, _) = resolve_serve_launch(&layout, &platform, Some(8080), token).unwrap();
        assert_eq!(
            launch.script,
            layout
                .version_dir("0.2.0")
                .join("app/qdral-mcp/dist/entrypoints/loopback_http.js")
        );
        assert_eq!(
            launch.qdrald,
            layout.version_dir("0.2.0").join("qdrald.exe")
        );
        assert_eq!(launch.port, Some(8080));
        assert_eq!(launch.token, token);
    }

    #[test]
    fn serve_without_credential_zero_port_or_tampering_fails_closed() {
        let platform = FakePlatform::default();
        let layout = install("0.2.0", &platform);
        let token = "0123456789abcdef0123456789abcdef";
        assert!(resolve_serve_launch(&layout, &platform, None, "").is_err());
        assert!(resolve_serve_launch(&layout, &platform, None, "short").is_err());
        assert!(resolve_serve_launch(&layout, &platform, Some(0), token).is_err());
        std::fs::write(
            layout
                .version_dir("0.2.0")
                .join("app/qdral-mcp/dist/index.js"),
            b"replaced",
        )
        .unwrap();
        assert!(resolve_serve_launch(&layout, &platform, None, token).is_err());
    }
}
