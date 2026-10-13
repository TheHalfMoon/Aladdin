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
        let _ = write_frame(&mut stdin, &HostRequest::Shutdown {});
        let deadline = Instant::now() + Duration::from_secs(2);
        while child.is_alive() && Instant::now() < deadline {
            match decode_frame::<HostReply, _>(&mut stdout) {
                Ok(HostReply::Bye {}) => break,
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

/// Owns zero or one live host, of either mode. Created per Deskal launch
/// scope.
pub struct Supervisor {
    live: Option<LiveHost>,
    #[cfg(all(windows, feature = "live-browser-test"))]
    worker: Option<LiveWorkerHost>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            live: None,
            #[cfg(all(windows, feature = "live-browser-test"))]
            worker: None,
        }
    }

    pub fn is_live(&mut self) -> bool {
        let classic = match self.live.as_mut() {
            Some(host) => host.is_alive(),
            None => false,
        };
        #[cfg(all(windows, feature = "live-browser-test"))]
        let classic = classic
            || match self.worker.as_mut() {
                Some(host) => host.child.is_alive(),
                None => false,
            };
        classic
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

    /// Shut the live host down deterministically, if any, of either mode.
    pub fn shutdown(&mut self) -> Result<bool, HostError> {
        let classic = match self.live.take() {
            Some(host) => host.shutdown(),
            None => Ok(true),
        };
        #[cfg(all(windows, feature = "live-browser-test"))]
        let classic = {
            let worker = self.shutdown_worker_host();
            match (classic, worker) {
                (Ok(a), Ok(b)) => Ok(a && b),
                (Err(err), _) | (_, Err(err)) => Err(err),
            }
        };
        classic
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

/// SG-000108 product caller, slice 1a: a host in worker mode. It owns its
/// own profile (the host makes it, so the #298 sweep sees the right owner),
/// and serves the closed worker-mode vocabulary. Test builds only.
#[cfg(all(windows, feature = "live-browser-test"))]
pub struct LiveWorkerHost {
    host_id: String,
    child: SupervisedChild,
    stdin: std::process::ChildStdin,
    /// Frames read by a reader thread, so every wait has a deadline (a host
    /// that is alive but silent can never block the supervisor).
    replies: std::sync::mpsc::Receiver<Result<crate::worker_host::WorkerHostReply, HostError>>,
}

/// The longest a worker-mode reply may take: above the worker's own longest
/// operation bound (a download), so the host times out first and answers.
#[cfg(all(windows, feature = "live-browser-test"))]
pub const WORKER_HOST_REPLY_TIMEOUT: Duration = Duration::from_secs(150);

/// The hello and shutdown deadlines.
#[cfg(all(windows, feature = "live-browser-test"))]
const WORKER_HOST_CONTROL_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(all(windows, feature = "live-browser-test"))]
impl LiveWorkerHost {
    pub fn host_id(&self) -> &str {
        &self.host_id
    }

    pub fn pid(&self) -> u32 {
        self.child.pid()
    }

    /// One request, one reply, within `WORKER_HOST_REPLY_TIMEOUT`.
    pub fn request(
        &mut self,
        request: &crate::worker_host::WorkerHostRequest,
    ) -> Result<crate::worker_host::WorkerHostReply, HostError> {
        self.request_within(request, WORKER_HOST_REPLY_TIMEOUT)
    }

    /// One request, one reply, within `timeout`. A silent host is a typed
    /// timeout; a closed or malformed channel is a protocol violation.
    pub fn request_within(
        &mut self,
        request: &crate::worker_host::WorkerHostRequest,
        timeout: Duration,
    ) -> Result<crate::worker_host::WorkerHostReply, HostError> {
        write_frame(&mut self.stdin, request)?;
        match self.replies.recv_timeout(timeout) {
            Ok(reply) => reply,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                Err(HostError::Unavailable(UnavailableReason::TimedOut))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(HostError::Unavailable(UnavailableReason::ProtocolViolation))
            }
        }
    }

    /// Ask the host to stop (it ends any lifetime first), wait for its bye
    /// within a bounded time, then confirm the whole family is gone (the
    /// job is terminated if the host did not leave by itself).
    pub fn shutdown(mut self) -> Result<bool, HostError> {
        let _ = write_frame(
            &mut self.stdin,
            &crate::worker_host::WorkerHostRequest::Shutdown {},
        );
        let deadline = Instant::now() + WORKER_HOST_CONTROL_TIMEOUT;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.replies.recv_timeout(left) {
                Ok(Ok(crate::worker_host::WorkerHostReply::Bye {})) => break,
                Ok(Ok(_)) => continue,
                Ok(Err(_)) | Err(_) => break,
            }
        }
        let LiveWorkerHost { child, stdin, .. } = self;
        drop(stdin);
        child.shutdown()
    }
}

