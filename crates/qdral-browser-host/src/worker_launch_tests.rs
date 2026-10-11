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
    // Control for the machine-wide residue check: it sees the live engine.
    assert!(
        processes_referencing(&profile).contains("msedge.exe"),
        "the residue query does not see the running engine"
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
