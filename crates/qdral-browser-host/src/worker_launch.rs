//! SG-000108 host launcher for the private browser worker.
//!
//! The host owns everything that grants authority: it checks the managed
//! policy precondition, builds the worker's environment from an allowlist
//! (never inherited), builds the engine argv (`build_worker_argv`), starts
//! Node with exactly `--disable-sigusr1` and the worker entry point inside a
//! kill-on-close Job Object with restricted handle inheritance, drives the
//! closed worker protocol with bounded waits, and then verifies the job: the
//! only members are the worker and the verified engine, exactly one browser
//! process exists, and its OS command line equals the host-built argv.
//! Every failure is typed; supervision ending always terminates the job and
//! removes an ephemeral profile.

use crate::argv::FORBIDDEN_FLAGS;
#[cfg(windows)]
use crate::confinement::build_worker_argv;
use crate::confinement::{assert_no_managed_policy, AdmittedDestination, PolicySource};
use crate::discovery::Engine;
use crate::error::{HostError, UnavailableReason};
use crate::supervise::{CleanEnv, JOB_ACTIVE_PROCESS_LIMIT};
#[cfg(any(windows, test))]
use serde::{Deserialize, Serialize};
#[cfg(any(windows, test))]
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// One process inside the worker's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobProcess {
    pub pid: u32,
    pub image: PathBuf,
    /// The OS-reported command line split with `CommandLineToArgvW`.
    pub argv: Vec<String>,
}

/// Worker protocol generation (apps/qdral-browser-worker/src/protocol.ts).
/// Generation 2 adds the slice-3a navigation frames; generation 3 adds
/// slice-3b observation.
pub const WORKER_PROTOCOL_GENERATION: u32 = 3;
/// Worker identity reported in its hello.
pub const WORKER_IDENTITY: &str = "qdral-browser-worker";
/// The only Node option the worker accepts (inspector activation refused).
pub const WORKER_NODE_OPTION: &str = "--disable-sigusr1";

/// Bounded waits owned by the launcher.
pub const WORKER_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
pub const WORKER_LAUNCH_TIMEOUT: Duration = Duration::from_secs(60);
pub const WORKER_REPLY_TIMEOUT: Duration = Duration::from_secs(10);
pub const WORKER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound for one navigation, including every route decision.
pub const WORKER_NAVIGATION_TIMEOUT: Duration = Duration::from_secs(45);

/// Upper bound for one observation round trip.
pub const WORKER_OBSERVATION_TIMEOUT: Duration = Duration::from_secs(10);

/// One node of a worker snapshot: ref, role, accessible name and (links
/// only) the link URL. The worker never sends field values.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotNode {
    #[serde(rename = "ref")]
    pub reference: String,
    pub role: String,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    pub children: Vec<SnapshotNode>,
}

/// What the host keeps about an observed node for its own typed selection;
/// never returned to callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostNode {
    pub reference: String,
    pub role: String,
    pub name: String,
    pub url: Option<String>,
}

/// A live observation: SG-000076 identities and the redacted, digested
/// observation. Raw names stay inside the host.
#[derive(Debug, Clone)]
pub struct LiveObservation {
    pub page: crate::observation::PageIdentity,
    pub document: crate::observation::DocumentIdentity,
    pub observation: crate::observation::Observation,
}

/// A completed top-level navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationOutcome {
    pub url: String,
    pub status: u16,
}

/// Everything the launcher needs; every path must be absolute. `node` and
/// `worker_main` must come from Deskal's install records, never from a
/// caller; the engine can only be a [`Engine`] that discovery verified
/// (kind, version, publisher), so no unverified executable can be launched
/// under the confinement contract.
#[derive(Debug, Clone)]
pub struct WorkerLaunch {
    /// node.exe recorded at install (the worker itself refuses Node < 22.14).
    pub node: PathBuf,
    /// The worker entry point, `…/dist/main.js`.
    pub worker_main: PathBuf,
    /// The verified engine (from `discover_engine`).
    pub engine: Engine,
    /// The Deskal-owned profile directory.
    pub profile_dir: PathBuf,
    /// Remove the profile directory when supervision ends. Production uses
    /// a fresh ephemeral profile per lifetime; the launcher does not refuse
    /// `false` (the native crash test reuses a profile deliberately).
    pub ephemeral_profile: bool,
    /// TEMP/TMP for the worker and engine.
    pub temp_dir: PathBuf,
    /// Destinations admitted into this engine lifetime.
    pub destinations: Vec<AdmittedDestination>,
    /// Test builds only: a fixture certificate's SPKI hash the engine
    /// trusts. The worker accepts it only with loopback fixture pins.
    #[cfg(test)]
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) test_trusted_spki: Option<String>,
}

/// How supervision ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReport {
    /// The worker answered `bye` and exited with code 0.
    pub graceful: bool,
    /// No process remained in the job afterwards.
    pub job_empty: bool,
    /// The ephemeral profile was removed. Always true for a non-ephemeral
    /// profile, which the launcher never removes (nothing was to be removed);
    /// for an ephemeral one, true only once the directory no longer exists.
    pub profile_removed: bool,
}

#[cfg(any(windows, test))]
#[derive(Debug, Serialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub(crate) enum WorkerRequest<'a> {
    Hello {
        generation: u32,
    },
    Launch {
        engine: &'a str,
        profile_dir: &'a str,
        argv: &'a [String],
        env: BTreeMap<String, String>,
    },
    Ping {
        nonce: &'a str,
    },
    Navigate {
        nav_id: u64,
        url: &'a str,
    },
    RouteDecision {
        request_id: u64,
        allow: bool,
    },
    Observe {
        obs_id: u64,
    },
    Shutdown,
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum WorkerReply {
    Hello {
        generation: u32,
        worker: String,
    },
    // Empty struct variants: serde ignores unknown fields on unit
    // variants of an internally tagged enum even with deny_unknown_fields.
    Launched {},
    Pong {
        nonce: String,
    },
    Bye {},
    RouteRequest {
        request_id: u64,
        kind: String,
        url: String,
        origin: String,
    },
    Navigated {
        nav_id: u64,
        url: String,
        status: u16,
    },
    NavigationFailed {
        nav_id: u64,
        reason: String,
    },
    Observed {
        obs_id: u64,
        url: String,
        truncated: bool,
        nodes: Vec<SnapshotNode>,
    },
    ObservationFailed {
        obs_id: u64,
    },
    Error {
        code: String,
    },
}

