//! Windows process primitives for the supervisor: verified process identity,
//! kill-on-close Job Objects, suspended launch with job assignment before
//! resume, and a per-install named stop event.

use crate::LifecycleError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr;
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, FILETIME, HANDLE, STILL_ACTIVE, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetExitCodeProcess, GetProcessTimes, OpenEventW, OpenProcess, OpenThread,
    QueryFullProcessImageNameW, ResumeThread, SetEvent, TerminateProcess, WaitForMultipleObjects,
    EVENT_MODIFY_STATE, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE, THREAD_SUSPEND_RESUME,
};

/// An owned kernel handle closed on drop.
pub struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE) -> Option<Self> {
        (handle != 0 && handle != -1).then_some(Self(handle))
    }
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by a successful open/create call and
        // is closed exactly once here.
        unsafe { CloseHandle(self.0) };
    }
}

fn last_error(context: &str) -> LifecycleError {
    LifecycleError::platform(format!("{context}: {}", std::io::Error::last_os_error()))
}

fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(Some(0)).collect()
}

/// A process identified by pid, creation time, and image path, so a reused
/// pid is never mistaken for the recorded process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub creation_time: u64,
    pub image: String,
}

/// An opened, identity-verified, still-running process.
pub struct LiveProcess {
    handle: OwnedHandle,
    pub identity: ProcessIdentity,
}

fn open_process(pid: u32, access: u32) -> Option<OwnedHandle> {
    // SAFETY: OpenProcess has no memory-safety preconditions; the returned
    // handle is owned by OwnedHandle.
    OwnedHandle::new(unsafe { OpenProcess(access, 0, pid) })
}

fn identity_of(handle: &OwnedHandle, pid: u32) -> Option<ProcessIdentity> {
    let mut exit_code = 0u32;
    // SAFETY: valid handle and out-pointer to a local.
    if unsafe { GetExitCodeProcess(handle.raw(), &mut exit_code) } == 0
        || exit_code != STILL_ACTIVE as u32
    {
        return None;
    }
    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut unused = [creation, creation, creation];
    // SAFETY: valid handle and out-pointers to locals.
    let ok = unsafe {
        GetProcessTimes(
            handle.raw(),
            &mut creation,
            &mut unused[0],
            &mut unused[1],
            &mut unused[2],
        )
    };
    if ok == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32_768];
    let mut size = buffer.len() as u32;
    // SAFETY: buffer length is passed in `size`.
    if unsafe {
        QueryFullProcessImageNameW(
            handle.raw(),
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &mut size,
        )
    } == 0
    {
        return None;
    }
    Some(ProcessIdentity {
        pid,
        creation_time: (u64::from(creation.dwHighDateTime) << 32)
            | u64::from(creation.dwLowDateTime),
        image: String::from_utf16_lossy(&buffer[..size as usize]),
    })
}

/// Returns the identity of a running process, or `None` if it is not running
/// or cannot be inspected.
pub fn identify(pid: u32) -> Option<ProcessIdentity> {
    let handle = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    identity_of(&handle, pid)
}

/// Opens the recorded process only if it is still running with the exact
/// recorded creation time and image path.
pub fn open_verified(recorded: &ProcessIdentity) -> Option<LiveProcess> {
    let handle = open_process(
        recorded.pid,
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
    )?;
    let identity = identity_of(&handle, recorded.pid)?;
    (identity.creation_time == recorded.creation_time
        && identity.image.eq_ignore_ascii_case(&recorded.image))
    .then_some(LiveProcess { handle, identity })
}

impl LiveProcess {
    /// Waits until the process exits; returns true only if exit was observed.
    pub fn wait(&self, timeout: Duration) -> bool {
        wait_any(&[self.handle.raw()], timeout) == Some(0)
    }

    pub fn terminate(&self) -> bool {
        // SAFETY: handle opened with PROCESS_TERMINATE.
        unsafe { TerminateProcess(self.handle.raw(), 1) != 0 }
    }
}

/// Waits for any handle; returns its index, or `None` on timeout or failure.
pub fn wait_any(handles: &[HANDLE], timeout: Duration) -> Option<usize> {
    let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
    // SAFETY: the slice pointer and length describe valid handles.
    let result =
        unsafe { WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, millis) };
    if result == WAIT_TIMEOUT {
        return None;
    }
    let index = result.wrapping_sub(WAIT_OBJECT_0) as usize;
    (index < handles.len()).then_some(index)
}

pub fn child_handle(child: &std::process::Child) -> HANDLE {
    child.as_raw_handle() as HANDLE
}

/// A Job Object that terminates every assigned process when its last handle
/// closes, including when the supervisor itself dies.
pub struct Job(OwnedHandle);

