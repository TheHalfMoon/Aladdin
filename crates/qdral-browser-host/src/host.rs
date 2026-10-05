//! SG-000074 Deskal supervisor for one isolated browser host.
//!
//! The supervisor owns at most one live host: discovery, ephemeral
//! profile, supervised host-binary launch over a private piped channel,
//! hello handshake, ping, and deterministic shutdown. A second concurrent
//! launch is refused. Dropping the supervisor shuts the host down.

use crate::discovery::{discover_engine, Engine, SearchConfig};
use crate::error::{HostError, UnavailableReason};
use crate::profile::{adopt_profile, setup_ephemeral_profile, AutomationProfile};
use crate::proto::{decode_frame, write_frame, HostReply, HostRequest, PROTOCOL_GENERATION};
use crate::supervise::{spawn_host_piped, SupervisedChild};
use std::path::Path;
use std::time::{Duration, Instant};

/// Bounded handshake wait owned by the supervisor.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Server-issued host identity binding engine, profile, and process.
#[derive(Debug, Clone)]
pub struct HostIdentity {
    pub host_id: String,
    pub engine_fingerprint: String,
    pub profile_identity: String,
    pub pid: u32,
}

/// The single live host under supervision.
pub struct LiveHost {
    identity: HostIdentity,
    profile: AutomationProfile,
    child: SupervisedChild,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl LiveHost {
    pub fn identity(&self) -> &HostIdentity {
        &self.identity
    }

    /// Liveness check: the supervised process still runs.
    pub fn is_alive(&mut self) -> bool {
        self.child.is_alive()
    }

    /// Round-trip ping through the private channel.
    pub fn ping(&mut self, nonce: &str) -> Result<(), HostError> {
        write_frame(
            &mut self.stdin,
            &HostRequest::Ping {
                nonce: nonce.to_owned(),
            },
        )?;
        match decode_frame(&mut self.stdout) {
            Ok(HostReply::Pong { nonce: back }) if back == nonce => Ok(()),
            Ok(HostReply::Error { code }) => {
                Err(HostError::Invalid(format!("host error reply: {code}")))
            }
            Ok(_) => Err(HostError::Unavailable(UnavailableReason::ProtocolViolation)),
            Err(err) => Err(err),
        }
    }

    /// Deterministic shutdown: ask the host to stop, terminate the family
    /// on timeout, confirm emptiness, and delete the ephemeral profile.
    pub fn shutdown(self) -> Result<bool, HostError> {
        let LiveHost {
            identity: _,
            profile,
            mut child,
            mut stdin,
            mut stdout,
        } = self;
        let _ = write_frame(&mut stdin, &HostRequest::Shutdown);
        let deadline = Instant::now() + Duration::from_secs(2);
        while child.is_alive() && Instant::now() < deadline {
            match decode_frame::<HostReply, _>(&mut stdout) {
                Ok(HostReply::Bye) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        drop((stdin, stdout));
        let confirmed = child.shutdown()?;
        profile.destroy()?;
        Ok(confirmed)
    }
}

/// Owns zero or one live host. Created per Deskal launch scope.
pub struct Supervisor {
    live: Option<LiveHost>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self { live: None }
    }

    pub fn is_live(&mut self) -> bool {
        match self.live.as_mut() {
            Some(host) => host.is_alive(),
            None => false,
        }
    }

    /// Launch the discovered engine under a fresh ephemeral profile and
    /// handshake over the private channel. Refused while a host is live.
    /// `host_binary` is the Deskal-owned `qdral-browser-host` image;
    /// `state_root` is Deskal-owned state. Neither is caller-controlled in
    /// production: the MCP surface exposes no host controls in this grain.
    pub fn launch(
        &mut self,
        host_binary: &Path,
        state_root: &Path,
        search: &SearchConfig,
    ) -> Result<HostIdentity, HostError> {
        if self.is_live() {
            return Err(HostError::Invalid("a browser host is already live".into()));
        }
        self.live = None;
        let engine: Engine = discover_engine(search)?;
        if !host_binary.is_file() {
            return Err(HostError::Unavailable(UnavailableReason::LaunchFailed));
        }
        let profile = setup_ephemeral_profile(state_root, &engine.fingerprint())?;
        // Adopt-back proves the marker round-trips before any launch.
        let profile = adopt_profile(profile.dir(), &engine.fingerprint())?;
        let host_id = format!("host-{:016x}", unique_nonce());
        let args = vec![
            "--engine".to_string(),
            engine.path().to_string_lossy().into_owned(),
            "--profile-dir".to_string(),
            profile.dir().to_string_lossy().into_owned(),
            "--host-id".to_string(),
            host_id.clone(),
        ];
        let env = host_env(state_root);
        let (child, pipes) = spawn_host_piped(host_binary, &args, &env)?;
        let mut live = LiveHost {
            identity: HostIdentity {
                host_id: host_id.clone(),
                engine_fingerprint: engine.fingerprint(),
                profile_identity: profile.identity().to_owned(),
                pid: child.pid(),
            },
            profile,
            child,
            stdin: pipes.stdin,
            stdout: pipes.stdout,
        };
        handshake(&mut live, &host_id)?;
        let identity = live.identity().clone();
        self.live = Some(live);
        Ok(identity)
    }

    /// Shut the live host down deterministically, if any.
    pub fn shutdown(&mut self) -> Result<bool, HostError> {
        match self.live.take() {
            Some(host) => host.shutdown(),
            None => Ok(true),
        }
    }

    /// Round-trip ping through the live host's private channel, if any.
    pub fn ping(&mut self, nonce: &str) -> Result<(), HostError> {
        match self.live.as_mut() {
            Some(host) => host.ping(nonce),
            None => Err(HostError::Unavailable(UnavailableReason::LaunchFailed)),
        }
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn handshake(live: &mut LiveHost, host_id: &str) -> Result<(), HostError> {
    write_frame(
        &mut live.stdin,
        &HostRequest::Hello {
            generation: PROTOCOL_GENERATION,
        },
    )?;
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    loop {
        if Instant::now() >= deadline {
            return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation));
        }
        match decode_frame::<HostReply, _>(&mut live.stdout) {
            Ok(HostReply::Hello { generation, host }) => {
                if generation != PROTOCOL_GENERATION || host != host_id {
                    return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation));
                }
                return Ok(());
            }
            Ok(_) => continue,
            Err(_) => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
        }
    }
}