/// Map a worker error code onto the typed vocabulary.
#[cfg(any(windows, test))]
pub(crate) fn reply_error(code: &str) -> HostError {
    match code {
        "launch_refused" => HostError::Invalid("the worker refused the launch contract".into()),
        "launch_failed" | "engine_exited" => {
            HostError::Unavailable(UnavailableReason::LaunchFailed)
        }
        _ => HostError::Unavailable(UnavailableReason::ProtocolViolation),
    }
}

/// Largest profile preferences file the host will rewrite.
#[cfg(any(windows, test))]
const MAX_PREFERENCES_BYTES: u64 = 8 * 1024 * 1024;

/// Turn off preloading in the profile before the engine starts. Speculation
/// rules prefetch and prerender run in the browser process, outside every
/// route and redirect guard (measured: an undecided request to another
/// admitted name, and a prerender whose activation moved the page). No
/// engine switch tested disables them; the profile's "Preload pages: never"
/// preference (`net.network_prediction_options` = 2) does. Existing
/// preferences are kept; an unreadable or oversized file fails closed.
#[cfg(any(windows, test))]
pub(crate) fn seed_profile_preferences(profile_dir: &Path) -> Result<(), HostError> {
    let dir = profile_dir.join("Default");
    std::fs::create_dir_all(&dir)
        .map_err(|err| HostError::Platform(format!("profile preferences: {err}")))?;
    let path = dir.join("Preferences");
    let mut value = match std::fs::metadata(&path) {
        Ok(meta) if meta.len() > MAX_PREFERENCES_BYTES => {
            return Err(HostError::Invalid(
                "profile preferences are oversized".into(),
            ))
        }
        Ok(_) => {
            let text = std::fs::read_to_string(&path)
                .map_err(|err| HostError::Platform(format!("profile preferences: {err}")))?;
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|_| HostError::Invalid("profile preferences are not JSON".into()))?
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => return Err(HostError::Platform(format!("profile preferences: {err}"))),
    };
    let root = value
        .as_object_mut()
        .ok_or_else(|| HostError::Invalid("profile preferences are not an object".into()))?;
    let net = root
        .entry("net")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| HostError::Invalid("profile `net` preferences are not an object".into()))?;
    net.insert("network_prediction_options".into(), serde_json::json!(2));
    let text = serde_json::to_string(&value)
        .map_err(|err| HostError::Invalid(format!("profile preferences: {err}")))?;
    std::fs::write(&path, text)
        .map_err(|err| HostError::Platform(format!("profile preferences: {err}")))
}

/// Test builds only: the fixture certificate trust flag.
#[cfg(test)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn test_certificate_flag(spki: &str) -> String {
    format!("--ignore-certificate-errors-spki-list={spki}")
}

/// Map a worker snapshot node onto an SG-000076 input node with a
/// host-issued identity, recording the raw name host-side. The value is
/// always empty: the worker never sends field values.
#[cfg(any(windows, test))]
pub(crate) fn input_node(
    node: &SnapshotNode,
    generation: u64,
    index: &mut std::collections::HashMap<String, HostNode>,
) -> crate::observation::InputNode {
    let node_id = format!("d{generation}-{}", node.reference);
    index.insert(
        node_id.clone(),
        HostNode {
            reference: node.reference.clone(),
            role: node.role.clone(),
            name: node.name.clone(),
            url: node.url.clone(),
        },
    );
    crate::observation::InputNode {
        identity: crate::observation::NodeIdentity {
            node_id,
            role: node.role.clone(),
            document_generation: generation,
        },
        role: node.role.clone(),
        name: node.name.clone(),
        value: String::new(),
        children: node
            .children
            .iter()
            .map(|child| input_node(child, generation, index))
            .collect(),
    }
}

/// Map a worker navigation failure onto the typed vocabulary.
#[cfg(any(windows, test))]
pub(crate) fn navigation_failure(reason: &str) -> HostError {
    match reason {
        "denied" => HostError::Invalid("navigation denied by the browser policy".into()),
        "timeout" | "failed" | "busy" => HostError::Unavailable(UnavailableReason::LaunchFailed),
        _ => HostError::Unavailable(UnavailableReason::ProtocolViolation),
    }
}

/// The worker's environment: an explicit allowlist, never inherited. Node
/// consumes variables such as NODE_OPTIONS before any worker code runs, so
/// nothing outside this list may reach it.
pub fn worker_environment(temp_dir: &Path) -> CleanEnv {
    let system_root =
        std::env::var_os("SystemRoot").unwrap_or_else(|| OsString::from("C:\\Windows"));
    let system_drive = std::env::var_os("SystemDrive").unwrap_or_else(|| OsString::from("C:"));
    let system32 = PathBuf::from(&system_root).join("System32");
    vec![
        (OsString::from("SystemRoot"), system_root),
        (OsString::from("SystemDrive"), system_drive),
        (OsString::from("PATH"), system32.into_os_string()),
        (OsString::from("TEMP"), temp_dir.as_os_str().to_owned()),
        (OsString::from("TMP"), temp_dir.as_os_str().to_owned()),
    ]
}

/// The worker's Node arguments: exactly the inspector guard and the entry.
pub fn worker_node_args(worker_main: &Path) -> Vec<String> {
    vec![
        WORKER_NODE_OPTION.to_string(),
        worker_main.to_string_lossy().into_owned(),
    ]
}

fn require_absolute(path: &Path, what: &str) -> Result<(), HostError> {
    if !path.is_absolute() {
        return Err(HostError::Invalid(format!(
            "{what} is not an absolute path"
        )));
    }
    Ok(())
}

