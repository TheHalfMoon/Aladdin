//! SG-000108 native launcher tests (Windows, real Node worker and Edge).
//!
//! Ignored in a plain `cargo test`: they need the built browser worker
//! (`apps/qdral-browser-worker/dist/main.js` with its dependencies) and an
//! installed Edge, located through QDRAL_SG108_NODE and
//! QDRAL_SG108_WORKER_MAIN. `npm run browser-launcher:native` (part of the
//! root `npm test`, after the worker build) runs them and requires every one
//! of them to run. Fixture destinations use the test-only loopback pin.

use super::*;
use crate::confinement::{PolicyHive, PolicyKeyState, RegistryPolicySource};
use crate::discovery::{discover_engine, Engine, SearchConfig};
use std::fs::File;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::os::windows::io::FromRawHandle;
use std::os::windows::process::CommandExt;
use std::sync::mpsc;
use std::time::Instant;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, STILL_ACTIVE};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};

const CHILD_MARKER: &str = "QDRAL_SG108_HOST_CHILD_OUT";

fn prerequisites() -> (PathBuf, PathBuf) {
    let node = std::env::var_os("QDRAL_SG108_NODE").expect("QDRAL_SG108_NODE (absolute node.exe)");
    let main = std::env::var_os("QDRAL_SG108_WORKER_MAIN")
        .expect("QDRAL_SG108_WORKER_MAIN (absolute dist/main.js)");
    (PathBuf::from(node), PathBuf::from(main))
}

fn engine() -> Engine {
    discover_engine(&SearchConfig::production()).expect("a verified installed Edge or Chrome")
}

fn temp_profile(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "sg108-launcher-{label}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create profile");
    dir
}

fn spec_with(main: PathBuf, profile: &Path, ephemeral: bool) -> WorkerLaunch {
    let (node, _) = prerequisites();
    WorkerLaunch {
        test_trusted_spki: None,
        node,
        worker_main: main,
        engine: engine(),
        profile_dir: profile.to_path_buf(),
        ephemeral_profile: ephemeral,
        temp_dir: std::env::temp_dir(),
        destinations: vec![AdmittedDestination::fixture(
            "fixture.test",
            IpAddr::V4(Ipv4Addr::LOCALHOST),
        )
        .expect("fixture destination")],
    }
}

fn spec(profile: &Path, ephemeral: bool) -> WorkerLaunch {
    spec_with(prerequisites().1, profile, ephemeral)
}

struct Policy(PolicyKeyState);
impl PolicySource for Policy {
    fn key_state(&self, _: PolicyHive, _: &str) -> PolicyKeyState {
        self.0
    }
}

fn alive(pid: u32) -> bool {
    // SAFETY: query-only open; the handle is closed below.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        return false;
    }
    let mut code = 0u32;
    // SAFETY: valid handle and out-pointer.
    let ok = unsafe { GetExitCodeProcess(handle, &mut code) } != 0;
    // SAFETY: opened above.
    unsafe { CloseHandle(handle) };
    ok && code == STILL_ACTIVE as u32
}