fn unique_nonce() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    now.wrapping_add((std::process::id() as u128) << 64) as u64
}

/// Scrubbed environment for the host binary: no secrets inherited.
fn host_env(state_root: &Path) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    let mut env = Vec::new();
    #[cfg(windows)]
    {
        let system_root = std::env::var_os("SystemRoot")
            .unwrap_or_else(|| std::ffi::OsString::from("C:\\Windows"));
        let system32 = Path::new(&system_root).join("System32");
        let system_temp = Path::new(&system_root).join("Temp");
        env.push((std::ffi::OsString::from("SystemRoot"), system_root.clone()));
        env.push((
            std::ffi::OsString::from("SystemDrive"),
            std::env::var_os("SystemDrive").unwrap_or_else(|| std::ffi::OsString::from("C:")),
        ));
        env.push((
            std::ffi::OsString::from("PATH"),
            std::ffi::OsString::from(system32.to_string_lossy().into_owned()),
        ));
        env.push((
            std::ffi::OsString::from("TEMP"),
            system_temp.clone().into_os_string(),
        ));
        env.push((
            std::ffi::OsString::from("TMP"),
            system_temp.into_os_string(),
        ));
        env.push((
            std::ffi::OsString::from("DESKAL_BROWSER_STATE"),
            state_root.as_os_str().to_owned(),
        ));
    }
    #[cfg(not(windows))]
    {
        env.push((
            std::ffi::OsString::from("PATH"),
            std::ffi::OsString::from("/usr/bin:/bin"),
        ));
        env.push((
            std::ffi::OsString::from("DESKAL_BROWSER_STATE"),
            state_root.as_os_str().to_owned(),
        ));
    }
    env
}