fn file_name_is(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

/// Validate the launch request before anything is spawned. A missing Node
/// or worker is a missing prerequisite (typed unavailable).
pub(crate) fn validate_launch(spec: &WorkerLaunch) -> Result<(), HostError> {
    // Exactly one destination per engine lifetime. The browser process
    // issues some requests outside every route and guard (measured:
    // fetchLater, Reporting/CSP reports, FedCM, audio worklet modules,
    // NoStatePrefetch). With a single admitted name, the pinned resolver and
    // the no-route proxy make every other host unreachable, so such requests
    // can reach only the page's own origin, which SG-000075 permits. The
    // policy is exact-origin anyway; another site is another lifetime.
    if spec.destinations.len() != 1 {
        return Err(HostError::Invalid(
            "a browser lifetime admits exactly one destination".into(),
        ));
    }
    for (path, what) in [
        (spec.node.as_path(), "node"),
        (&spec.worker_main, "worker entry point"),
        (spec.engine.path(), "engine"),
        (&spec.profile_dir, "profile directory"),
        (&spec.temp_dir, "temp directory"),
    ] {
        require_absolute(path, what)?;
    }
    if !file_name_is(&spec.node, "node.exe") || !spec.node.is_file() {
        return Err(HostError::Unavailable(
            UnavailableReason::UnsupportedRuntime,
        ));
    }
    if !file_name_is(&spec.worker_main, "main.js") || !spec.worker_main.is_file() {
        return Err(HostError::Unavailable(
            UnavailableReason::UnsupportedRuntime,
        ));
    }
    if !spec.engine.path().is_file() {
        return Err(HostError::Unavailable(UnavailableReason::NoEngine));
    }
    if !spec.profile_dir.is_dir() || !spec.temp_dir.is_dir() {
        return Err(HostError::Invalid(
            "profile or temp directory does not exist".into(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn path_text(path: &Path, what: &str) -> Result<String, HostError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| HostError::Invalid(format!("{what} is not UTF-8")))
}

/// The switch name of a command-line argument as Chromium on Windows reads
/// it: a `--`, `-` or `/` prefix, the name lowercased, any `=value` dropped.
fn switch_name(arg: &str) -> Option<String> {
    let name = arg
        .strip_prefix("--")
        .or_else(|| arg.strip_prefix('-'))
        .or_else(|| arg.strip_prefix('/'))?;
    let name = name.split('=').next().unwrap_or(name);
    Some(name.to_ascii_lowercase())
}

/// The process type Chromium derives from a command line: switches end at a
/// bare `--` or at `--single-argument` (after which Chromium on Windows reads
/// the rest as one plain argument; matched here in any spelling, which can
/// only turn a process into a browser and so fails closed), the last `type`
/// switch in any accepted spelling wins, and an absent or empty value means
/// the browser process (`None`).
fn chromium_process_type(argv: &[String]) -> Option<String> {
    let mut kind = None;
    for arg in argv.iter().skip(1) {
        if arg == "--" || switch_name(arg).as_deref() == Some("single-argument") {
            break;
        }
        if switch_name(arg).as_deref() == Some("type") {
            kind = arg
                .split_once('=')
                .map(|(_, value)| value.to_owned())
                .filter(|value| !value.is_empty());
        }
    }
    kind
}

/// Verify the job against the host-built launch: the worker runs with
/// exactly its host-built arguments, every other member is the verified
/// engine, no member carries a forbidden flag (in any spelling Chromium
/// accepts), exactly one browser process (an engine process without
/// `--type=`) exists, and its OS command line equals
/// `[engine, …expected_argv]`. Returns its pid.
pub fn classify_job(
    processes: &[JobProcess],
    worker_pid: u32,
    node: &Path,
    worker_args: &[String],
    engine: &Path,
    expected_argv: &[String],
) -> Result<u32, HostError> {
    if processes.len() > JOB_ACTIVE_PROCESS_LIMIT as usize {
        return Err(HostError::Invalid(
            "browser job exceeds its process limit".into(),
        ));
    }
    let same = |a: &Path, b: &Path| {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    };
    let Some(worker) = processes
        .iter()
        .find(|process| process.pid == worker_pid && same(&process.image, node))
    else {
        return Err(HostError::Invalid(
            "the worker is not a member of its job".into(),
        ));
    };
    if worker.argv.get(1..) != Some(worker_args) {
        return Err(HostError::Invalid(
            "the worker command line differs from the host-built one".into(),
        ));
    }
    let forbidden: Vec<String> = FORBIDDEN_FLAGS
        .iter()
        .filter_map(|flag| switch_name(flag))
        .collect();
    let mut browsers = Vec::new();
    for process in processes {
        if process.pid == worker_pid {
            continue;
        }
        if !same(&process.image, engine) {
            return Err(HostError::Invalid(format!(
                "a foreign process ({}) is in the browser job",
                process
                    .image
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            )));
        }
        for arg in process.argv.iter().skip(1) {
            if let Some(name) = switch_name(arg).filter(|name| forbidden.contains(name)) {
                return Err(HostError::Invalid(format!(
                    "an engine process carries the forbidden flag --{name}"
                )));
            }
        }
        if chromium_process_type(&process.argv).is_none() {
            browsers.push(process);
        }
    }
    let [browser] = browsers.as_slice() else {
        return Err(HostError::Invalid(format!(
            "expected exactly one browser process in the job, found {}",
            browsers.len()
        )));
    };
    let (program, args) = browser
        .argv
        .split_first()
        .ok_or_else(|| HostError::Invalid("browser command line is empty".into()))?;
    if !same(Path::new(program), engine) || args != expected_argv {
        return Err(HostError::Invalid(
            "the browser command line differs from the host-built argv".into(),
        ));
    }
    Ok(browser.pid)
}

/// Remove a profile directory, retrying for up to 30 s. After every engine
/// process has exited, the kernel can still hold memory-mapped profile files
/// briefly (Restart Manager reports the holder as System, pid 4); under load
/// that release has been observed to exceed 10 s. A profile still held after
/// the budget is reported as not removed, never as removed.
#[cfg(windows)]
pub(crate) fn remove_profile(path: &Path) -> bool {
    let mut last = None;
    for _ in 0..120 {
        match std::fs::remove_dir_all(path) {
            Ok(()) => return true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
            Err(error) => {
                last = Some(error);
                std::thread::sleep(Duration::from_millis(250));
            }
        }
    }
    #[cfg(test)]
    eprintln!("SG108-DIAG remove_profile failed: {last:?}");
    let _ = last;
    !path.exists()
}

#[cfg(windows)]
pub use platform::{launch_worker, BrowserWorker};

#[cfg(not(windows))]
pub use fallback::{launch_worker, BrowserWorker};

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::proto::{decode_frame, write_frame};
    use crate::worker_navigation::{NavigationPolicy, RouteKind};
    use crate::worker_process::{spawn, WorkerProcess};
    use std::fs::File;
    use std::io::BufReader;
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
    use std::time::Instant;

    /// A supervised browser worker. Dropping it terminates the job and
    /// removes an ephemeral profile.
    pub struct BrowserWorker {
        pub(super) process: WorkerProcess,
        stdin: File,
        replies: Receiver<Result<WorkerReply, HostError>>,
        node: PathBuf,
        worker_args: Vec<String>,
        engine: PathBuf,
        argv: Vec<String>,
        profile: Option<PathBuf>,
        browser_pid: u32,
        finished: bool,
        policy: NavigationPolicy,
        next_nav: u64,
        last_request: u64,
        next_obs: u64,
        /// The committed document: URL and document generation.
        committed: Option<(String, u64)>,
        /// Raw names of the last observation, by host-issued node id.
        host_nodes: std::collections::HashMap<String, HostNode>,
    }

    /// Launch the worker and its engine, or fail closed with a typed error
    /// and nothing left running.
    pub fn launch_worker(
        spec: &WorkerLaunch,
        policy: &impl PolicySource,
    ) -> Result<BrowserWorker, HostError> {
        validate_launch(spec)?;
        assert_no_managed_policy(spec.engine.kind(), policy)
            .map_err(|_| HostError::Unavailable(UnavailableReason::ManagedPolicy))?;
        #[allow(unused_mut)] // mutated in test builds only
        let mut argv = build_worker_argv(&spec.profile_dir, &spec.destinations)?;
        #[cfg(test)]
        if let Some(spki) = &spec.test_trusted_spki {
            // Before the trailing blank page, where the worker expects it.
            argv.insert(argv.len() - 1, test_certificate_flag(spki));
        }
        seed_profile_preferences(&spec.profile_dir)?;
        let env = worker_environment(&spec.temp_dir);
        let worker_args = worker_node_args(&spec.worker_main);
        let (process, pipes) = spawn(&spec.node, &worker_args, &env)?;
        let (sender, replies) = mpsc::channel();
        let stdout = pipes.stdout;
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let frame = decode_frame::<WorkerReply, _>(&mut reader);
                let stop = frame.is_err();
                if sender.send(frame).is_err() || stop {
                    return;
                }
            }
        });
        let mut worker = BrowserWorker {
            process,
            stdin: pipes.stdin,
            replies,
            node: spec.node.clone(),
            worker_args,
            engine: spec.engine.path().to_path_buf(),
            argv,
            profile: spec.ephemeral_profile.then(|| spec.profile_dir.clone()),
            browser_pid: 0,
            finished: false,
            policy: NavigationPolicy::new(&spec.destinations),
            next_obs: 0,
            committed: None,
            host_nodes: std::collections::HashMap::new(),
            next_nav: 0,
            last_request: 0,
        };
        worker.handshake(spec)?;
        Ok(worker)
    }

    impl BrowserWorker {
        fn send(&mut self, request: &WorkerRequest<'_>) -> Result<(), HostError> {
            write_frame(&mut self.stdin, request)
                .map_err(|_| HostError::Unavailable(UnavailableReason::ProtocolViolation))
        }

        /// The next reply within `timeout`. A worker that exits with code 2
        /// before answering refused its runtime (typed unsupported runtime).
        fn reply(&mut self, timeout: Duration) -> Result<WorkerReply, HostError> {
            let deadline = Instant::now() + timeout;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                // A worker that keeps sending route requests cannot hold the
                // host past the deadline.
                if remaining.is_zero() {
                    return Err(HostError::Unavailable(UnavailableReason::LaunchFailed));
                }
                match self.next_frame(remaining)? {
                    WorkerReply::RouteRequest {
                        request_id,
                        kind,
                        url,
                        origin,
                    } => self.answer_route(request_id, &kind, &url, &origin)?,
                    reply => return Ok(reply),
                }
            }
        }

        /// Decide one forwarded engine request with the SG-000075 policy.
        /// Request ids must increase strictly; an unknown kind is refused.
        fn answer_route(
            &mut self,
            request_id: u64,
            kind: &str,
            url: &str,
            origin: &str,
        ) -> Result<(), HostError> {
            if request_id <= self.last_request {
                return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation));
            }
            self.last_request = request_id;
            let allow = match RouteKind::parse(kind) {
                Some(kind) => self.policy.decide(kind, url, origin).is_ok(),
                None => false,
            };
            self.send(&WorkerRequest::RouteDecision { request_id, allow })
        }

        fn next_frame(&mut self, timeout: Duration) -> Result<WorkerReply, HostError> {
            match self.replies.recv_timeout(timeout) {
                Ok(Ok(WorkerReply::Error { code })) => Err(reply_error(&code)),
                Ok(Ok(reply)) => Ok(reply),
                Ok(Err(_)) | Err(RecvTimeoutError::Disconnected) => {
                    match self.process.wait_exit(Duration::from_secs(2)) {
                        Some(2) => Err(HostError::Unavailable(
                            UnavailableReason::UnsupportedRuntime,
                        )),
                        _ => Err(HostError::Unavailable(UnavailableReason::ProtocolViolation)),
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    Err(HostError::Unavailable(UnavailableReason::LaunchFailed))
                }
            }
        }

        fn handshake(&mut self, spec: &WorkerLaunch) -> Result<(), HostError> {
            self.send(&WorkerRequest::Hello {
                generation: WORKER_PROTOCOL_GENERATION,
            })?;
            match self.reply(WORKER_HANDSHAKE_TIMEOUT)? {
                WorkerReply::Hello { generation, worker }
                    if generation == WORKER_PROTOCOL_GENERATION && worker == WORKER_IDENTITY => {}
                _ => return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation)),
            }
            let env: BTreeMap<String, String> = worker_environment(&spec.temp_dir)
                .into_iter()
                .map(|(key, value)| {
                    (
                        key.to_string_lossy().into_owned(),
                        value.to_string_lossy().into_owned(),
                    )
                })
                .collect();
            let engine = path_text(spec.engine.path(), "engine")?;
            let profile_dir = path_text(&spec.profile_dir, "profile directory")?;
            let argv = self.argv.clone();
            self.send(&WorkerRequest::Launch {
                engine: &engine,
                profile_dir: &profile_dir,
                argv: &argv,
                env,
            })?;
            match self.reply(WORKER_LAUNCH_TIMEOUT)? {
                WorkerReply::Launched {} => {}
                _ => return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation)),
            }
            self.browser_pid = self.verify()?;
            Ok(())
        }

        /// Re-verify the job membership and the browser command line.
        pub fn verify(&self) -> Result<u32, HostError> {
            let processes = self.process.snapshot()?;
            classify_job(
                &processes,
                self.process.pid(),
                &self.node,
                &self.worker_args,
                &self.engine,
                &self.argv,
            )
        }

        /// The current job members.
        pub fn job_processes(&self) -> Result<Vec<JobProcess>, HostError> {
            self.process.snapshot()
        }

        pub fn worker_pid(&self) -> u32 {
            self.process.pid()
        }

        pub fn browser_pid(&self) -> u32 {
            self.browser_pid
        }

        /// Liveness check over the closed protocol.
        pub fn ping(&mut self, nonce: &str) -> Result<(), HostError> {
            self.send(&WorkerRequest::Ping { nonce })?;
            match self.reply(WORKER_REPLY_TIMEOUT)? {
                WorkerReply::Pong { nonce: echoed } if echoed == nonce => Ok(()),
                _ => Err(HostError::Unavailable(UnavailableReason::ProtocolViolation)),
            }
        }

        /// Navigate the primary page to an admitted HTTPS URL. Every request
        /// the engine makes meanwhile is decided here by the SG-000075
        /// policy; the outcome is typed.
        pub fn navigate(&mut self, url: &str) -> Result<NavigationOutcome, HostError> {
            let outcome = self.navigate_inner(url);
            if outcome.is_err() {
                self.policy.abort();
            }
            outcome
        }

        fn navigate_inner(&mut self, url: &str) -> Result<NavigationOutcome, HostError> {
            self.policy.begin(url)?;
            self.next_nav += 1;
            let nav_id = self.next_nav;
            self.send(&WorkerRequest::Navigate { nav_id, url })?;
            match self.reply(WORKER_NAVIGATION_TIMEOUT)? {
                WorkerReply::Navigated {
                    nav_id: done,
                    url: committed,
                    status,
                } if done == nav_id => {
                    self.policy.commit(&committed)?;
                    // A new document: earlier node identities are stale.
                    self.committed = Some((committed.clone(), nav_id));
                    self.host_nodes.clear();
                    Ok(NavigationOutcome {
                        url: committed,
                        status,
                    })
                }
                WorkerReply::NavigationFailed {
                    nav_id: done,
                    reason,
                } if done == nav_id => {
                    #[cfg(test)]
                    eprintln!("SG108-DIAG navigation {url} failed: {reason}");
                    Err(navigation_failure(&reason))
                }
                other => {
                    #[cfg(test)]
                    eprintln!("SG108-DIAG navigation {url} unexpected reply: {other:?}");
                    let _ = other;
                    Err(HostError::Unavailable(UnavailableReason::ProtocolViolation))
                }
            }
        }

        /// Observe the committed document (SG-000076). The snapshot must be
        /// of exactly the committed URL (a page that moved itself is stale
        /// and needs a new navigation). Node identities are issued here,
        /// bound to the document generation; callers get only the
        /// redacted, digested observation.
        pub fn observe(
            &mut self,
            scope: &crate::observation::ObservationScope,
        ) -> Result<LiveObservation, HostError> {
            let (url, generation) = self
                .committed
                .clone()
                .ok_or_else(|| HostError::Invalid("no committed document to observe".into()))?;
            let origin = self
                .policy
                .committed_origin()
                .ok_or_else(|| HostError::Invalid("no committed origin".into()))?
                .to_owned();
            self.next_obs += 1;
            let obs_id = self.next_obs;
            let started = Instant::now();
            self.send(&WorkerRequest::Observe { obs_id })?;
            let (observed_url, nodes) = match self.reply(WORKER_OBSERVATION_TIMEOUT)? {
                WorkerReply::Observed {
                    obs_id: done,
                    url: observed_url,
                    truncated: _,
                    nodes,
                } if done == obs_id => (observed_url, nodes),
                WorkerReply::ObservationFailed { obs_id: done } if done == obs_id => {
                    return Err(HostError::Unavailable(UnavailableReason::LaunchFailed))
                }
                _ => return Err(HostError::Unavailable(UnavailableReason::ProtocolViolation)),
            };
            if observed_url != url {
                return Err(HostError::Invalid(
                    "stale page: the document changed".into(),
                ));
            }
            let elapsed = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            let page = crate::observation::PageIdentity {
                page_id: format!("p{generation}"),
                origin,
                page_generation: generation,
            };
            let document = crate::observation::DocumentIdentity {
                page_id: page.page_id.clone(),
                document_generation: generation,
            };
            let mut index = std::collections::HashMap::new();
            let roots: Vec<crate::observation::InputNode> = nodes
                .iter()
                .map(|node| input_node(node, generation, &mut index))
                .collect();
            let observation =
                crate::observation::observe_snapshot(scope, &page, &document, &roots, elapsed)?;
            self.host_nodes = index;
            Ok(LiveObservation {
                page,
                document,
                observation,
            })
        }

        /// Host-only view of an observed node (for typed selection).
        pub fn host_node(&self, node_id: &str) -> Option<&HostNode> {
            self.host_nodes.get(node_id)
        }

        /// Graceful shutdown with a forced fallback: ask the worker to stop,
        /// wait for `bye` and its exit, then terminate the job regardless and
        /// confirm it is empty, then remove an ephemeral profile.
        pub fn shutdown(mut self) -> ShutdownReport {
            let bye = self.send(&WorkerRequest::Shutdown).is_ok()
                && matches!(self.reply(WORKER_SHUTDOWN_TIMEOUT), Ok(WorkerReply::Bye {}));
            let exited = self.process.wait_exit(WORKER_SHUTDOWN_TIMEOUT);
            self.finish(bye && exited == Some(0))
        }

        /// Forced termination (cancellation): no protocol exchange.
        pub fn kill(mut self) -> ShutdownReport {
            self.finish(false)
        }

        fn finish(&mut self, graceful: bool) -> ShutdownReport {
            self.finished = true;
            let job_empty = self.process.terminate_and_confirm(WORKER_SHUTDOWN_TIMEOUT);
            let profile_removed = match self.profile.take() {
                Some(profile) => job_empty && remove_profile(&profile),
                None => true,
            };
            ShutdownReport {
                graceful,
                job_empty,
                profile_removed,
            }
        }
    }

    impl Drop for BrowserWorker {
        fn drop(&mut self) {
            if !self.finished {
                self.finish(false);
            }
        }
    }
}

