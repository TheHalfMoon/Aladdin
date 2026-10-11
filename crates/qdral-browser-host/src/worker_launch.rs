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
pub const WORKER_PROTOCOL_GENERATION: u32 = 1;
/// Worker identity reported in its hello.
pub const WORKER_IDENTITY: &str = "qdral-browser-worker";
/// The only Node option the worker accepts (inspector activation refused).
pub const WORKER_NODE_OPTION: &str = "--disable-sigusr1";

/// Bounded waits owned by the launcher.
pub const WORKER_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
pub const WORKER_LAUNCH_TIMEOUT: Duration = Duration::from_secs(60);
pub const WORKER_REPLY_TIMEOUT: Duration = Duration::from_secs(10);
pub const WORKER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

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
    Shutdown,
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum WorkerReply {
    Hello { generation: u32, worker: String },
    // Empty struct variants: serde ignores unknown fields on unit
    // variants of an internally tagged enum even with deny_unknown_fields.
    Launched {},
    Pong { nonce: String },
    Bye {},
    Error { code: String },
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
/// bare `--`, the last `type` switch in any accepted spelling wins, and an
/// absent or empty value means the browser process (`None`).
fn chromium_process_type(argv: &[String]) -> Option<String> {
    let mut kind = None;
    for arg in argv.iter().skip(1) {
        if arg == "--" {
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
    use crate::worker_process::{spawn, WorkerProcess};
    use std::fs::File;
    use std::io::BufReader;
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};

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
        let argv = build_worker_argv(&spec.profile_dir, &spec.destinations)?;
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
    fn worker_frames_match_the_typescript_vocabulary() {
        let json = |request: &WorkerRequest<'_>| serde_json::to_string(request).unwrap();
        assert_eq!(
            json(&WorkerRequest::Hello { generation: 1 }),
            r#"{"frame":"hello","generation":1}"#
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
    fn missing_prerequisites_are_typed_and_nothing_is_spawned() {
        let base = std::env::temp_dir();
        // Built from temp_dir so the path is absolute on every platform.
        let engine = Engine::for_tests(base.join("msedge.exe"), crate::discovery::EngineKind::Edge);
        let spec = |node: PathBuf, main: PathBuf| WorkerLaunch {
            node,
            worker_main: main,
            engine: engine.clone(),
            profile_dir: base.clone(),
            ephemeral_profile: false,
            temp_dir: base.clone(),
            destinations: Vec::new(),
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