impl Job {
    pub fn kill_on_close() -> Result<Self, LifecycleError> {
        // SAFETY: null attributes and name are permitted.
        let job = OwnedHandle::new(unsafe { CreateJobObjectW(ptr::null(), ptr::null()) })
            .ok_or_else(|| last_error("CreateJobObjectW"))?;
        // SAFETY: zeroed JOBOBJECT_EXTENDED_LIMIT_INFORMATION is a valid value.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: pointer and size describe `limits`.
        let ok = unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            return Err(last_error("SetInformationJobObject"));
        }
        Ok(Self(job))
    }

    pub fn assign(&self, process: HANDLE) -> Result<(), LifecycleError> {
        // SAFETY: both handles are valid for the duration of the call.
        if unsafe { AssignProcessToJobObject(self.0.raw(), process) } == 0 {
            return Err(last_error("AssignProcessToJobObject"));
        }
        Ok(())
    }

    pub fn terminate(&self) -> bool {
        // SAFETY: valid job handle.
        unsafe { TerminateJobObject(self.0.raw(), 1) != 0 }
    }

    pub fn active_processes(&self) -> Result<u32, LifecycleError> {
        // SAFETY: zeroed accounting information is a valid value.
        let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: pointer and size describe `info`.
        let ok = unsafe {
            QueryInformationJobObject(
                self.0.raw(),
                JobObjectBasicAccountingInformation,
                &mut info as *mut _ as *mut c_void,
                std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(last_error("QueryInformationJobObject"));
        }
        Ok(info.ActiveProcesses)
    }

    /// Terminates the job and waits until no process remains in it.
    pub fn terminate_and_confirm_empty(&self, timeout: Duration) -> Result<bool, LifecycleError> {
        self.terminate();
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if self.active_processes()? == 0 {
                return Ok(true);
            }
            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Resumes every thread of a process created with `CREATE_SUSPENDED`.
pub fn resume_process(pid: u32) -> Result<(), LifecycleError> {
    // SAFETY: snapshot flags and pid have no memory-safety preconditions.
    let snapshot = OwnedHandle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })
        .ok_or_else(|| last_error("CreateToolhelp32Snapshot"))?;
    // SAFETY: zeroed THREADENTRY32 with dwSize set is the documented input.
    let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
    let mut resumed = 0;
    // SAFETY: valid snapshot handle and entry pointer.
    let mut more = unsafe { Thread32First(snapshot.raw(), &mut entry) } != 0;
    while more {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: OpenThread has no memory-safety preconditions.
            if let Some(thread) = OwnedHandle::new(unsafe {
                OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID)
            }) {
                // SAFETY: handle opened with THREAD_SUSPEND_RESUME.
                if unsafe { ResumeThread(thread.raw()) } != u32::MAX {
                    resumed += 1;
                }
            }
        }
        // SAFETY: valid snapshot handle and entry pointer.
        more = unsafe { Thread32Next(snapshot.raw(), &mut entry) } != 0;
    }
    if resumed == 0 {
        return Err(LifecycleError::platform(
            "the suspended tunnel client could not be resumed",
        ));
    }
    Ok(())
}

/// Name of the per-install stop event in the session-local namespace.
pub fn stop_event_name(root: &Path) -> String {
    let digest = Sha256::digest(root.to_string_lossy().to_ascii_lowercase().as_bytes());
    let hex = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("Local\\Cotra-Stop-{hex}")
}

/// The supervisor's manual-reset stop event. Creation fails if another
/// supervisor for the same install already owns it.
pub struct StopEvent(OwnedHandle);

impl StopEvent {
    pub fn create(name: &str) -> Result<Self, LifecycleError> {
        let name = wide(std::ffi::OsStr::new(name));
        // SAFETY: null attributes are permitted; the name is NUL-terminated.
        let handle = unsafe { CreateEventW(ptr::null(), 1, 0, name.as_ptr()) };
        // SAFETY: GetLastError has no preconditions.
        let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let handle = OwnedHandle::new(handle).ok_or_else(|| last_error("CreateEventW"))?;
        if already {
            return Err(LifecycleError::conflict(
                "another Cotra supervisor for this install is already running",
            ));
        }
        Ok(Self(handle))
    }

    pub fn raw(&self) -> HANDLE {
        self.0.raw()
    }

    /// Signals an existing stop event; returns false if none exists.
    pub fn signal(name: &str) -> bool {
        let name = wide(std::ffi::OsStr::new(name));
        // SAFETY: the name is NUL-terminated.
        match OwnedHandle::new(unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr()) }) {
            // SAFETY: handle opened with EVENT_MODIFY_STATE.
            Some(event) => unsafe { SetEvent(event.raw()) != 0 },
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;

    fn ping() -> Command {
        let mut command = Command::new(
            std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
                .join("System32")
                .join("ping.exe"),
        );
        command
            .args(["-n", "30", "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    #[test]
    fn identity_detects_pid_reuse_shape_and_exit() {
        let mut child = ping().spawn().unwrap();
        let identity = identify(child.id()).unwrap();
        assert!(identity.image.to_ascii_lowercase().ends_with("ping.exe"));
        assert!(open_verified(&identity).is_some());
        let mut forged = identity.clone();
        forged.creation_time += 1;
        assert!(open_verified(&forged).is_none());
        let live = open_verified(&identity).unwrap();
        assert!(live.terminate());
        assert!(live.wait(Duration::from_secs(10)));
        let _ = child.wait();
        assert!(identify(identity.pid).is_none() || open_verified(&identity).is_none());
    }

    #[test]
    fn suspended_child_is_jobbed_before_resume_and_job_termination_empties_it() {
        let job = Job::kill_on_close().unwrap();
        let mut child = ping().creation_flags(CREATE_SUSPENDED).spawn().unwrap();
        job.assign(child_handle(&child)).unwrap();
        resume_process(child.id()).unwrap();
        assert_eq!(job.active_processes().unwrap(), 1);
        assert!(job
            .terminate_and_confirm_empty(Duration::from_secs(10))
            .unwrap());
        let _ = child.wait();
    }

    #[test]
    fn stop_event_is_exclusive_and_signalable() {
        let name = format!("Local\\Cotra-Test-{}", crate::nonce());
        assert!(!StopEvent::signal(&name));
        let event = StopEvent::create(&name).unwrap();
        assert!(StopEvent::create(&name).is_err());
        assert!(StopEvent::signal(&name));
        assert_eq!(wait_any(&[event.raw()], Duration::from_secs(1)), Some(0));
    }
}