#[cfg(not(windows))]
mod fallback {
    use super::*;

    /// The confinement design is Windows-only (the no-route proxy relies on
    /// Windows refusing connections to 0.0.0.0); elsewhere nothing launches.
    pub struct BrowserWorker {
        _private: (),
    }

    impl BrowserWorker {
        pub fn shutdown(self) -> ShutdownReport {
            ShutdownReport {
                graceful: false,
                job_empty: true,
                profile_removed: true,
            }
        }

        /// API parity: no worker ever exists off Windows.
        pub fn navigate(&mut self, _url: &str) -> Result<NavigationOutcome, HostError> {
            Err(HostError::Unavailable(
                UnavailableReason::UnsupportedRuntime,
            ))
        }

        /// API parity: no worker ever exists off Windows.
        pub fn observe(
            &mut self,
            _scope: &crate::observation::ObservationScope,
        ) -> Result<LiveObservation, HostError> {
            Err(HostError::Unavailable(
                UnavailableReason::UnsupportedRuntime,
            ))
        }

        /// API parity: no worker ever exists off Windows.
        pub fn host_node(&self, _node_id: &str) -> Option<&HostNode> {
            None
        }
    }

    pub fn launch_worker(
        spec: &WorkerLaunch,
        policy: &impl PolicySource,
    ) -> Result<BrowserWorker, HostError> {
        validate_launch(spec)?;
        assert_no_managed_policy(spec.engine.kind(), policy)
            .map_err(|_| HostError::Unavailable(UnavailableReason::ManagedPolicy))?;
        Err(HostError::Unavailable(
            UnavailableReason::UnsupportedRuntime,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, image: &str, argv: &[&str]) -> JobProcess {
        JobProcess {
            pid,
            image: PathBuf::from(image),
            argv: argv.iter().map(|arg| arg.to_string()).collect(),
        }
    }

    const NODE: &str = "C:\\Program Files\\nodejs\\node.exe";
    const EDGE: &str = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";

    fn expected() -> Vec<String> {
        vec![
            "--headless".into(),
            "--user-data-dir=C:\\p".into(),
            "about:blank".into(),
        ]
    }

    fn worker_args() -> Vec<String> {
        vec!["--disable-sigusr1".into(), "C:\\w\\main.js".into()]
    }

    fn healthy() -> Vec<JobProcess> {
        vec![
            process(10, NODE, &[NODE, "--disable-sigusr1", "C:\\w\\main.js"]),
            process(
                11,
                EDGE,
                &[EDGE, "--headless", "--user-data-dir=C:\\p", "about:blank"],
            ),
            process(
                12,
                EDGE,
                &[EDGE, "--type=renderer", "--user-data-dir=C:\\p"],
            ),
            process(13, EDGE, &[EDGE, "--type=gpu-process"]),
        ]
    }

    fn classify(processes: &[JobProcess]) -> Result<u32, HostError> {
        classify_job(
            processes,
            10,
            Path::new(NODE),
            &worker_args(),
            Path::new(EDGE),
            &expected(),
        )
    }

    #[test]
    fn a_healthy_job_yields_the_browser_pid() {
        assert_eq!(classify(&healthy()).unwrap(), 11);
        // Image paths compare case-insensitively.
        let mut upper = healthy();
        upper[1].image = PathBuf::from(EDGE.to_uppercase());
        assert_eq!(classify(&upper).unwrap(), 11);
        // A helper whose last `type` switch is non-empty stays a helper,
        // whatever its spelling.
        let mut helpers = healthy();
        helpers.push(process(18, EDGE, &[EDGE, "/Type=utility"]));
        helpers.push(process(19, EDGE, &[EDGE, "--type=", "--type=gpu-process"]));
        assert_eq!(classify(&helpers).unwrap(), 11);
    }

    #[test]
    fn every_unexpected_job_shape_is_refused() {
        let mut cases: Vec<Vec<JobProcess>> = Vec::new();
        // A second browser process (no --type=).
        let mut second = healthy();
        second.push(process(14, EDGE, &[EDGE, "--headless", "about:blank"]));
        cases.push(second);
        // No browser process at all.
        cases.push(healthy().into_iter().filter(|p| p.pid != 11).collect());
        // A foreign process in the job.
        let mut foreign = healthy();
        foreign.push(process(
            15,
            "C:\\Windows\\System32\\PING.EXE",
            &["PING.EXE"],
        ));
        cases.push(foreign);
        // The worker is not a member, or another node is.
        cases.push(healthy().into_iter().filter(|p| p.pid != 10).collect());
        let mut other_node = healthy();
        other_node.push(process(16, NODE, &[NODE, "evil.js"]));
        cases.push(other_node);
        // Browser argv drift: an added, removed or changed argument.
        for argv in [
            vec![
                EDGE,
                "--headless",
                "--user-data-dir=C:\\p",
                "--enable-automation",
                "about:blank",
            ],
            vec![EDGE, "--headless", "about:blank"],
            vec![EDGE, "--headless", "--user-data-dir=C:\\q", "about:blank"],
            vec![
                "C:\\other\\msedge.exe",
                "--headless",
                "--user-data-dir=C:\\p",
                "about:blank",
            ],
        ] {
            let mut drift = healthy();
            drift[1] = process(11, EDGE, &argv);
            cases.push(drift);
        }
        // A forbidden flag on a child process.
        let mut child = healthy();
        child[2] = process(
            12,
            EDGE,
            &[EDGE, "--type=renderer", "--remote-debugging-port=9222"],
        );
        cases.push(child);
        // The same flag in every other spelling Chromium accepts on Windows.
        for flag in [
            "-remote-debugging-port=9222",
            "/remote-debugging-port=9222",
            "--Remote-Debugging-Port=9222",
            "--NO-SANDBOX",
        ] {
            let mut child = healthy();
            child[2] = process(12, EDGE, &[EDGE, "--type=renderer", flag]);
            cases.push(child);
        }
        // A second browser disguised as a helper: Chromium takes the last
        // `type` switch, treats an empty value as the browser, accepts any
        // switch spelling, and stops reading switches at a bare `--`.
        for argv in [
            vec![EDGE, "--type=renderer", "--type=", "--user-data-dir=C:\\q"],
            vec![EDGE, "--type=renderer", "--type", "--user-data-dir=C:\\q"],
            vec![EDGE, "--type=", "--user-data-dir=C:\\q"],
            vec![EDGE, "--", "--type=renderer", "--user-data-dir=C:\\q"],
            vec![EDGE, "-type=renderer", "-type=", "--user-data-dir=C:\\q"],
            vec![
                EDGE,
                "--single-argument",
                "--type=renderer",
                "--user-data-dir=C:\\q",
            ],
            vec![EDGE, "/Single-Argument", "--type=renderer"],
        ] {
            let mut disguised = healthy();
            disguised.push(process(17, EDGE, &argv));
            cases.push(disguised);
        }
        // The worker with any arguments other than the host-built ones.
        for argv in [
            vec![NODE, "--disable-sigusr1", "C:\\w\\main.js", "--inspect"],
            vec![NODE, "--inspect", "C:\\w\\main.js"],
            vec![NODE, "--disable-sigusr1", "C:\\w\\other.js"],
            vec![NODE],
        ] {
            let mut drift = healthy();
            drift[0] = process(10, NODE, &argv);
            cases.push(drift);
        }
        // Over the job limit.
        let mut many = healthy();
        for pid in 100..100 + JOB_ACTIVE_PROCESS_LIMIT {
            many.push(process(pid, EDGE, &[EDGE, "--type=utility"]));
        }
        cases.push(many);
        for (index, case) in cases.iter().enumerate() {
            assert!(classify(case).is_err(), "case {index} was accepted");
        }
    }

    #[test]
    fn the_worker_environment_is_exactly_the_allowlist() {
        let env = worker_environment(Path::new("C:\\deskal\\tmp"));
        let keys: Vec<String> = env
            .iter()
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .collect();
        assert_eq!(keys, ["SystemRoot", "SystemDrive", "PATH", "TEMP", "TMP"]);
        let value = |key: &str| {
            env.iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        assert!(value("PATH").to_ascii_lowercase().ends_with("system32"));
        assert_eq!(value("TEMP"), "C:\\deskal\\tmp");
        assert_eq!(
            worker_node_args(Path::new("C:\\w\\main.js")),
            ["--disable-sigusr1", "C:\\w\\main.js"]
        );
    }

    #[test]
    fn profile_preferences_turn_preloading_off_and_keep_the_rest() {
        let dir = std::env::temp_dir().join(format!(
            "qdral-sg108-prefs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let read = |dir: &Path| -> serde_json::Value {
            serde_json::from_str(
                &std::fs::read_to_string(dir.join("Default").join("Preferences")).unwrap(),
            )
            .unwrap()
        };
        // A fresh profile gets the preference.
        seed_profile_preferences(&dir).unwrap();
        assert_eq!(read(&dir)["net"]["network_prediction_options"], 2);
        // Existing preferences are kept and the value is forced.
        std::fs::write(
            dir.join("Default").join("Preferences"),
            r#"{"keep":1,"net":{"network_prediction_options":0,"other":true}}"#,
        )
        .unwrap();
        seed_profile_preferences(&dir).unwrap();
        let value = read(&dir);
        assert_eq!(value["keep"], 1);
        assert_eq!(value["net"]["other"], true);
        assert_eq!(value["net"]["network_prediction_options"], 2);
        // Malformed preferences fail closed.
        for bad in ["not json", "[1]", r#"{"net":5}"#] {
            std::fs::write(dir.join("Default").join("Preferences"), bad).unwrap();
            assert!(seed_profile_preferences(&dir).is_err(), "{bad}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn observation_frames_and_identities_are_exact() {
        let json = |request: &WorkerRequest<'_>| serde_json::to_string(request).unwrap();
        assert_eq!(
            json(&WorkerRequest::Observe { obs_id: 4 }),
            r#"{"frame":"observe","obs_id":4}"#
        );
        let reply = |text: &str| serde_json::from_str::<WorkerReply>(text);
        let observed = reply(
            r#"{"frame":"observed","obs_id":4,"url":"https://fixture.test/r","truncated":false,"nodes":[{"ref":"e1","role":"list","name":"","children":[{"ref":"e2","role":"link","name":"Report 2026-09-30","url":"/r.pdf","children":[]}]}]}"#,
        )
        .unwrap();
        let WorkerReply::Observed { nodes, .. } = observed else {
            panic!("not an observation")
        };
        // Unknown fields (for example a field value) are refused anywhere.
        assert!(reply(r#"{"frame":"observed","obs_id":4,"url":"u","truncated":false,"nodes":[{"ref":"e1","role":"textbox","name":"","value":"secret","children":[]}]}"#).is_err());
        assert!(reply(r#"{"frame":"observation_failed","obs_id":4,"x":1}"#).is_err());
        // Identities are host-issued and bound to the document generation;
        // the raw name stays in the host index; values are always empty.
        let mut index = std::collections::HashMap::new();
        let roots: Vec<_> = nodes.iter().map(|n| input_node(n, 7, &mut index)).collect();
        assert_eq!(roots[0].identity.node_id, "d7-e1");
        assert_eq!(roots[0].children[0].identity.node_id, "d7-e2");
        assert_eq!(roots[0].children[0].identity.document_generation, 7);
        assert!(roots[0].children[0].value.is_empty());
        let link = &index["d7-e2"];
        assert_eq!(link.name, "Report 2026-09-30");
        assert_eq!(link.url.as_deref(), Some("/r.pdf"));
        assert_eq!(WORKER_OBSERVATION_TIMEOUT, Duration::from_secs(10));
        // The SG-000076 pipeline digests names and checks generations.
        let scope = crate::observation::ObservationScope {
            workspace: "ws".into(),
            profile_identity: "profile".into(),
        };
        let page = crate::observation::PageIdentity {
            page_id: "p7".into(),
            origin: "https://fixture.test".into(),
            page_generation: 7,
        };
        let document = crate::observation::DocumentIdentity {
            page_id: "p7".into(),
            document_generation: 7,
        };
        let observation =
            crate::observation::observe_snapshot(&scope, &page, &document, &roots, 5).unwrap();
        let text = format!("{observation:?}");
        assert!(
            !text.contains("Report 2026-09-30"),
            "a raw name reached the observation"
        );
        let stale = crate::observation::DocumentIdentity {
            page_id: "p7".into(),
            document_generation: 8,
        };
        assert!(crate::observation::observe_snapshot(&scope, &page, &stale, &roots, 5).is_err());
    }

    #[test]
    fn navigation_frames_match_the_typescript_vocabulary() {
        let json = |request: &WorkerRequest<'_>| serde_json::to_string(request).unwrap();
        assert_eq!(
            json(&WorkerRequest::Navigate {
                nav_id: 3,
                url: "https://fixture.test/a"
            }),
            r#"{"frame":"navigate","nav_id":3,"url":"https://fixture.test/a"}"#
        );
        assert_eq!(
            json(&WorkerRequest::RouteDecision {
                request_id: 9,
                allow: false
            }),
            r#"{"frame":"route_decision","request_id":9,"allow":false}"#
        );
        let reply = |text: &str| serde_json::from_str::<WorkerReply>(text);
        assert_eq!(
            reply(r#"{"frame":"route_request","request_id":1,"kind":"frame","url":"https://x.test/","origin":"https://fixture.test"}"#).unwrap(),
            WorkerReply::RouteRequest {
                request_id: 1,
                kind: "frame".into(),
                url: "https://x.test/".into(),
                origin: "https://fixture.test".into()
            }
        );
        assert_eq!(
            reply(
                r#"{"frame":"navigated","nav_id":3,"url":"https://fixture.test/a","status":200}"#
            )
            .unwrap(),
            WorkerReply::Navigated {
                nav_id: 3,
                url: "https://fixture.test/a".into(),
                status: 200
            }
        );
        assert_eq!(
            reply(r#"{"frame":"navigation_failed","nav_id":3,"reason":"denied"}"#).unwrap(),
            WorkerReply::NavigationFailed {
                nav_id: 3,
                reason: "denied".into()
            }
        );
        // Unknown fields, negative ids and out-of-range statuses are refused.
        assert!(reply(r#"{"frame":"navigated","nav_id":3,"url":"u","status":200,"x":1}"#).is_err());
        assert!(reply(
            r#"{"frame":"route_request","request_id":-1,"kind":"frame","url":"u","origin":""}"#
        )
        .is_err());
        assert!(reply(r#"{"frame":"navigated","nav_id":3,"url":"u","status":70000}"#).is_err());
        // Failure reasons map onto the typed vocabulary.
        assert!(matches!(
            navigation_failure("denied"),
            HostError::Invalid(_)
        ));
        for reason in ["timeout", "failed", "busy"] {
            assert!(matches!(
                navigation_failure(reason),
                HostError::Unavailable(UnavailableReason::LaunchFailed)
            ));
        }
        assert!(matches!(
            navigation_failure("other"),
            HostError::Unavailable(UnavailableReason::ProtocolViolation)
        ));
        assert_eq!(WORKER_NAVIGATION_TIMEOUT, Duration::from_secs(45));
    }

    #[test]
    fn worker_frames_match_the_typescript_vocabulary() {
        let json = |request: &WorkerRequest<'_>| serde_json::to_string(request).unwrap();
        assert_eq!(
            json(&WorkerRequest::Hello { generation: 2 }),
            r#"{"frame":"hello","generation":2}"#
        );
        assert_eq!(json(&WorkerRequest::Shutdown), r#"{"frame":"shutdown"}"#);
        assert_eq!(
            json(&WorkerRequest::Ping { nonce: "n1" }),
            r#"{"frame":"ping","nonce":"n1"}"#
        );
        let argv = vec!["--headless".to_string()];
        let mut env = BTreeMap::new();
        env.insert("PATH".to_string(), "C:\\Windows\\System32".to_string());
        assert_eq!(
            json(&WorkerRequest::Launch {
                engine: "C:\\e.exe",
                profile_dir: "C:\\p",
                argv: &argv,
                env
            }),
            r#"{"frame":"launch","engine":"C:\\e.exe","profile_dir":"C:\\p","argv":["--headless"],"env":{"PATH":"C:\\Windows\\System32"}}"#
        );
        let reply = |text: &str| serde_json::from_str::<WorkerReply>(text);
        assert_eq!(
            reply(r#"{"frame":"hello","generation":1,"worker":"qdral-browser-worker"}"#).unwrap(),
            WorkerReply::Hello {
                generation: 1,
                worker: WORKER_IDENTITY.into()
            }
        );
        assert_eq!(
            reply(r#"{"frame":"launched"}"#).unwrap(),
            WorkerReply::Launched {}
        );
        assert!(reply(r#"{"frame":"launched","extra":1}"#).is_err());
        assert!(reply(r#"{"frame":"bye","extra":1}"#).is_err());
        assert_eq!(reply(r#"{"frame":"bye"}"#).unwrap(), WorkerReply::Bye {});
        assert!(reply(r#"{"frame":"navigate","url":"x"}"#).is_err());
        assert!(matches!(
            reply_error("launch_failed"),
            HostError::Unavailable(UnavailableReason::LaunchFailed)
        ));
        assert!(matches!(
            reply_error("launch_refused"),
            HostError::Invalid(_)
        ));
        assert!(matches!(
            reply_error("anything"),
            HostError::Unavailable(UnavailableReason::ProtocolViolation)
        ));
    }

    struct NoPolicy;
    impl PolicySource for NoPolicy {
        fn key_state(
            &self,
            _: crate::confinement::PolicyHive,
            _: &str,
        ) -> crate::confinement::PolicyKeyState {
            crate::confinement::PolicyKeyState::Absent
        }
    }

    #[test]
    fn a_lifetime_admits_exactly_one_destination() {
        let base = std::env::temp_dir();
        let engine = Engine::for_tests(base.join("msedge.exe"), crate::discovery::EngineKind::Edge);
        let destination = |host: &str| {
            crate::confinement::AdmittedDestination::fixture(
                host,
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            )
            .unwrap()
        };
        let launch = |destinations| WorkerLaunch {
            test_trusted_spki: None,
            node: base.join("node.exe"),
            worker_main: base.join("main.js"),
            engine: engine.clone(),
            profile_dir: base.clone(),
            ephemeral_profile: false,
            temp_dir: base.clone(),
            destinations,
        };
        for destinations in [
            Vec::new(),
            vec![destination("fixture.test"), destination("other.test")],
        ] {
            assert!(matches!(
                validate_launch(&launch(destinations)),
                Err(HostError::Invalid(message)) if message.contains("exactly one destination")
            ));
        }
        // One destination passes this rule (and then fails on the missing
        // runtime, as a typed prerequisite).
        assert!(matches!(
            validate_launch(&launch(vec![destination("fixture.test")])),
            Err(HostError::Unavailable(
                UnavailableReason::UnsupportedRuntime
            ))
        ));
    }

    #[test]
    fn missing_prerequisites_are_typed_and_nothing_is_spawned() {
        let base = std::env::temp_dir();
        // Built from temp_dir so the path is absolute on every platform.
        let engine = Engine::for_tests(base.join("msedge.exe"), crate::discovery::EngineKind::Edge);
        let spec = |node: PathBuf, main: PathBuf| WorkerLaunch {
            test_trusted_spki: None,
            node,
            worker_main: main,
            engine: engine.clone(),
            profile_dir: base.clone(),
            ephemeral_profile: false,
            temp_dir: base.clone(),
            destinations: vec![crate::confinement::AdmittedDestination::fixture(
                "fixture.test",
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            )
            .unwrap()],
        };
        let missing = base.join("qdral-sg108-missing");
        // A missing Node or worker is a missing prerequisite.
        assert!(matches!(
            launch_worker(
                &spec(missing.join("node.exe"), missing.join("main.js")),
                &NoPolicy
            ),
            Err(HostError::Unavailable(
                UnavailableReason::UnsupportedRuntime
            ))
        ));
        // A relative path is an invalid request.
        assert!(matches!(
            launch_worker(
                &spec(PathBuf::from("node.exe"), missing.join("main.js")),
                &NoPolicy
            ),
            Err(HostError::Invalid(_))
        ));
    }
}

#[cfg(all(test, windows))]
#[path = "worker_launch_tests.rs"]
mod native_tests;