/// What Deskal passes a worker-mode host (never caller input).
#[cfg(all(windows, feature = "live-browser-test"))]
#[derive(Debug, Clone)]
pub struct WorkerHostLaunch {
    pub node: std::path::PathBuf,
    pub worker_main: std::path::PathBuf,
    pub profile_root: std::path::PathBuf,
    pub temp_dir: std::path::PathBuf,
    pub state_root: std::path::PathBuf,
}

#[cfg(all(windows, feature = "live-browser-test"))]
impl Supervisor {
    /// Launch `host_binary` in worker mode and complete the worker-mode
    /// hello, within a deadline. Refused while any host is live (one lifetime at a time).
    pub fn launch_worker_host(
        &mut self,
        host_binary: &Path,
        launch: &WorkerHostLaunch,
    ) -> Result<String, HostError> {
        if self.is_live() {
            return Err(HostError::Invalid("a browser host is already live".into()));
        }
        self.live = None;
        self.worker = None;
        if !host_binary.is_file() {
            return Err(HostError::Unavailable(UnavailableReason::LaunchFailed));
        }
        let host_id = format!("host-{:016x}", unique_nonce());
        let path = |value: &Path| value.to_string_lossy().into_owned();
        let args = vec![
            "--worker".to_string(),
            "--host-id".to_string(),
            host_id.clone(),
            "--node".to_string(),
            path(&launch.node),
            "--worker-main".to_string(),
            path(&launch.worker_main),
            "--profile-root".to_string(),
            path(&launch.profile_root),
            "--temp-dir".to_string(),
            path(&launch.temp_dir),
        ];
        let env = host_env(&launch.state_root);
        let (child, pipes) = spawn_host_piped(host_binary, &args, &env)?;
        let (sender, replies) = std::sync::mpsc::channel();
        let mut stdout = pipes.stdout;
        std::thread::spawn(move || loop {
            let frame = decode_frame::<crate::worker_host::WorkerHostReply, _>(&mut stdout);
            let stop = frame.is_err();
            if sender.send(frame).is_err() || stop {
                return;
            }
        });
        let mut host = LiveWorkerHost {
            host_id: host_id.clone(),
            child,
            stdin: pipes.stdin,
            replies,
        };
        let hello = host.request_within(
            &crate::worker_host::WorkerHostRequest::Hello {
                generation: crate::worker_host::WORKER_HOST_GENERATION,
            },
            WORKER_HOST_CONTROL_TIMEOUT,
        );
        let hello = match hello {
            Ok(reply) => reply,
            Err(err) => {
                let _ = host.shutdown();
                return Err(err);
            }
        };
        match hello {
            crate::worker_host::WorkerHostReply::Hello {
                generation,
                host: id,
            } if generation == crate::worker_host::WORKER_HOST_GENERATION && id == host_id => {}
            _ => {
                let _ = host.shutdown();
                return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation));
            }
        }
        self.worker = Some(host);
        Ok(host_id)
    }

    /// The live worker-mode host, if any.
    pub fn worker_host(&mut self) -> Option<&mut LiveWorkerHost> {
        self.worker.as_mut()
    }

    /// Shut the worker-mode host down deterministically, if any.
    pub fn shutdown_worker_host(&mut self) -> Result<bool, HostError> {
        match self.worker.take() {
            Some(host) => host.shutdown(),
            None => Ok(true),
        }
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