fn all_dead(pids: &[u32], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if pids.iter().all(|pid| !alive(*pid)) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn terminate(pid: u32) {
    // SAFETY: terminate-only open; closed below.
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle != 0 {
        // SAFETY: valid handle.
        unsafe {
            TerminateProcess(handle, 1);
            CloseHandle(handle);
        }
    }
}

fn job_pids(worker: &BrowserWorker) -> Vec<u32> {
    worker
        .job_processes()
        .expect("job snapshot")
        .iter()
        .map(|process| process.pid)
        .collect()
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn launch_verifies_the_job_and_shuts_down_without_residue() {
    let profile = temp_profile("lifecycle");
    let mut worker =
        launch_worker(&spec(&profile, true), &Policy(PolicyKeyState::Absent)).expect("launch");
    let pids = job_pids(&worker);
    assert!(pids.contains(&worker.worker_pid()));
    assert!(pids.contains(&worker.browser_pid()));
    assert!(
        pids.len() <= JOB_ACTIVE_PROCESS_LIMIT as usize,
        "{} processes",
        pids.len()
    );
    assert_eq!(worker.verify().expect("verify"), worker.browser_pid());
    worker.ping("native-1").expect("ping");
    // Control for the machine-wide residue check: it sees the verified
    // browser process (by pid: discovery may pick Edge or Chrome).
    let holders = processes_referencing(&profile);
    let browser = worker.browser_pid().to_string();
    assert!(
        holders
            .lines()
            .any(|line| line.split(' ').next() == Some(browser.as_str())),
        "the residue query does not see the running browser {browser}: {holders}"
    );
    let started = Instant::now();
    let report = worker.shutdown();
    eprintln!(
        "SG108-DIAG graceful shutdown took {:?}: {report:?}",
        started.elapsed()
    );
    assert!(report.graceful, "the worker did not answer bye and exit 0");
    assert!(report.job_empty, "the job did not empty");
    assert!(
        all_dead(&pids, Duration::from_secs(5)),
        "a job member survived"
    );
    assert_profile_gone(report.profile_removed, &profile);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn dropping_supervision_kills_the_job_and_removes_the_profile() {
    let profile = temp_profile("drop");
    let worker =
        launch_worker(&spec(&profile, true), &Policy(PolicyKeyState::Absent)).expect("launch");
    let pids = job_pids(&worker);
    drop(worker);
    assert!(
        all_dead(&pids, Duration::from_secs(10)),
        "a job member survived the drop"
    );
    assert_profile_gone(!profile.exists(), &profile);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_crashed_worker_is_detected_contained_and_restartable() {
    let profile = temp_profile("crash");
    let mut worker =
        launch_worker(&spec(&profile, false), &Policy(PolicyKeyState::Absent)).expect("launch");
    let pids = job_pids(&worker);
    terminate(worker.worker_pid());
    assert!(
        worker.ping("after-crash").is_err(),
        "a dead worker answered"
    );
    let report = worker.kill();
    assert!(report.job_empty && !report.graceful);
    assert!(
        all_dead(&pids, Duration::from_secs(10)),
        "the engine outlived its worker"
    );
    // The same profile can be used again once the crashed lifetime is gone.
    // Windows releases the dead engine's exclusive profile lock a moment
    // after the job empties (0.2-4.4 s observed); relaunching earlier makes
    // the new engine exit, so wait (bounded) for the release first.
    assert!(
        wait_for_profile_lock_release(&profile, Duration::from_secs(30)),
        "the crashed engine's profile lock was never released"
    );
    let mut again =
        launch_worker(&spec(&profile, true), &Policy(PolicyKeyState::Absent)).expect("relaunch");
    again.ping("restarted").expect("ping after restart");
    let second_lifetime = job_pids(&again);
    let report = again.shutdown();
    assert!(report.job_empty, "the restarted job did not empty");
    assert!(
        all_dead(&second_lifetime, Duration::from_secs(5)),
        "a restarted member survived"
    );
    assert_profile_gone(report.profile_removed, &profile);
}

/// Wait until Chromium's exclusive `lockfile` in `profile` is gone or can
/// be opened with no sharing, i.e. no process (or kernel mapping) holds it.
fn wait_for_profile_lock_release(profile: &Path, timeout: Duration) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    let lock = profile.join("lockfile");
    let started = Instant::now();
    loop {
        let free = !lock.exists()
            || std::fs::OpenOptions::new()
                .write(true)
                .share_mode(0)
                .open(&lock)
                .is_ok();
        if free {
            eprintln!(
                "SG108-DIAG profile lock released after {:?}",
                started.elapsed()
            );
            return true;
        }
        if started.elapsed() >= timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// No process anywhere on the machine (not only the job's recorded
/// members) may still name `profile` on its command line once supervision
/// has ended: a helper started outside the job would otherwise go unseen.
fn assert_no_process_references(profile: &Path) {
    let started = Instant::now();
    loop {
        let holders = processes_referencing(profile);
        if holders.is_empty() {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "processes still reference {}: {holders}",
            profile.display()
        );
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Every process except this one whose command line names the profile
/// directory, as "pid image" lines (native process snapshot).
fn processes_referencing(profile: &Path) -> String {
    let marker = profile
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .expect("profile directory name");
    crate::worker_process::processes_with_command_line_containing(&marker)
        .iter()
        .map(|process| format!("{} {}", process.pid, process.image.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The launcher must have removed the profile within its own budget. A
/// failure is diagnosed (the held file and how long it stays held, which
/// also cleans up) and still fails the test.
fn assert_profile_gone(reported_removed: bool, profile: &Path) {
    assert_no_process_references(profile);
    if reported_removed && !profile.exists() {
        return;
    }
    diagnose_locked(profile);
    panic!(
        "profile not removed by the launcher (reported removed: {reported_removed}): {}",
        profile.display()
    );
}

/// The real registry precondition on this machine: a launch exactly when
/// the policy check passes, and typed ManagedPolicy (nothing started)
/// exactly when it refuses. Hosted CI runners carry Edge policies, so both
/// outcomes are legitimate; which one ran is printed as evidence.
#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn the_real_registry_precondition_decides_the_launch() {
    let profile = temp_profile("registry");
    let launch = spec(&profile, true);
    let expected_refusal =
        crate::confinement::assert_no_managed_policy(launch.engine.kind(), &RegistryPolicySource)
            .is_err();
    match launch_worker(&launch, &RegistryPolicySource) {
        Ok(mut worker) => {
            assert!(!expected_refusal, "launched despite a managed policy");
            worker.ping("registry").expect("ping");
            let report = worker.shutdown();
            assert!(report.job_empty, "the job did not empty");
            assert_profile_gone(report.profile_removed, &profile);
            eprintln!("SG108-EVIDENCE registry precondition: no managed policy; launched");
        }
        Err(HostError::Unavailable(UnavailableReason::ManagedPolicy)) => {
            assert!(expected_refusal, "refused without a managed policy");
            assert_profile_gone(remove_profile(&profile), &profile);
            eprintln!(
                "SG108-EVIDENCE registry precondition: managed policy present; refused typed"
            );
        }
        Err(other) => panic!("unexpected launch result: {other:?}"),
    }
}

/// Investigation aid: name the first file that cannot be opened for
/// deletion and measure how long the lock lasts.
fn diagnose_locked(profile: &Path) {
    fn first_locked(dir: &Path) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = first_locked(&path) {
                    return Some(found);
                }
            } else if std::fs::OpenOptions::new().write(true).open(&path).is_err() {
                return Some(path);
            }
        }
        None
    }
    let started = Instant::now();
    let locked = first_locked(profile);
    eprintln!("SG108-DIAG locked file: {locked:?}");
    while profile.exists() && started.elapsed() < Duration::from_secs(90) {
        let _ = std::fs::remove_dir_all(profile);
        std::thread::sleep(Duration::from_millis(250));
    }
    eprintln!(
        "SG108-DIAG removable after a further {:?} (exists: {})",
        started.elapsed(),
        profile.exists()
    );
}

/// Child half of the host-death test: launch, publish the job pids, wait to
/// be killed. A no-op unless the parent test started this process.
#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn host_child_entry() {
    let Some(out) = std::env::var_os(CHILD_MARKER) else {
        return;
    };
    let profile =
        PathBuf::from(std::env::var_os("QDRAL_SG108_HOST_CHILD_PROFILE").expect("profile"));
    let worker =
        launch_worker(&spec(&profile, false), &Policy(PolicyKeyState::Absent)).expect("launch");
    // The worker and the browser first, then every other current member.
    let first = [worker.worker_pid(), worker.browser_pid()];
    let mut pids = first.to_vec();
    pids.extend(
        job_pids(&worker)
            .into_iter()
            .filter(|pid| !first.contains(pid)),
    );
    let text: Vec<String> = pids.iter().map(u32::to_string).collect();
    std::fs::write(PathBuf::from(out), text.join("\n")).expect("publish pids");
    std::thread::sleep(Duration::from_secs(120));
    drop(worker);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn killing_the_host_process_takes_the_worker_and_engine_down() {
    let profile = temp_profile("host-death");
    let out = profile.with_extension("pids");
    let (node, main) = prerequisites();
    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "worker_launch::native_tests::host_child_entry",
            "--ignored",
            "--test-threads=1",
        ])
        .env(CHILD_MARKER, &out)
        .env("QDRAL_SG108_HOST_CHILD_PROFILE", &profile)
        .env("QDRAL_SG108_NODE", node)
        .env("QDRAL_SG108_WORKER_MAIN", main)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn child host");
    let deadline = Instant::now() + Duration::from_secs(90);
    let pids: Vec<u32> = loop {
        if let Ok(text) = std::fs::read_to_string(&out) {
            let pids: Vec<u32> = text
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect();
            if pids.len() >= 2 {
                break pids;
            }
        }
        assert!(Instant::now() < deadline, "the child host never launched");
        std::thread::sleep(Duration::from_millis(200));
    };
    // The worker and the browser are alive before the kill (helpers may be
    // short-lived and are only required to be gone afterwards).
    assert!(
        alive(pids[0]) && alive(pids[1]),
        "worker or browser not running"
    );
    // Hard kill: no destructor runs in the child; only kill-on-close remains.
    child.kill().expect("kill child host");
    let _ = child.wait();
    assert!(
        all_dead(&pids, Duration::from_secs(15)),
        "worker or engine outlived the host"
    );
    let _ = std::fs::remove_file(&out);
    assert_profile_gone(remove_profile(&profile), &profile);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_second_browser_or_a_foreign_process_in_the_job_is_refused() {
    let profile = temp_profile("second");
    let worker =
        launch_worker(&spec(&profile, true), &Policy(PolicyKeyState::Absent)).expect("launch");
    assert!(worker.verify().is_ok());
    // A foreign process joins the job.
    let system32 =
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32");
    let mut foreign = std::process::Command::new(system32.join("PING.EXE"))
        .args(["-n", "30", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn foreign process");
    worker
        .process
        .assign_pid(foreign.id())
        .expect("assign foreign");
    let error = worker
        .verify()
        .expect_err("a foreign process must be refused");
    assert!(format!("{error}").contains("foreign process"), "{error}");
    let report = worker.kill();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
    let _ = foreign.wait();

    // A second browser process joins the job.
    let profile = temp_profile("second-b");
    let other = temp_profile("second-b-other");
    let worker =
        launch_worker(&spec(&profile, true), &Policy(PolicyKeyState::Absent)).expect("launch");
    // Suspended: it cannot exit or start helpers outside the job before it
    // is assigned; terminating the job ends it.
    let mut second = std::process::Command::new(engine().path())
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_SUSPENDED)
        .args([
            "--headless",
            "--no-first-run",
            &format!("--user-data-dir={}", other.display()),
            "about:blank",
        ])
        .spawn()
        .expect("spawn second browser");
    worker
        .process
        .assign_pid(second.id())
        .expect("assign second browser");
    let error = worker
        .verify()
        .expect_err("a second browser must be refused");
    assert!(
        format!("{error}").contains("exactly one browser process"),
        "{error}"
    );
    assert!(worker.kill().job_empty);
    let _ = second.wait();
    assert_profile_gone(remove_profile(&other), &other);
}

/// An inheritable pipe whose write end the test closes after starting a
/// process: EOF on the read end within `window` means the process did not
/// inherit the write end.
fn inheritable_pipe() -> (File, HANDLE) {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 1,
    };
    let (mut read, mut write): (HANDLE, HANDLE) = (0, 0);
    // SAFETY: valid out-pointers and attributes.
    assert!(unsafe { CreatePipe(&mut read, &mut write, &attributes, 0) } != 0);
    // SAFETY: the read end is owned by the returned File.
    (unsafe { File::from_raw_handle(read as _) }, write)
}

fn eof_within(mut read: File, write: HANDLE, window: Duration) -> bool {
    // SAFETY: the test's copy of the write end, closed exactly once.
    unsafe { CloseHandle(write) };
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut byte = [0u8; 1];
        let _ = sender.send(read.read(&mut byte).map(|n| n == 0).unwrap_or(true));
    });
    matches!(receiver.recv_timeout(window), Ok(true))
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn the_worker_inherits_no_handle_beyond_its_stdio() {
    let profile = temp_profile("inherit");
    let (read, write) = inheritable_pipe();
    let worker =
        launch_worker(&spec(&profile, true), &Policy(PolicyKeyState::Absent)).expect("launch");
    let inherited = !eof_within(read, write, Duration::from_secs(3));
    assert!(worker.shutdown().job_empty);
    assert!(
        !inherited,
        "the worker (or its engine) holds an inheritable host handle"
    );

    // Negative control: a process started without a handle list inherits the
    // same kind of handle, so the probe can detect inheritance.
    let (read, write) = inheritable_pipe();
    let system32 =
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32");
    let mut control = std::process::Command::new(system32.join("PING.EXE"))
        .args(["-n", "6", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn control");
    let control_inherited = !eof_within(read, write, Duration::from_millis(1500));
    let _ = control.kill();
    let _ = control.wait();
    assert!(control_inherited, "the probe cannot detect inheritance");
}

fn fake_worker(label: &str, body: &str) -> (PathBuf, PathBuf) {
    let dir = temp_profile(label);
    let main = dir.join("main.js");
    std::fs::write(&main, body).expect("write fake worker");
    (dir, main)
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_silent_worker_times_out_typed_and_is_terminated() {
    let (dir, main) = fake_worker(
        "silent",
        "require('fs').writeFileSync(__dirname + '\\\\pid', String(process.pid)); setInterval(() => {}, 1000);",
    );
    let profile = temp_profile("silent-profile");
    let started = Instant::now();
    let result = launch_worker(
        &spec_with(main, &profile, true),
        &Policy(PolicyKeyState::Absent),
    );
    assert!(matches!(
        result,
        Err(HostError::Unavailable(UnavailableReason::LaunchFailed))
    ));
    assert!(started.elapsed() >= WORKER_HANDSHAKE_TIMEOUT);
    let pid: u32 = std::fs::read_to_string(dir.join("pid"))
        .expect("the fake worker ran")
        .trim()
        .parse()
        .expect("pid");
    assert!(
        all_dead(&[pid], Duration::from_secs(5)),
        "a timed-out worker survived"
    );
    assert!(remove_profile(&dir) && remove_profile(&profile));
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_refused_runtime_is_typed_unsupported() {
    let (dir, main) = fake_worker("refused", "process.exit(2);");
    let profile = temp_profile("refused-profile");
    let result = launch_worker(
        &spec_with(main, &profile, true),
        &Policy(PolicyKeyState::Absent),
    );
    assert!(matches!(
        result,
        Err(HostError::Unavailable(
            UnavailableReason::UnsupportedRuntime
        ))
    ));
    assert!(remove_profile(&dir) && remove_profile(&profile));
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_managed_policy_refuses_before_anything_starts() {
    let profile = temp_profile("policy");
    for state in [PolicyKeyState::Populated, PolicyKeyState::Unreadable] {
        let result = launch_worker(&spec(&profile, false), &Policy(state));
        assert!(matches!(
            result,
            Err(HostError::Unavailable(UnavailableReason::ManagedPolicy))
        ));
    }
    assert!(remove_profile(&profile));
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn the_registry_reader_sees_a_real_test_hive() {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_ALL_ACCESS, REG_DWORD, REG_OPTION_NON_VOLATILE,
    };
    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() };
    // A test hive under the application's own key, never a policy location.
    let root = format!("SOFTWARE\\QdralTest\\SG000108\\{}", std::process::id());
    let create = |subkey: &str| -> HKEY {
        let mut key: HKEY = 0;
        // SAFETY: NUL-terminated name, valid out-pointer.
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(subkey).as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_ALL_ACCESS,
                std::ptr::null(),
                &mut key,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, 0, "RegCreateKeyExW");
        key
    };
    let reader = RegistryPolicySource;
    let state = || reader.key_state(PolicyHive::CurrentUser, &root);
    assert_eq!(state(), PolicyKeyState::Absent);
    let key = create(&root);
    assert_eq!(state(), PolicyKeyState::Empty);
    let value: u32 = 1;
    // SAFETY: valid key, NUL-terminated name, data pointer and size.
    let status = unsafe {
        RegSetValueExW(
            key,
            wide("ProxyMode").as_ptr(),
            0,
            REG_DWORD,
            &value as *const u32 as *const u8,
            4,
        )
    };
    assert_eq!(status, 0, "RegSetValueExW");
    assert_eq!(state(), PolicyKeyState::Populated);
    // SAFETY: opened above, closed once.
    unsafe { RegCloseKey(key) };
    // SAFETY: deletes only the test hive created above.
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide(&root).as_ptr()) };
    let sub = create(&format!("{root}\\Recommended"));
    // SAFETY: opened above, closed once.
    unsafe { RegCloseKey(sub) };
    assert_eq!(
        state(),
        PolicyKeyState::Populated,
        "a subkey alone is populated"
    );
    // SAFETY: deletes only the test hive.
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide("SOFTWARE\\QdralTest").as_ptr()) };
    assert_eq!(state(), PolicyKeyState::Absent);
}

/// The test-only HTTPS fixture (apps/qdral-browser-worker/scripts/
/// https-fixture.mjs): fixture.test and other.test on 127.0.0.1:443 with a
/// throwaway certificate, logging every request it receives.
struct HttpsFixture {
    child: std::process::Child,
    spki: String,
    hits: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl HttpsFixture {
    fn start() -> Self {
        use std::io::BufRead;
        let (node, main) = prerequisites();
        let app = main
            .parent()
            .and_then(Path::parent)
            .expect("worker app directory");
        let mut child = std::process::Command::new(node)
            .arg(app.join("scripts").join("https-fixture.mjs"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("start the HTTPS fixture");
        let stdout = child.stdout.take().expect("fixture stdout");
        let hits = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (ready, spki) = mpsc::channel();
        let log = hits.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
            {
                if let Some(value) = line.strip_prefix("READY ") {
                    let _ = ready.send(value.to_owned());
                } else if let Some(hit) = line.strip_prefix("HIT ") {
                    log.lock().expect("hits").push(hit.to_owned());
                }
            }
        });
        let spki = spki
            .recv_timeout(Duration::from_secs(30))
            .expect("the HTTPS fixture did not start");
        Self { child, spki, hits }
    }

    fn hits(&self) -> Vec<String> {
        self.hits.lock().expect("hits").clone()
    }
}

impl Drop for HttpsFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Keep the host answering route requests for `duration` (it decides only
/// while it is in a call, as a real caller would be).
fn pump(worker: &mut BrowserWorker, duration: Duration) {
    let until = Instant::now() + duration;
    let mut n = 0;
    while Instant::now() < until {
        std::thread::sleep(Duration::from_millis(250));
        n += 1;
        worker
            .ping(&format!("pump-{n}"))
            .expect("ping while pumping");
    }
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn the_host_decides_every_request_of_a_live_navigation() {
    let fixture = HttpsFixture::start();
    let profile = temp_profile("navigate");
    let mut launch = spec(&profile, true);
    let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
    // One destination per lifetime: other.test is served by the fixture but
    // is not admitted, so the engine cannot reach it at all, whatever issues
    // the request (routes, guards, or the browser process itself).
    launch.destinations =
        vec![AdmittedDestination::fixture("fixture.test", loopback).expect("fixture.test")];
    launch.test_trusted_spki = Some(fixture.spki.clone());
    let mut worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");

    // An admitted page loads and commits.
    let landed = worker
        .navigate("https://fixture.test/landing")
        .expect("admitted navigation");
    assert_eq!(
        (landed.url.as_str(), landed.status),
        ("https://fixture.test/landing", 200)
    );
    // A same-origin redirect is followed hop by hop.
    let redirected = worker
        .navigate("https://fixture.test/redirect-same")
        .expect("same-origin redirect");
    assert_eq!(redirected.url, "https://fixture.test/landing");
    // Widening redirects, loops and over-long chains are refused (typed).
    for url in [
        "https://fixture.test/redirect-cross",
        "https://fixture.test/loop-a",
        "https://fixture.test/chain/0",
    ] {
        assert!(
            matches!(worker.navigate(url), Err(HostError::Invalid(_))),
            "{url} was not refused"
        );
    }
    // A destination outside this lifetime is refused before anything is sent.
    for url in [
        "https://example.com/",
        "http://fixture.test/",
        "https://127.0.0.1/",
    ] {
        assert!(worker.navigate(url).is_err(), "{url} was not refused");
    }
    // Cross-origin frames, images and fetches are refused; same-origin
    // resources load.
    worker
        .navigate("https://fixture.test/mixed")
        .expect("mixed page");
    // Popups, speculation rules, beacons, workers and the like settle; the
    // host answers the page's own requests only while it is in a call, so
    // keep it in calls.
    pump(&mut worker, Duration::from_secs(4));
    // The prerender-then-activate escape: the page prerenders a same-origin
    // URL that redirects to other.test and then navigates to it. With
    // preloading off the redirect hop is asked and refused.
    worker
        .navigate("https://fixture.test/spec-escape")
        .expect("spec-escape page");
    pump(&mut worker, Duration::from_secs(5));
    let hits = fixture.hits();
    let other: Vec<&String> = hits
        .iter()
        .filter(|hit| hit.starts_with("other.test"))
        .collect();
    assert!(other.is_empty(), "other.test was contacted: {other:?}");
    // The escapes really ran: the page navigated itself to /redirect-evil
    // (whose cross-origin hop was then refused), and the shared worker's
    // script was refused outright.
    assert!(
        hits.contains(&"fixture.test /redirect-evil".to_string()),
        "the activation attempt never happened: {hits:?}"
    );
    assert!(
        !hits.contains(&"fixture.test /shared.js".to_string()),
        "the shared worker script was fetched: {hits:?}"
    );
    for path in ["/pixel.png", "/frame", "/api", "/style.css"] {
        assert!(
            hits.contains(&format!("fixture.test {path}")),
            "same-origin {path} did not load: {hits:?}"
        );
    }
    let report = worker.shutdown();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn observation_is_bounded_redacted_and_bound_to_the_committed_document() {
    let fixture = HttpsFixture::start();
    let profile = temp_profile("observe");
    let mut launch = spec(&profile, true);
    launch.destinations =
        vec![
            AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST))
                .expect("fixture.test"),
        ];
    launch.test_trusted_spki = Some(fixture.spki.clone());
    let mut worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");
    let scope = crate::observation::ObservationScope {
        workspace: "workspace".into(),
        profile_identity: "profile".into(),
    };
    // Nothing to observe before a committed document.
    assert!(worker.observe(&scope).is_err());

    worker
        .navigate("https://fixture.test/reports")
        .expect("reports page");
    let live = worker.observe(&scope).expect("observe");
    assert!(
        live.observation.node_count > 5,
        "{}",
        live.observation.node_count
    );
    let shown = format!("{:?}", live.observation);
    for raw in ["Quarterly report", "hunter2", "alice"] {
        assert!(
            !shown.contains(raw),
            "{raw} reached the caller-visible observation"
        );
    }
    // Host-issued identities bound to this document; the host keeps the raw
    // link names and URLs for its own typed selection.
    let generation = live.document.document_generation;
    let mut links = Vec::new();
    let mut stack: Vec<&crate::observation::ObservedNode> = live.observation.nodes.iter().collect();
    while let Some(node) = stack.pop() {
        assert!(
            node.node_id.starts_with(&format!("d{generation}-")),
            "{}",
            node.node_id
        );
        let host = worker.host_node(&node.node_id).expect("host index entry");
        assert!(!host.name.contains("hunter2") && !host.name.contains("alice"));
        if host.role == "link" {
            links.push((host.name.clone(), host.url.clone()));
        }
        stack.extend(node.children.iter());
    }
    links.sort();
    assert_eq!(
        links
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        [
            "Quarterly report 2026-03-31",
            "Quarterly report 2026-06-30",
            "Quarterly report 2026-09-30"
        ]
    );
    assert!(links
        .iter()
        .all(|(_, url)| url.as_deref().is_some_and(|u| u.starts_with("/reports/"))));

    // A button named by a password field (aria-labelledby): the raw secret
    // never reaches the caller, and its digest is keyed (not checkable
    // against a guessed value).
    worker
        .navigate("https://fixture.test/secret-name")
        .expect("secret-name page");
    let live = worker.observe(&scope).expect("observe secret-name");
    assert!(!format!("{:?}", live.observation).contains("HUNTER2PW"));
    let unkeyed = crate::observation::short_digest("HUNTER2PW");
    let mut stack: Vec<&crate::observation::ObservedNode> = live.observation.nodes.iter().collect();
    let mut buttons = 0;
    while let Some(node) = stack.pop() {
        if node.role == "button" {
            buttons += 1;
            assert_ne!(
                node.name_digest, unkeyed,
                "the name digest is dictionary-checkable"
            );
        }
        stack.extend(node.children.iter());
    }
    assert_eq!(buttons, 1, "the secret-named button was not observed");

    // A page that moved itself is stale until the host navigates again.
    worker
        .navigate("https://fixture.test/self-move")
        .expect("self-moving page");
    pump(&mut worker, Duration::from_secs(2));
    assert!(
        matches!(worker.observe(&scope), Err(HostError::Invalid(_))),
        "a moved page was observed"
    );
    let report = worker.shutdown();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
    drop(fixture);
}

/// An SG-000077 target and a matching one-shot approval for a node of the
/// current observation.
fn approved_click(
    worker: &BrowserWorker,
    live: &LiveObservation,
    node_id: &str,
) -> (
    crate::actuation::ActuationTarget,
    crate::actuation::ApprovalToken,
) {
    // The approval names where the click goes: the host-resolved link
    // destination is the target's state.
    let state = worker.link_destination(node_id).unwrap_or_default();
    let target = crate::actuation::ActuationTarget {
        page_id: live.page.page_id.clone(),
        origin: live.page.origin.clone(),
        document_generation: live.document.document_generation,
        node_id: node_id.to_owned(),
        role: "link".into(),
        state,
        action: crate::actuation::SupportedAction::Click,
        workspace: "workspace".into(),
        policy_revision: 1,
    };
    let approval = crate::actuation::ApprovalToken {
        approval_id: "approval-1".into(),
        target_digest: crate::actuation::target_digest(&target),
        workspace: "workspace".into(),
        policy_revision: 1,
        consumed: false,
    };
    (target, approval)
}

/// A download approval: it names the link, the approved folder and the
/// file name (the host's `download_binding`).
fn approved_download(
    worker: &BrowserWorker,
    live: &LiveObservation,
    node_id: &str,
    root: &Path,
    relative: &str,
) -> (
    crate::actuation::ActuationTarget,
    crate::actuation::ApprovalToken,
) {
    let (mut target, _) = approved_click(worker, live, node_id);
    target.action = crate::actuation::SupportedAction::Download;
    target.state = worker
        .download_binding(node_id, root, relative)
        .unwrap_or_default();
    let approval = crate::actuation::ApprovalToken {
        approval_id: "approval-dl".into(),
        target_digest: crate::actuation::target_digest(&target),
        workspace: "workspace".into(),
        policy_revision: 1,
        consumed: false,
    };
    (target, approval)
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn an_approved_activation_follows_the_latest_dated_link_and_nothing_else() {
    let fixture = HttpsFixture::start();
    let profile = temp_profile("activate");
    let mut launch = spec(&profile, true);
    launch.destinations =
        vec![
            AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST))
                .expect("fixture.test"),
        ];
    launch.test_trusted_spki = Some(fixture.spki.clone());
    let mut worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");
    let scope = crate::observation::ObservationScope {
        workspace: "workspace".into(),
        profile_identity: "profile".into(),
    };

    worker
        .navigate("https://fixture.test/archive")
        .expect("archive");
    let live = worker.observe(&scope).expect("observe");
    let latest = worker
        .select_latest_dated_link()
        .expect("latest dated link");
    assert_eq!(
        worker.host_node(&latest).map(|n| n.name.as_str()),
        Some("Annual review 2026-09-30")
    );
    // A digest that does not match the target is refused before any click.
    let (target, mut forged) = approved_click(&worker, &live, &latest);
    forged.target_digest = "0000000000000000".into();
    assert!(worker
        .activate(&target, &mut forged, 1, "workspace")
        .is_err());
    // The approved click navigates to the selected document.
    let (target, mut approval) = approved_click(&worker, &live, &latest);
    let opened = worker
        .activate(&target, &mut approval, 1, "workspace")
        .expect("approved activation");
    assert_eq!(opened.url, "https://fixture.test/archive/2026-09-30");
    assert!(approval.consumed);
    // The approval cannot be replayed, and the old node is stale.
    assert!(worker
        .activate(&target, &mut approval, 1, "workspace")
        .is_err());
    let (target, mut fresh) = approved_click(&worker, &live, &latest);
    assert!(
        worker
            .activate(&target, &mut fresh, 1, "workspace")
            .is_err(),
        "stale node"
    );
    // The next observation is of the new document, one generation later.
    let after = worker.observe(&scope).expect("observe after activation");
    assert_eq!(
        after.document.document_generation,
        live.document.document_generation + 1
    );

    // A hostile page whose latest-dated link is cross-origin: the host's
    // selection picks it, and the activation is refused by the policy.
    worker
        .navigate("https://fixture.test/archive-hostile")
        .expect("hostile archive");
    let live = worker.observe(&scope).expect("observe hostile");
    let latest = worker.select_latest_dated_link().expect("latest");
    assert_eq!(
        worker.host_node(&latest).map(|n| n.name.as_str()),
        Some("Annual review 2099-12-31")
    );
    let (target, mut approval) = approved_click(&worker, &live, &latest);
    assert!(
        worker
            .activate(&target, &mut approval, 1, "workspace")
            .is_err(),
        "a cross-origin activation was followed"
    );
    assert!(
        !approval.consumed,
        "a refused destination consumed the approval"
    );

    // A click redirected by script, and an href swapped after observation:
    // both open another document than the approved destination: refused.
    let find = |worker: &BrowserWorker, live: &LiveObservation, name: &str| {
        let mut stack: Vec<&crate::observation::ObservedNode> =
            live.observation.nodes.iter().collect();
        while let Some(node) = stack.pop() {
            if worker
                .host_node(&node.node_id)
                .is_some_and(|h| h.name == name)
            {
                return node.node_id.clone();
            }
            stack.extend(node.children.iter());
        }
        panic!("no node named {name}");
    };
    worker
        .navigate("https://fixture.test/archive-tricky")
        .expect("tricky");
    let live = worker.observe(&scope).expect("observe tricky");
    let scripted = find(&worker, &live, "Annual review 2026-09-30");
    let (target, mut approval) = approved_click(&worker, &live, &scripted);
    assert!(
        worker
            .activate(&target, &mut approval, 1, "workspace")
            .is_err(),
        "script redirect followed"
    );
    worker
        .navigate("https://fixture.test/archive-tricky")
        .expect("tricky again");
    let live = worker.observe(&scope).expect("observe tricky again");
    let swapped = find(&worker, &live, "Annual review 2026-06-30");
    pump(&mut worker, Duration::from_millis(2_500));
    let (target, mut approval) = approved_click(&worker, &live, &swapped);
    assert!(
        worker
            .activate(&target, &mut approval, 1, "workspace")
            .is_err(),
        "swapped href followed"
    );
    // A link that opens a popup fails fast and typed, and the popup's
    // document is never requested.
    worker
        .navigate("https://fixture.test/archive-tricky")
        .expect("tricky for the popup");
    let live = worker
        .observe(&scope)
        .expect("observe tricky for the popup");
    let blank = find(&worker, &live, "Annual review 2025-12-31");
    let (target, mut approval) = approved_click(&worker, &live, &blank);
    let started = std::time::Instant::now();
    assert!(
        worker
            .activate(&target, &mut approval, 1, "workspace")
            .is_err(),
        "a popup was taken as the activation"
    );
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "the popup activation waited {:?}",
        started.elapsed()
    );
    worker.ping("after-popup").expect("the lifetime survived");
    pump(&mut worker, Duration::from_secs(1));
    let hits = fixture.hits();
    assert!(
        !hits.iter().any(|h| h.contains("?popup")),
        "the popup was requested: {hits:?}"
    );
    for path in ["/js-dest", "/evil"] {
        assert!(
            !hits.contains(&format!("fixture.test {path}")),
            "{path} was requested: {hits:?}"
        );
    }
    pump(&mut worker, Duration::from_secs(1));
    assert!(
        !fixture
            .hits()
            .iter()
            .any(|hit| hit.starts_with("other.test")),
        "other.test was contacted"
    );
    let report = worker.shutdown();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_page_is_extracted_as_untrusted_markdown_in_pieces_without_hidden_text() {
    let fixture = HttpsFixture::start();
    let profile = temp_profile("extract");
    let mut launch = spec(&profile, true);
    launch.destinations =
        vec![
            AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST))
                .expect("fixture.test"),
        ];
    launch.test_trusted_spki = Some(fixture.spki.clone());
    let mut worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");
    // Nothing is committed yet: nothing to extract.
    assert!(matches!(
        worker.extract_page(crate::extraction::ExtractOptions::default()),
        Err(HostError::Invalid(_))
    ));

    worker.navigate("https://fixture.test/read").expect("read");
    pump(&mut worker, Duration::from_millis(500));
    let before = fixture.hits().len();
    let started = std::time::Instant::now();
    let page = worker
        .extract_page(crate::extraction::ExtractOptions::default())
        .expect("extract");
    let elapsed = started.elapsed();
    assert_eq!(page.url, "https://fixture.test/read");
    assert!(!page.truncated);
    assert!(page.chunks.len() > 1, "{} chunks", page.chunks.len());
    let text: String = page
        .chunks
        .iter()
        .map(crate::webmcp::PageText::untrusted)
        .collect::<Vec<_>>()
        .join("\n");
    println!(
        "SG108-EVIDENCE extraction: {} chunks, {} chars, {:?}",
        page.chunks.len(),
        text.chars().count(),
        elapsed
    );
    assert!(text.contains("# Reading room"), "{}", &text[..200]);
    assert!(text.contains("Visible opening paragraph"));
    assert!(text.contains("end-0") && text.contains("end-159"));
    assert!(text.contains("Closing paragraph."));
    assert!(
        text.contains("\u{e9}\u{4e2d}"),
        "non-ASCII text survives the pieces"
    );
    // Hidden text never crosses; page-injected syntax stays literal.
    assert!(!text.contains("LEAK"), "hidden text leaked");
    assert!(
        !text.contains("[click me](javascript"),
        "a page-injected link formed"
    );
    assert!(
        text.contains("\\[click me\\]"),
        "the injected link is escaped"
    );
    assert!(!text.contains("<script>"), "page-injected HTML formed");
    // A preformatted block larger than one frame comes back as one chunk:
    // the injected link at its end stays inside its fence.
    let pre = page
        .chunks
        .iter()
        .map(crate::webmcp::PageText::untrusted)
        .find(|c| c.contains("[pre link](javascript:alert(3))"))
        .expect("the preformatted block");
    let at = pre.find("[pre link]").unwrap_or_default();
    assert_eq!(
        pre[..at].matches("```").count() % 2,
        1,
        "the injected link is outside a fence"
    );
    assert!(pre.chars().count() > 60_000, "the block was cut");
    // Links and images are opt-in.
    assert!(!text.contains("](https://"));
    // The engine read its own DOM: the page requested nothing.
    assert_eq!(fixture.hits().len(), before, "the extraction made requests");

    let linked = worker
        .extract_page(crate::extraction::ExtractOptions {
            links: true,
            images: true,
        })
        .expect("extract with links");
    let text: String = linked
        .chunks
        .iter()
        .map(crate::webmcp::PageText::untrusted)
        .collect();
    assert!(
        text.contains("[the archive](https://fixture.test/archive)"),
        "{}",
        &text[..400]
    );
    assert!(text.contains("![a chart](https://fixture.test/img.png)"));

    // A generation later, the old extraction's document is gone; a new one
    // is of the new document.
    worker
        .navigate("https://fixture.test/archive")
        .expect("archive");
    let archive = worker
        .extract_page(crate::extraction::ExtractOptions::default())
        .expect("extract archive");
    assert_eq!(archive.document_generation, page.document_generation + 1);
    assert!(archive
        .chunks
        .iter()
        .any(|c| c.untrusted().contains("Annual review 2026-09-30")));
    worker.ping("after-extract").expect("the lifetime goes on");

    let report = worker.shutdown();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn webmcp_declarative_tools_are_discovered_read_only_and_bound_to_the_document() {
    let fixture = HttpsFixture::start();
    let profile = temp_profile("webmcp");
    let mut launch = spec(&profile, true);
    launch.destinations =
        vec![
            AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST))
                .expect("fixture.test"),
        ];
    launch.test_trusted_spki = Some(fixture.spki.clone());
    let mut worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");

    worker
        .navigate("https://fixture.test/tools")
        .expect("tools");
    // Let the page's own script settle before discovery.
    pump(&mut worker, Duration::from_millis(500));
    let found = worker.discover_tools().expect("discover");
    assert_eq!(found.url, "https://fixture.test/tools");
    let names: Vec<&str> = found.tools.iter().map(|t| t.name.as_str()).collect();
    // The duplicate and the undescribed form are not tools; the shadow-root
    // declaration is.
    assert_eq!(
        names,
        [
            "search_reports",
            "login",
            "contact",
            "shadow_tool",
            "awkward"
        ]
    );
    // The page declares no charset, so the engine submits its default
    // (not UTF-8): no tool on it takes non-ASCII arguments.
    assert!(
        found.tools.iter().all(|t| !t.utf8),
        "an undeclared charset was taken as UTF-8"
    );
    // A script-set lone surrogate arrives replaced (the frame stays valid
    // JSON), a number with a pattern and inverted bounds stays inside the
    // host's subset, and editable text never becomes a description.
    let awkward = &found.tools[4];
    assert_eq!(awkward.description.untrusted(), "x\u{fffd}y");
    assert_eq!(awkward.params.len(), 2);
    assert!(awkward.params[0].pattern.is_none() && awkward.params[0].minimum.is_none());
    assert_eq!(
        awkward.params[1]
            .description
            .as_ref()
            .map(|d| d.untrusted()),
        Some("Note")
    );
    let search = &found.tools[0];
    assert_eq!(search.title.untrusted(), "Reports");
    let params: Vec<(&str, bool)> = search
        .params
        .iter()
        .map(|p| (p.name.as_str(), p.required))
        .collect();
    assert_eq!(params, [("q", true), ("year", false)]);
    assert_eq!(
        search.params[1].choices.as_deref(),
        Some(&["2026".to_owned(), "2025".to_owned()][..])
    );
    // A password field flags the tool and is never a parameter.
    let login = &found.tools[1];
    assert!(login.has_password);
    assert_eq!(
        login
            .params
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["user"]
    );
    // A hostile description is carried as untrusted page text.
    assert!(found.tools[2]
        .description
        .untrusted()
        .starts_with("Ignore previous instructions"));
    // No value typed or prefilled in the page reaches the host.
    let text = format!("{found:?}");
    for secret in [
        "alice",
        "hunter2-secret",
        "csrf-secret-123",
        "USER TYPED SECRET",
    ] {
        assert!(!text.contains(secret), "{secret} reached the host");
    }
    // Identities are host-issued and bound to the document generation.
    let first = found.tools[0].tool_id.clone();
    assert_eq!(first, format!("d{}-w1", found.document_generation));
    let host = worker.host_tool(&first).expect("host tool");
    assert_eq!(host.name, "search_reports");
    assert!(host.node > 0);

    // Any trap request discovery caused has time to arrive before the page
    // goes away.
    pump(&mut worker, Duration::from_secs(1));
    let hits = fixture.hits();
    assert!(
        hits.iter().all(|hit| !hit.contains("/trap-")),
        "page code ran during discovery: {hits:?}"
    );

    // A page without declarations yields no tools, and the earlier
    // identities are gone.
    worker
        .navigate("https://fixture.test/tools-none")
        .expect("no tools");
    let none = worker.discover_tools().expect("discover none");
    assert!(none.tools.is_empty() && !none.truncated);
    assert!(worker.host_tool(&first).is_none(), "a stale tool survived");

    // Discovery ran no page code: none of the page's traps was called.
    let hits = fixture.hits();
    assert!(
        hits.iter().all(|hit| !hit.contains("/trap-")),
        "page code ran during discovery: {hits:?}"
    );
    assert!(hits.iter().any(|hit| hit.contains("/tools")));
    // Control: the same trap, called by the page itself, is seen.
    worker
        .navigate("https://fixture.test/tools-control")
        .expect("control");
    pump(&mut worker, Duration::from_secs(2));
    let control = fixture.hits();
    for trap in [
        "/trap-control",
        "/trap-control-mutation",
        "/trap-control-focus",
    ] {
        assert!(
            control.iter().any(|hit| hit.ends_with(trap)),
            "the {trap} control never reached the fixture: {control:?}"
        );
    }
    let report = worker.shutdown();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
}

/// An approval for executing `tool_id` with `args` (the host's binding).
fn approved_execution(
    worker: &BrowserWorker,
    found: &crate::webmcp::DiscoveredTools,
    tool_id: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> (
    crate::actuation::ActuationTarget,
    crate::actuation::ApprovalToken,
) {
    let target = crate::actuation::ActuationTarget {
        page_id: format!("p{}", found.document_generation),
        origin: "https://fixture.test".into(),
        document_generation: found.document_generation,
        node_id: tool_id.to_owned(),
        role: "webmcp-tool".into(),
        state: worker.execution_binding(tool_id, args).unwrap_or_default(),
        action: crate::actuation::SupportedAction::Execute,
        workspace: "workspace".into(),
        policy_revision: 1,
    };
    let approval = crate::actuation::ApprovalToken {
        approval_id: "approval-exec".into(),
        target_digest: crate::actuation::target_digest(&target),
        workspace: "workspace".into(),
        policy_revision: 1,
        consumed: false,
    };
    (target, approval)
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn webmcp_tools_execute_only_with_approval_and_return_the_verified_result() {
    let fixture = HttpsFixture::start();
    let profile = temp_profile("webmcp-exec");
    let mut launch = spec(&profile, true);
    launch.destinations =
        vec![
            AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST))
                .expect("fixture.test"),
        ];
    launch.test_trusted_spki = Some(fixture.spki.clone());
    let mut worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");
    let args = |json: &str| -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str(json).expect("args")
    };
    let open = |worker: &mut BrowserWorker| {
        worker
            .navigate("https://fixture.test/exec")
            .expect("exec page");
        worker.discover_tools().expect("discover")
    };
    let id_of = |found: &crate::webmcp::DiscoveredTools, name: &str| {
        found
            .tools
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.tool_id.clone())
            .expect("tool")
    };

    // GET: the approved arguments are submitted, the browser adds the hidden
    // field, and the result's structured data comes back as page text.
    let found = open(&mut worker);
    let search = id_of(&found, "search_reports");
    let wanted = args(r#"{"q":"rust","year":"2025","exact":true}"#);
    let (target, mut approval) = approved_execution(&worker, &found, &search, &wanted);
    // A forged approval, an approval for other arguments, and undeclared
    // arguments are refused without spending anything.
    let mut forged = approval.clone();
    forged.target_digest = "forged".into();
    assert!(worker
        .execute_tool(&target, &mut forged, &wanted, 1, "workspace")
        .is_err());
    assert!(!forged.consumed);
    let other = args(r#"{"q":"evil","year":"2025","exact":true}"#);
    assert!(worker
        .execute_tool(&target, &mut approval, &other, 1, "workspace")
        .is_err());
    assert!(
        !approval.consumed,
        "an approval for other arguments was spent"
    );
    assert!(worker
        .execute_tool(
            &target,
            &mut approval,
            &args(r#"{"q":"rust","extra":1}"#),
            1,
            "workspace"
        )
        .is_err());
    assert!(!approval.consumed);
    let outcome = worker
        .execute_tool(&target, &mut approval, &wanted, 1, "workspace")
        .expect("execute");
    assert!(approval.consumed);
    assert!(
        outcome.url.starts_with("https://fixture.test/search?"),
        "{}",
        outcome.url
    );
    assert_eq!(outcome.status, 200);
    let result: serde_json::Value =
        serde_json::from_str(outcome.structured[0].untrusted()).expect("structured data");
    assert_eq!(result["query"], "rust");
    assert_eq!(result["year"], "2025");
    assert_eq!(result["exact"], "on");
    assert_eq!(
        result["csrf"], "csrf-123",
        "the hidden field was not submitted"
    );
    // The old tool identity is stale after the submission's navigation.
    assert!(matches!(
        worker.execute_tool(&target, &mut approval, &wanted, 1, "workspace"),
        Err(HostError::Invalid(message)) if message.contains("stale")
    ));
    // A replay: the spent approval on a fresh identity is refused as a
    // replay, and nothing is submitted again.
    let found = open(&mut worker);
    let (fresh, _) = approved_execution(&worker, &found, &id_of(&found, "search_reports"), &wanted);
    assert!(matches!(
        worker.execute_tool(&fresh, &mut approval, &wanted, 1, "workspace"),
        Err(HostError::Invalid(message)) if message.contains("replay")
    ));

    // POST: the exact action, with a multi-line note.
    let found = open(&mut worker);
    let note = id_of(&found, "post_note");
    let text = args(r#"{"note":"hello\nworld"}"#);
    let (target, mut approval) = approved_execution(&worker, &found, &note, &text);
    let saved = worker
        .execute_tool(&target, &mut approval, &text, 1, "workspace")
        .expect("post");
    assert_eq!(saved.url, "https://fixture.test/note");
    let result: serde_json::Value =
        serde_json::from_str(saved.structured[0].untrusted()).expect("structured data");
    assert_eq!(result["saved"], true);
    // The fixture saw exactly the approved note ("hello", CRLF, "world").
    assert_eq!(result["length"], 12);
    assert!(saved.structured_complete);

    // Tools that must not execute: no autosubmit (no binding at all), a
    // cross-origin action (refused before dispatch, approval unspent).
    let found = open(&mut worker);
    assert!(worker
        .execution_binding(&id_of(&found, "needs_user"), &args(r#"{"q":"x"}"#))
        .is_err());
    let escape = id_of(&found, "escape");
    let (target, mut approval) =
        approved_execution(&worker, &found, &escape, &args(r#"{"q":"x"}"#));
    assert!(worker
        .execute_tool(
            &target,
            &mut approval,
            &args(r#"{"q":"x"}"#),
            1,
            "workspace"
        )
        .is_err());
    assert!(
        !approval.consumed,
        "a cross-origin action spent its approval"
    );

    // A page script that swallows the submission opens no document: not a
    // verified execution (the approval was dispatched, so it is spent).
    let found = open(&mut worker);
    let swallowed = id_of(&found, "script_handled");
    let (target, mut approval) =
        approved_execution(&worker, &found, &swallowed, &args(r#"{"q":"swallowed"}"#));
    let refused = worker.execute_tool(
        &target,
        &mut approval,
        &args(r#"{"q":"swallowed"}"#),
        1,
        "workspace",
    );
    assert!(refused.is_err(), "{refused:?}");
    assert!(approval.consumed);

    // A page script that moves somewhere else instead is refused at the
    // request.
    let found = open(&mut worker);
    let moving = id_of(&found, "redirecting");
    let (target, mut approval) =
        approved_execution(&worker, &found, &moving, &args(r#"{"q":"moved"}"#));
    assert!(worker
        .execute_tool(
            &target,
            &mut approval,
            &args(r#"{"q":"moved"}"#),
            1,
            "workspace"
        )
        .is_err());
    assert!(
        approval.consumed,
        "a dispatched execution must spend its approval"
    );

    // Structures where position and ownership disagree are not executable:
    // no binding, so no approval can even be formed.
    let found = open(&mut worker);
    for name in ["decoy", "foreign_button", "shadowed", "method_override"] {
        let tool = found.tools.iter().find(|t| t.name == name).expect(name);
        assert!(!tool.self_contained, "{name} was taken as self-contained");
        assert!(
            worker
                .execution_binding(&tool.tool_id, &args(r#"{"q":"x"}"#))
                .is_err(),
            "{name} was executable"
        );
    }

    // A submit handler that rewrites a field is refused at the request: the
    // request does not carry the approved value.
    let rewriting = id_of(&found, "rewriting");
    let (target, mut approval) =
        approved_execution(&worker, &found, &rewriting, &args(r#"{"q":"approved"}"#));
    assert!(worker
        .execute_tool(
            &target,
            &mut approval,
            &args(r#"{"q":"approved"}"#),
            1,
            "workspace"
        )
        .is_err());
    assert!(approval.consumed);

    // The form's own hidden token is submitted as the page set it.
    let found = open(&mut worker);
    let hidden = id_of(&found, "hidden_token");
    let tool = found.tools.iter().find(|t| t.tool_id == hidden).unwrap();
    assert_eq!(tool.fixed_names, vec!["csrf".to_owned()]);
    let (target, mut approval) =
        approved_execution(&worker, &found, &hidden, &args(r#"{"q":"tokened"}"#));
    worker
        .execute_tool(
            &target,
            &mut approval,
            &args(r#"{"q":"tokened"}"#),
            1,
            "workspace",
        )
        .expect("a hidden token is the page's own");

    // A submit handler that adds a name the form never declared is refused
    // at the request.
    let found = open(&mut worker);
    let adding = id_of(&found, "adding");
    let (target, mut approval) =
        approved_execution(&worker, &found, &adding, &args(r#"{"q":"plain"}"#));
    assert!(worker
        .execute_tool(
            &target,
            &mut approval,
            &args(r#"{"q":"plain"}"#),
            1,
            "workspace"
        )
        .is_err());
    assert!(approval.consumed);

    // A form that submits another charset takes only ASCII arguments: a
    // non-ASCII one gets no binding.
    let found = open(&mut worker);
    let latin = id_of(&found, "latin");
    assert!(
        !found
            .tools
            .iter()
            .find(|t| t.tool_id == latin)
            .unwrap()
            .utf8
    );
    assert!(worker
        .execution_binding(&latin, &args(r#"{"q":"café"}"#))
        .is_err());
    assert!(worker
        .execution_binding(&latin, &args(r#"{"q":"cafe"}"#))
        .is_ok());

    // A parameter the page already fills must be supplied; names the
    // browser adds (image coordinates, dirname) and an invalid button type
    // are the form's own; the page declares UTF-8 and the form follows it.
    let found = open(&mut worker);
    let tool = |name: &str| found.tools.iter().find(|t| t.name == name).expect(name);
    assert!(tool("search_reports").utf8, "a utf-8 page");
    let defaulted = id_of(&found, "defaulted");
    assert!(worker
        .execution_binding(&defaulted, &args(r#"{"q":"d"}"#))
        .is_err());
    for (name, query) in [
        ("defaulted", r#"{"q":"filled","year":"2024"}"#),
        ("browser_names", r#"{"q":"named"}"#),
        ("odd_button", r#"{"q":"odd"}"#),
    ] {
        let found = open(&mut worker);
        let id = id_of(&found, name);
        assert!(
            found
                .tools
                .iter()
                .find(|t| t.tool_id == id)
                .unwrap()
                .self_contained,
            "{name}"
        );
        let (target, mut approval) = approved_execution(&worker, &found, &id, &args(query));
        worker
            .execute_tool(&target, &mut approval, &args(query), 1, "workspace")
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }

    // A result too large for one frame is kept in part and marked
    // incomplete, and the lifetime goes on.
    let found = open(&mut worker);
    let big = id_of(&found, "big_result");
    let (target, mut approval) = approved_execution(&worker, &found, &big, &args(r#"{"q":"big"}"#));
    let outcome = worker
        .execute_tool(
            &target,
            &mut approval,
            &args(r#"{"q":"big"}"#),
            1,
            "workspace",
        )
        .expect("big result");
    assert!(!outcome.structured_complete);
    assert!(
        outcome.structured.len() < 8,
        "{} blocks",
        outcome.structured.len()
    );
    worker.ping("after-big").expect("the lifetime survived");

    // A tool whose declaration changed after discovery is refused (fresh
    // discovery before dispatch), and its approval is unspent.
    let found = open(&mut worker);
    let mutating = id_of(&found, "mutating");
    let (target, mut approval) =
        approved_execution(&worker, &found, &mutating, &args(r#"{"q":"m"}"#));
    pump(&mut worker, Duration::from_secs(2));
    assert!(matches!(
        worker.execute_tool(&target, &mut approval, &args(r#"{"q":"m"}"#), 1, "workspace"),
        Err(HostError::Invalid(message)) if message.contains("changed")
    ));
    assert!(!approval.consumed);

    let report = worker.shutdown();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
    // Ground truth: what the fixture received.
    let hits = fixture.hits();
    assert!(
        hits.iter()
            .any(|h| h.ends_with("/search?q=rust&year=2025&exact=on&csrf=csrf-123")),
        "{hits:?}"
    );
    assert!(
        hits.iter()
            .any(|h| h.ends_with("/search?csrf=PAGETOKEN&q=tokened")),
        "{hits:?}"
    );
    for submitted in [
        "/search?q=filled&year=2024",
        "/search?q=named&q.dir=ltr&img.x=",
        "/search?q=odd&go=1",
    ] {
        assert!(
            hits.iter().any(|h| h.contains(submitted)),
            "{submitted} missing: {hits:?}"
        );
    }
    for never in [
        "other.test",
        "q=evil",
        "q=swallowed",
        "/elsewhere",
        "q=moved",
        "PAGEDEFAULT",
        "OTHERFORM",
        "q=evil-rewrite",
        "SCRIPTADDED",
        "PAGEYEAR",
        "caf%",
    ] {
        assert!(
            !hits.iter().any(|h| h.contains(never)),
            "{never} was requested: {hits:?}"
        );
    }
    assert!(
        !hits.iter().any(|h| h.ends_with("/search?q=m")),
        "the changed tool was submitted: {hits:?}"
    );
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn a_stalled_cancelled_or_killed_download_is_typed_and_commits_nothing() {
    let fixture = HttpsFixture::start();
    let approved = temp_profile("approved-outcomes");
    let launch_one = |label: &str| {
        let profile = temp_profile(label);
        let mut launch = spec(&profile, true);
        launch.destinations =
            vec![
                AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST))
                    .expect("fixture.test"),
            ];
        launch.test_trusted_spki = Some(fixture.spki.clone());
        let worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");
        (worker, profile)
    };
    let scope = crate::observation::ObservationScope {
        workspace: "workspace".into(),
        profile_identity: "profile".into(),
    };
    let stalled_link = |worker: &mut BrowserWorker| {
        worker
            .navigate("https://fixture.test/files")
            .expect("files");
        let live = worker.observe(&scope).expect("observe files");
        let node = live
            .observation
            .nodes
            .iter()
            .flat_map(|n| std::iter::once(n).chain(n.children.iter()))
            .flat_map(|n| std::iter::once(n).chain(n.children.iter()))
            .find(|n| {
                worker
                    .host_node(&n.node_id)
                    .is_some_and(|h| h.name == "Stalled 2026-01-05")
            })
            .map(|n| n.node_id.clone())
            .expect("stalled link");
        (live, node)
    };
    let stall_requests = |hits: &std::sync::Mutex<Vec<String>>| {
        hits.lock()
            .expect("hits")
            .iter()
            .filter(|hit| hit.contains("/files/stall.pdf"))
            .count()
    };
    // Fire `act` once the fixture has seen stall request number `nth`, so
    // the download is provably in flight (no fixed sleep).
    let when_stalled = |nth: usize, act: Box<dyn FnOnce() + Send>| {
        let hits = std::sync::Arc::clone(&fixture.hits);
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            while stall_requests(&hits) < nth && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            let reached = stall_requests(&hits) >= nth;
            std::thread::sleep(Duration::from_millis(500));
            act();
            reached
        })
    };
    let no_residue = |worker: &BrowserWorker, out: &str| {
        assert!(!approved.join(out).exists(), "{out} was committed");
        let staged = std::fs::read_dir(worker.staging_dir())
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(staged, 0, "a staging file was left behind");
    };

    // Timeout: the body never ends. The request went out, the worker gives
    // up at its 30 s capture deadline (inside the host's own) and reports a
    // typed timeout; the lifetime stays usable.
    let (mut worker, profile) = launch_one("download-outcomes");
    let (live, node) = stalled_link(&mut worker);
    let (target, mut approval) = approved_download(&worker, &live, &node, &approved, "stall.pdf");
    let started = Instant::now();
    let outcome = worker.download(
        &target,
        &mut approval,
        1,
        "workspace",
        &approved,
        "stall.pdf",
    );
    let elapsed = started.elapsed();
    assert!(
        matches!(
            outcome,
            Err(HostError::Unavailable(UnavailableReason::TimedOut))
        ),
        "{outcome:?}"
    );
    assert_eq!(
        stall_requests(&fixture.hits),
        1,
        "the stalled document was never requested"
    );
    assert!(
        elapsed >= Duration::from_secs(29) && elapsed < WORKER_NAVIGATION_TIMEOUT,
        "the timeout came from somewhere else: {elapsed:?}"
    );
    assert!(
        approval.consumed,
        "a dispatched download must spend its approval"
    );
    no_residue(&worker, "stall.pdf");
    worker
        .ping("after-timeout")
        .expect("ping after a timed-out download");

    // Cancellation: another thread cancels while the download is in flight.
    // The outcome is typed, the whole job is terminated, nothing is
    // committed, and every later call reports the cancellation.
    let (live, node) = stalled_link(&mut worker);
    let (target, mut approval) = approved_download(&worker, &live, &node, &approved, "cancel.pdf");
    let pids = job_pids(&worker);
    let handle = worker.cancel_handle();
    let canceller = when_stalled(
        2,
        Box::new(move || assert!(handle.cancel(), "nothing to cancel")),
    );
    let outcome = worker.download(
        &target,
        &mut approval,
        1,
        "workspace",
        &approved,
        "cancel.pdf",
    );
    assert!(
        canceller.join().expect("canceller thread"),
        "the download never reached the network"
    );
    assert!(
        matches!(
            outcome,
            Err(HostError::Unavailable(UnavailableReason::Cancelled))
        ),
        "{outcome:?}"
    );
    assert!(
        approval.consumed,
        "a dispatched download must spend its approval"
    );
    no_residue(&worker, "cancel.pdf");
    assert!(
        all_dead(&pids, Duration::from_secs(10)),
        "a member outlived the cancellation"
    );
    assert!(matches!(
        worker.ping("after-cancel"),
        Err(HostError::Unavailable(UnavailableReason::Cancelled))
    ));
    let (unspent, mut unspent_approval) = (target.clone(), approval.clone());
    unspent_approval.consumed = false;
    assert!(matches!(
        worker.download(
            &unspent,
            &mut unspent_approval,
            1,
            "workspace",
            &approved,
            "cancel.pdf"
        ),
        Err(HostError::Unavailable(UnavailableReason::Cancelled))
    ));
    assert!(
        !unspent_approval.consumed,
        "an ended lifetime spent an approval"
    );
    let staging = worker.staging_dir().to_path_buf();
    let report = worker.kill();
    assert!(
        report.job_empty,
        "the job did not empty after the cancellation"
    );
    assert!(
        !staging.exists(),
        "the staging directory outlived the lifetime"
    );
    assert_profile_gone(report.profile_removed, &profile);

    // Crash: the worker is killed while a download is in flight. The outcome
    // is typed, nothing is committed, and the lifetime is over.
    let (mut worker, profile) = launch_one("download-crash");
    let (live, node) = stalled_link(&mut worker);
    let (target, mut approval) = approved_download(&worker, &live, &node, &approved, "crash.pdf");
    let pids = job_pids(&worker);
    let victim = worker.worker_pid();
    let killer = when_stalled(3, Box::new(move || terminate(victim)));
    let outcome = worker.download(
        &target,
        &mut approval,
        1,
        "workspace",
        &approved,
        "crash.pdf",
    );
    assert!(
        killer.join().expect("killer thread"),
        "the download never reached the network"
    );
    assert!(
        matches!(
            outcome,
            Err(HostError::Unavailable(UnavailableReason::WorkerExited))
        ),
        "{outcome:?}"
    );
    assert!(
        approval.consumed,
        "a dispatched download must spend its approval"
    );
    no_residue(&worker, "crash.pdf");
    assert!(
        all_dead(&pids, Duration::from_secs(10)),
        "a member outlived the crash"
    );
    assert!(matches!(
        worker.navigate("https://fixture.test/files"),
        Err(HostError::Unavailable(UnavailableReason::WorkerExited))
    ));
    let staging = worker.staging_dir().to_path_buf();
    let report = worker.kill();
    assert!(report.job_empty, "the job did not empty after the crash");
    assert!(
        !staging.exists(),
        "the staging directory outlived the lifetime"
    );
    assert_profile_gone(report.profile_removed, &profile);
    let _ = std::fs::remove_dir_all(&approved);
}

#[test]
#[ignore = "requires the built browser worker and Edge; run by npm run browser-launcher:native"]
fn the_journey_downloads_the_latest_report_create_only_and_verified() {
    let fixture = HttpsFixture::start();
    let profile = temp_profile("download");
    let approved = temp_profile("approved-folder");
    let mut launch = spec(&profile, true);
    launch.destinations =
        vec![
            AdmittedDestination::fixture("fixture.test", IpAddr::V4(Ipv4Addr::LOCALHOST))
                .expect("fixture.test"),
        ];
    launch.test_trusted_spki = Some(fixture.spki.clone());
    let mut worker = launch_worker(&launch, &Policy(PolicyKeyState::Absent)).expect("launch");
    let scope = crate::observation::ObservationScope {
        workspace: "workspace".into(),
        profile_identity: "profile".into(),
    };

    // Open the controlled site and let the host select the latest report.
    worker
        .navigate("https://fixture.test/reports")
        .expect("reports");
    let live = worker.observe(&scope).expect("observe");
    let latest = worker.select_latest_dated_link().expect("latest report");
    assert_eq!(
        worker
            .host_node(&latest)
            .and_then(|n| n.url.clone())
            .as_deref(),
        Some("/reports/2026-09-30.pdf")
    );
    // Path traversal is refused before anything is dispatched.
    let (target, mut approval) =
        approved_download(&worker, &live, &latest, &approved, "../escape.pdf");
    assert!(worker
        .download(
            &target,
            &mut approval,
            1,
            "workspace",
            &approved,
            "../escape.pdf"
        )
        .is_err());
    assert!(
        !approval.consumed,
        "a refused destination consumed the approval"
    );
    // A click approval is not a download approval, and a download approval
    // names one folder and file: neither is spent on a mismatch.
    let (click, mut click_approval) = approved_click(&worker, &live, &latest);
    assert!(worker
        .download(
            &click,
            &mut click_approval,
            1,
            "workspace",
            &approved,
            "reports/latest.pdf"
        )
        .is_err());
    assert!(
        !click_approval.consumed,
        "a click approval was spent on a download"
    );
    let (target, mut approval) =
        approved_download(&worker, &live, &latest, &approved, "reports/other.pdf");
    assert!(worker
        .download(
            &target,
            &mut approval,
            1,
            "workspace",
            &approved,
            "reports/latest.pdf"
        )
        .is_err());
    assert!(!approval.consumed, "an approval for another file was spent");
    // The approved download, committed create-only.
    let (target, mut approval) =
        approved_download(&worker, &live, &latest, &approved, "reports/latest.pdf");
    let committed = worker
        .download(
            &target,
            &mut approval,
            1,
            "workspace",
            &approved,
            "reports/latest.pdf",
        )
        .expect("approved download");
    assert!(approval.consumed);
    assert_eq!(committed.path, approved.join("reports").join("latest.pdf"));
    // Independent check: the exact fixture bytes and their SHA-256.
    let expected = b"%PDF-1.4\n% /reports/2026-09-30.pdf\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF\n";
    let on_disk = std::fs::read(&committed.path).expect("read committed file");
    assert_eq!(on_disk, expected.to_vec(), "committed bytes differ");
    assert_eq!(committed.sha256, sha256_hex(expected));
    assert_eq!(committed.bytes, expected.len() as u64);
    assert_eq!(
        committed.source_url,
        "https://fixture.test/reports/2026-09-30.pdf"
    );

    // An existing file is never replaced.
    // The page now shows the inert replacement of the PDF; return to the list.
    worker
        .navigate("https://fixture.test/reports")
        .expect("reports again");
    let live = worker.observe(&scope).expect("observe again");
    let latest = worker.select_latest_dated_link().expect("latest");
    let (target, mut again) =
        approved_download(&worker, &live, &latest, &approved, "reports/latest.pdf");
    assert!(matches!(
        worker.download(&target, &mut again, 1, "workspace", &approved, "reports/latest.pdf"),
        Err(HostError::Invalid(message)) if message.contains("create-only")
    ));
    assert_eq!(std::fs::read(&committed.path).unwrap(), expected.to_vec());

    // An oversized file and an executable disguised as a PDF are refused,
    // and nothing is committed for them.
    // The type check uses the name actually written: a PDF-headed polyglot
    // is never committed as a script or a page, and a bidi override cannot
    // disguise the extension.
    // The same polyglot under a `.pdf` name is an admitted inert PDF: the
    // control that the refusals below come from the name rule.
    for (name, out, refusal) in [
        ("Oversized 2026-01-01", "oversized.pdf", None),
        ("Disguised 2026-01-02", "fake.pdf", None),
        (
            "Polyglot 2026-01-03",
            "evil.cmd",
            Some("filename type denied"),
        ),
        (
            "Polyglot 2026-01-03",
            "evil.html",
            Some("declared type denied"),
        ),
        (
            "Polyglot 2026-01-03",
            "a\u{202e}fdp.cmd",
            Some("format characters"),
        ),
        ("Fragment 2026-01-04", "frag.pdf", Some("fragment")),
        ("Polyglot 2026-01-03", "poly.pdf", Some("")),
    ] {
        // Each refused attempt leaves the page on the replaced document: the
        // host requires a fresh navigation (stale), so start from the list.
        worker
            .navigate("https://fixture.test/files")
            .expect("files");
        let live = worker.observe(&scope).expect("observe files");
        let node = live
            .observation
            .nodes
            .iter()
            .flat_map(|n| std::iter::once(n).chain(n.children.iter()))
            .flat_map(|n| std::iter::once(n).chain(n.children.iter()))
            .find(|n| worker.host_node(&n.node_id).is_some_and(|h| h.name == name))
            .map(|n| n.node_id.clone())
            .expect("file link");
        let (target, mut approval) = approved_download(&worker, &live, &node, &approved, out);
        let result = worker.download(&target, &mut approval, 1, "workspace", &approved, out);
        if refusal == Some("") {
            let committed = result.expect("the polyglot as an inert PDF");
            assert!(std::fs::read(&committed.path)
                .unwrap()
                .starts_with(b"%PDF-1.4\r\n@echo POLYGLOT"));
            continue;
        }
        match (&result, refusal) {
            (Err(HostError::Invalid(message)), Some(expected)) => {
                assert!(message.contains(expected), "{out}: {message}")
            }
            (Err(_), None) => {}
            (other, _) => assert!(other.is_err() && refusal.is_none(), "{out}: {other:?}"),
        }
        assert!(!approved.join(out).exists(), "{name} left a file as {out}");
        if refusal == Some("fragment") {
            assert!(!approval.consumed, "a fragment link spent its approval");
        }
    }
    let report = worker.shutdown();
    assert!(report.job_empty, "the job did not empty");
    assert_profile_gone(report.profile_removed, &profile);
    let _ = std::fs::remove_dir_all(&approved);
}
