use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SECRETISH: &[&str] = &[
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "API_KEY",
    "PRIVATE_KEY",
    "TUNNEL_KEY",
];

const FORBIDDEN_QUNTAL_ENV: &[&str] = &[
    "QUNTAL_TUNNEL_ID",
    "QUNTAL_TUNNEL_CLIENT",
    "QUNTAL_TUNNEL_KEY_FILE",
    "QUNTAL_TUNNEL_HEALTH_URL_FILE",
    "QUNTAL_DAEMON",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionLimits {
    pub timeout: Duration,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            stdout_bytes: 2 * 1024 * 1024,
            stderr_bytes: 256 * 1024,
        }
    }
}

impl ExecutionLimits {
    pub fn validate(&self) -> Result<(), ExecutionPlanError> {
        if self.timeout.is_zero() || self.timeout > Duration::from_secs(30 * 60) {
            return Err(ExecutionPlanError::new(
                "timeout must be between 1 second and 30 minutes",
            ));
        }
        if self.stdout_bytes == 0 || self.stdout_bytes > 16 * 1024 * 1024 {
            return Err(ExecutionPlanError::new(
                "stdout limit must be between 1 byte and 16 MiB",
            ));
        }
        if self.stderr_bytes == 0 || self.stderr_bytes > 4 * 1024 * 1024 {
            return Err(ExecutionPlanError::new(
                "stderr limit must be between 1 byte and 4 MiB",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub executable: PathBuf,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub limits: ExecutionLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlanError {
    pub message: String,
}

impl ExecutionPlanError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub fn build_execution_plan(
    workspace_root: impl AsRef<Path>,
    executable: impl AsRef<Path>,
    argv: &[String],
    requested_cwd: impl AsRef<Path>,
    source_env: &BTreeMap<String, String>,
    limits: ExecutionLimits,
) -> Result<ExecutionPlan, ExecutionPlanError> {
    limits.validate()?;

    let workspace_root = canonical_dir(workspace_root.as_ref(), "workspace root")?;
    let executable = executable.as_ref();
    if !executable.is_absolute() {
        return Err(ExecutionPlanError::new(
            "execution executable must be an absolute path",
        ));
    }
    let executable = canonical_file(executable, "execution executable")?;

    for arg in argv {
        if arg.contains('\0') {
            return Err(ExecutionPlanError::new("argv contains a NUL byte"));
        }
    }

    let requested_cwd = requested_cwd.as_ref();
    let cwd = if requested_cwd.as_os_str().is_empty() || requested_cwd == Path::new(".") {
        workspace_root.clone()
    } else if requested_cwd.is_absolute() {
        canonical_dir(requested_cwd, "execution cwd")?
    } else {
        canonical_dir(&workspace_root.join(requested_cwd), "execution cwd")?
    };
    if cwd != workspace_root && !cwd.starts_with(&workspace_root) {
        return Err(ExecutionPlanError::new(
            "execution cwd resolves outside the trusted workspace",
        ));
    }

    Ok(ExecutionPlan {
        executable,
        argv: argv.to_vec(),
        cwd,
        env: sanitized_execution_env(source_env),
        limits,
    })
}

pub fn sanitized_execution_env(source: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (name, value) in source {
        let upper = name.to_ascii_uppercase();
        if SECRETISH.iter().any(|needle| upper.contains(needle)) {
            continue;
        }
        if FORBIDDEN_QUNTAL_ENV
            .iter()
            .any(|forbidden| upper == *forbidden)
        {
            continue;
        }
        if allowed_execution_env(name) {
            out.insert(name.clone(), value.clone());
        }
    }
    out
}

fn allowed_execution_env(name: &str) -> bool {
    matches!(
        name,
        "PATH"
            | "Path"
            | "PATHEXT"
            | "ComSpec"
            | "SystemRoot"
            | "WINDIR"
            | "TEMP"
            | "TMP"
            | "HOME"
            | "USERPROFILE"
            | "LOCALAPPDATA"
            | "APPDATA"
            | "PROGRAMDATA"
            | "ProgramFiles"
            | "ProgramFiles(x86)"
            | "LANG"
            | "LC_ALL"
    )
}

fn canonical_dir(path: &Path, label: &str) -> Result<PathBuf, ExecutionPlanError> {
    let resolved = fs::canonicalize(path)
        .map_err(|error| ExecutionPlanError::new(format!("resolve {label}: {error}")))?;
    if !resolved.is_dir() {
        return Err(ExecutionPlanError::new(format!(
            "{label} is not a directory"
        )));
    }
    Ok(resolved)
}

fn canonical_file(path: &Path, label: &str) -> Result<PathBuf, ExecutionPlanError> {
    let resolved = fs::canonicalize(path)
        .map_err(|error| ExecutionPlanError::new(format!("resolve {label}: {error}")))?;
    if !resolved.is_file() {
        return Err(ExecutionPlanError::new(format!("{label} is not a file")));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("quntal-process-{name}-{suffix}"));
        fs::create_dir_all(&root).expect("create root");
        root
    }

    fn executable(root: &Path) -> PathBuf {
        let path = root.join("tool.bin");
        fs::write(&path, b"test").expect("seed executable");
        path
    }

    #[test]
    fn creates_bounded_plan_inside_workspace() {
        let workspace = root("ok");
        let work = workspace.join("project");
        fs::create_dir_all(&work).unwrap();
        let exe = executable(&workspace);
        let env = BTreeMap::from([
            ("PATH".to_owned(), "bin".to_owned()),
            ("OPENAI_API_KEY".to_owned(), "secret".to_owned()),
            (
                "QUNTAL_TUNNEL_KEY_FILE".to_owned(),
                "secret-path".to_owned(),
            ),
        ]);

        let plan = build_execution_plan(
            &workspace,
            &exe,
            &["--version".into()],
            "project",
            &env,
            ExecutionLimits::default(),
        )
        .unwrap();

        assert_eq!(plan.cwd, fs::canonicalize(work).unwrap());
        assert_eq!(plan.env.get("PATH").map(String::as_str), Some("bin"));
        assert!(!plan.env.contains_key("OPENAI_API_KEY"));
        assert!(!plan.env.contains_key("QUNTAL_TUNNEL_KEY_FILE"));
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn rejects_relative_executable() {
        let workspace = root("relative-exe");
        let error = build_execution_plan(
            &workspace,
            "tool.exe",
            &[],
            ".",
            &BTreeMap::new(),
            ExecutionLimits::default(),
        )
        .unwrap_err();
        assert!(error.message.contains("absolute path"));
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn rejects_cwd_escape() {
        let workspace = root("workspace");
        let outside = root("outside");
        let exe = executable(&workspace);
        let error = build_execution_plan(
            &workspace,
            &exe,
            &[],
            &outside,
            &BTreeMap::new(),
            ExecutionLimits::default(),
        )
        .unwrap_err();
        assert!(error.message.contains("outside the trusted workspace"));
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn rejects_nul_argv() {
        let workspace = root("nul");
        let exe = executable(&workspace);
        let error = build_execution_plan(
            &workspace,
            &exe,
            &["bad\0arg".into()],
            ".",
            &BTreeMap::new(),
            ExecutionLimits::default(),
        )
        .unwrap_err();
        assert!(error.message.contains("NUL"));
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn validates_appcontainer_profile_names() {
        assert!(validate_appcontainer_name("Quntal.Probe.123").is_ok());
        assert!(validate_appcontainer_name("").is_err());
        assert!(validate_appcontainer_name("Quntal/Probe").is_err());
        assert!(validate_appcontainer_name(&"a".repeat(65)).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_appcontainer_profile_lifecycle_is_available() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let name = format!("Quntal.Probe.{}.{}", std::process::id(), suffix);
        let result = probe_appcontainer_profile(&name).expect("AppContainer lifecycle");
        assert!(result.profile_created);
        assert!(result.sid_derived);
        assert!(result.profile_deleted);
    }

    #[test]
    fn private_qualification_plan_rejects_non_allowlisted_environment() {
        let workspace = root("private-env");
        let exe = executable(&workspace);
        let mut source = BTreeMap::new();
        source.insert("PATH".to_owned(), "safe".to_owned());
        source.insert("QUNTAL_TUNNEL_KEY_FILE".to_owned(), "secret".to_owned());
        source.insert("API_KEY".to_owned(), "secret".to_owned());
        let plan = build_execution_plan(
            &workspace,
            &exe,
            &[],
            ".",
            &source,
            ExecutionLimits::default(),
        )
        .expect("plan");
        assert!(!plan
            .env
            .keys()
            .any(|key| key.contains("QUNTAL") || key.contains("API")));
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn private_qualification_plan_rejects_nul_and_parent_cwd() {
        let workspace = root("private-invalid");
        let exe = executable(&workspace);
        assert!(build_execution_plan(
            &workspace,
            &exe,
            &["bad\0arg".into()],
            ".",
            &BTreeMap::new(),
            ExecutionLimits::default(),
        )
        .is_err());
        assert!(build_execution_plan(
            &workspace,
            &exe,
            &[],
            "..",
            &BTreeMap::new(),
            ExecutionLimits::default(),
        )
        .is_err());
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn rejects_unbounded_limits() {
        let workspace = root("limits");
        let exe = executable(&workspace);
        let error = build_execution_plan(
            &workspace,
            &exe,
            &[],
            ".",
            &BTreeMap::new(),
            ExecutionLimits {
                timeout: Duration::from_secs(31 * 60),
                stdout_bytes: 1,
                stderr_bytes: 1,
            },
        )
        .unwrap_err();
        assert!(error.message.contains("30 minutes"));
        let _ = fs::remove_dir_all(workspace);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppContainerProbe {
    pub profile_created: bool,
    pub sid_derived: bool,
    pub profile_deleted: bool,
}

pub fn validate_appcontainer_name(name: &str) -> Result<(), ExecutionPlanError> {
    if name.is_empty() || name.encode_utf16().count() > 64 {
        return Err(ExecutionPlanError::new(
            "AppContainer name must contain 1..=64 UTF-16 code units",
        ));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ' '))
    {
        return Err(ExecutionPlanError::new(
            "AppContainer name contains a character outside the Windows allowed set",
        ));
    }
    Ok(())
}

#[cfg(windows)]
pub fn probe_appcontainer_profile(name: &str) -> Result<AppContainerProbe, ExecutionPlanError> {
    windows_appcontainer::probe(name)
}

#[cfg(not(windows))]
pub fn probe_appcontainer_profile(_name: &str) -> Result<AppContainerProbe, ExecutionPlanError> {
    Err(ExecutionPlanError::new(
        "AppContainer probing is available only on Windows",
    ))
}

#[cfg(windows)]
mod windows_appcontainer {
    use super::{validate_appcontainer_name, AppContainerProbe, ExecutionPlanError};
    use core::ffi::c_void;
    use std::ptr;

    type Hresult = i32;
    type Psid = *mut c_void;

    #[repr(C)]
    struct SidAndAttributes {
        sid: Psid,
        attributes: u32,
    }

    #[link(name = "userenv")]
    unsafe extern "system" {
        fn CreateAppContainerProfile(
            app_container_name: *const u16,
            display_name: *const u16,
            description: *const u16,
            capabilities: *const SidAndAttributes,
            capability_count: u32,
            app_container_sid: *mut Psid,
        ) -> Hresult;

        fn DeriveAppContainerSidFromAppContainerName(
            app_container_name: *const u16,
            app_container_sid: *mut Psid,
        ) -> Hresult;

        fn DeleteAppContainerProfile(app_container_name: *const u16) -> Hresult;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn FreeSid(sid: Psid) -> *mut c_void;
    }

    pub(super) fn probe(name: &str) -> Result<AppContainerProbe, ExecutionPlanError> {
        validate_appcontainer_name(name)?;
        let name = wide(name);
        let display = wide("Quntal execution probe");
        let description = wide("Temporary Quntal AppContainer capability probe");

        let mut created_sid: Psid = ptr::null_mut();
        let create_hr = unsafe {
            CreateAppContainerProfile(
                name.as_ptr(),
                display.as_ptr(),
                description.as_ptr(),
                ptr::null(),
                0,
                &mut created_sid,
            )
        };
        if create_hr != 0 {
            return Err(hresult_error("CreateAppContainerProfile", create_hr));
        }

        if !created_sid.is_null() {
            unsafe {
                FreeSid(created_sid);
            }
        }

        let mut derived_sid: Psid = ptr::null_mut();
        let derive_hr =
            unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut derived_sid) };
        if derive_hr != 0 {
            let _ = unsafe { DeleteAppContainerProfile(name.as_ptr()) };
            return Err(hresult_error(
                "DeriveAppContainerSidFromAppContainerName",
                derive_hr,
            ));
        }

        if !derived_sid.is_null() {
            unsafe {
                FreeSid(derived_sid);
            }
        }

        let delete_hr = unsafe { DeleteAppContainerProfile(name.as_ptr()) };
        if delete_hr != 0 {
            let _ = unsafe { DeleteAppContainerProfile(name.as_ptr()) };
            return Err(hresult_error("DeleteAppContainerProfile", delete_hr));
        }

        Ok(AppContainerProbe {
            profile_created: true,
            sid_derived: true,
            profile_deleted: true,
        })
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn hresult_error(operation: &str, code: Hresult) -> ExecutionPlanError {
        ExecutionPlanError::new(format!(
            "{operation} failed with HRESULT 0x{:08X}",
            code as u32
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobObjectProbe {
    pub job_created: bool,
    pub kill_on_close_set: bool,
    pub handle_closed: bool,
}

#[cfg(windows)]
pub fn probe_job_object_kill_on_close() -> Result<JobObjectProbe, ExecutionPlanError> {
    windows_job_object::probe()
}

#[cfg(not(windows))]
pub fn probe_job_object_kill_on_close() -> Result<JobObjectProbe, ExecutionPlanError> {
    Err(ExecutionPlanError::new(
        "Job Object probing is available only on Windows",
    ))
}

#[cfg(windows)]
mod windows_job_object {
    use super::{ExecutionPlanError, JobObjectProbe};
    use core::ffi::c_void;
    use std::mem;
    use std::ptr;

    type Handle = *mut c_void;

    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct JobObjectBasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct JobObjectExtendedLimitInformation {
        basic_limit_information: JobObjectBasicLimitInformation,
        io_info: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(job_attributes: *const c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(
            job: Handle,
            information_class: i32,
            information: *const c_void,
            information_length: u32,
        ) -> i32;
        fn CloseHandle(object: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    pub(super) fn probe() -> Result<JobObjectProbe, ExecutionPlanError> {
        let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if job.is_null() {
            return Err(last_error("CreateJobObjectW"));
        }

        let mut information = JobObjectExtendedLimitInformation::default();
        information.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let configured = unsafe {
            SetInformationJobObject(
                job,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                (&information as *const JobObjectExtendedLimitInformation).cast(),
                mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
            )
        };

        if configured == 0 {
            let error = last_error("SetInformationJobObject");
            unsafe {
                CloseHandle(job);
            }
            return Err(error);
        }

        let closed = unsafe { CloseHandle(job) };
        if closed == 0 {
            return Err(last_error("CloseHandle"));
        }

        Ok(JobObjectProbe {
            job_created: true,
            kill_on_close_set: true,
            handle_closed: true,
        })
    }

    fn last_error(operation: &str) -> ExecutionPlanError {
        let code = unsafe { GetLastError() };
        ExecutionPlanError::new(format!("{operation} failed with Win32 error {code}"))
    }
}

#[cfg(all(test, windows))]
mod job_object_tests {
    use super::*;

    #[test]
    fn windows_job_object_kill_on_close_is_available() {
        let result = probe_job_object_kill_on_close().expect("Job Object lifecycle");
        assert!(result.job_created);
        assert!(result.kill_on_close_set);
        assert!(result.handle_closed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainedLaunchProbe {
    pub profile_created: bool,
    pub process_created_suspended: bool,
    pub appcontainer_token_verified: bool,
    pub assigned_to_job_before_resume: bool,
    pub resumed: bool,
    pub child_completed: bool,
    pub exit_code: u32,
    pub profile_deleted: bool,
}

#[cfg(windows)]
pub fn probe_contained_appcontainer_job_launch(
    name: &str,
) -> Result<ContainedLaunchProbe, ExecutionPlanError> {
    windows_contained_launch::probe(name)
}

#[cfg(not(windows))]
pub fn probe_contained_appcontainer_job_launch(
    _name: &str,
) -> Result<ContainedLaunchProbe, ExecutionPlanError> {
    Err(ExecutionPlanError::new(
        "contained AppContainer launch probing is available only on Windows",
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateExecutionResult {
    pub exit_code: u32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub appcontainer_verified: bool,
    pub assigned_to_job_before_resume: bool,
    pub job_quiescent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateExecutionFailure {
    InvalidPlan(String),
    Provider(String),
    ProcessTimeout,
    OutputLimit(OutputStream),
    TerminationUnverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrivateExecutionMode {
    Public,
    Success,
    Timeout,
    StdoutLimit,
    StderrLimit,
    DescendantDrain,
}

#[cfg(windows)]
pub fn execute_contained(
    plan: ExecutionPlan,
    profile_name: &str,
) -> Result<PrivateExecutionResult, PrivateExecutionFailure> {
    qualify_private_execution_mode(plan, profile_name, PrivateExecutionMode::Public)
}

#[cfg(not(windows))]
pub fn execute_contained(
    _plan: ExecutionPlan,
    _profile_name: &str,
) -> Result<PrivateExecutionResult, PrivateExecutionFailure> {
    Err(PrivateExecutionFailure::Provider(
        "contained execution is available only on Windows".into(),
    ))
}

#[cfg(windows)]
pub fn qualify_private_execution(
    plan: ExecutionPlan,
    profile_name: &str,
) -> Result<PrivateExecutionResult, PrivateExecutionFailure> {
    qualify_private_execution_mode(plan, profile_name, PrivateExecutionMode::Success)
}

#[cfg(windows)]
pub(crate) fn qualify_private_execution_mode(
    plan: ExecutionPlan,
    profile_name: &str,
    mode: PrivateExecutionMode,
) -> Result<PrivateExecutionResult, PrivateExecutionFailure> {
    windows_contained_launch::qualify_private_execution(plan, profile_name, mode)
}

#[cfg(not(windows))]
pub fn qualify_private_execution(
    _plan: ExecutionPlan,
    _profile_name: &str,
) -> Result<PrivateExecutionResult, PrivateExecutionFailure> {
    Err(PrivateExecutionFailure::Provider(
        "private contained execution qualification is available only on Windows".into(),
    ))
}

#[cfg(windows)]
mod windows_contained_launch {
    use super::{
        allowed_execution_env, validate_appcontainer_name, ContainedLaunchProbe, ExecutionPlan,
        ExecutionPlanError, OutputStream, PrivateExecutionFailure, PrivateExecutionMode,
        PrivateExecutionResult, FORBIDDEN_QUNTAL_ENV, SECRETISH,
    };
    use core::ffi::c_void;
    use std::ffi::{OsStr, OsString};
    use std::mem;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use std::ptr;

    type Handle = *mut c_void;
    type Psid = *mut c_void;
    type Hresult = i32;

    const CREATE_SUSPENDED: u32 = 0x0000_0004;
    const CREATE_UNICODE_ENVIRONMENT: u32 = 0x0000_0400;
    const EXTENDED_STARTUPINFO_PRESENT: u32 = 0x0008_0000;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES: usize = 131_081;
    const PROC_THREAD_ATTRIBUTE_HANDLE_LIST: usize = 131_074;
    const GENERIC_READ: u32 = 0x8000_0000;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_IS_APP_CONTAINER: i32 = 29;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;
    const WAIT_FAILED: u32 = u32::MAX;
    const RESUME_FAILED: u32 = u32::MAX;
    const PROBE_TIMEOUT_MS: u32 = 15_000;
    const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
    const JOB_OBJECT_BASIC_ACCOUNTING_INFORMATION_CLASS: i32 = 1;
    const ERROR_BROKEN_PIPE: u32 = 109;
    const QUIESCENCE_POLL_MS: u64 = 10;
    const QUIESCENCE_TIMEOUT_MS: u64 = 5_000;
    const POST_EXIT_DRAIN_TIMEOUT_MS: u64 = 500;
    const POST_TERMINATION_DRAIN_TIMEOUT_MS: u64 = 1_000;

    #[repr(C)]
    struct SidAndAttributes {
        sid: Psid,
        attributes: u32,
    }

    #[repr(C)]
    struct SecurityAttributes {
        length: u32,
        security_descriptor: *mut c_void,
        inherit_handle: i32,
    }

    #[repr(C)]
    struct SecurityCapabilities {
        app_container_sid: Psid,
        capabilities: *mut SidAndAttributes,
        capability_count: u32,
        reserved: u32,
    }

    #[repr(C)]
    struct StartupInfoW {
        cb: u32,
        lp_reserved: *mut u16,
        lp_desktop: *mut u16,
        lp_title: *mut u16,
        dw_x: u32,
        dw_y: u32,
        dw_x_size: u32,
        dw_y_size: u32,
        dw_x_count_chars: u32,
        dw_y_count_chars: u32,
        dw_fill_attribute: u32,
        dw_flags: u32,
        w_show_window: u16,
        cb_reserved_2: u16,
        lp_reserved_2: *mut u8,
        h_std_input: Handle,
        h_std_output: Handle,
        h_std_error: Handle,
    }

    #[repr(C)]
    struct StartupInfoExW {
        startup_info: StartupInfoW,
        attribute_list: *mut c_void,
    }

    #[repr(C)]
    struct ProcessInformation {
        process: Handle,
        thread: Handle,
        process_id: u32,
        thread_id: u32,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct JobObjectBasicAccountingInformation {
        total_user_time: i64,
        total_kernel_time: i64,
        this_period_total_user_time: i64,
        this_period_total_kernel_time: i64,
        total_page_fault_count: u32,
        total_processes: u32,
        active_processes: u32,
        total_terminated_processes: u32,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct JobObjectBasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    struct JobObjectExtendedLimitInformation {
        basic_limit_information: JobObjectBasicLimitInformation,
        io_info: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[link(name = "userenv")]
    unsafe extern "system" {
        fn CreateAppContainerProfile(
            app_container_name: *const u16,
            display_name: *const u16,
            description: *const u16,
            capabilities: *const SidAndAttributes,
            capability_count: u32,
            app_container_sid: *mut Psid,
        ) -> Hresult;
        fn DeleteAppContainerProfile(app_container_name: *const u16) -> Hresult;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn FreeSid(sid: Psid) -> *mut c_void;
        fn OpenProcessToken(
            process_handle: Handle,
            desired_access: u32,
            token_handle: *mut Handle,
        ) -> i32;
        fn GetTokenInformation(
            token_handle: Handle,
            token_information_class: i32,
            token_information: *mut c_void,
            token_information_length: u32,
            return_length: *mut u32,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn InitializeProcThreadAttributeList(
            attribute_list: *mut c_void,
            attribute_count: u32,
            flags: u32,
            size: *mut usize,
        ) -> i32;
        fn UpdateProcThreadAttribute(
            attribute_list: *mut c_void,
            flags: u32,
            attribute: usize,
            value: *mut c_void,
            size: usize,
            previous_value: *mut c_void,
            return_size: *mut usize,
        ) -> i32;
        fn DeleteProcThreadAttributeList(attribute_list: *mut c_void);
        fn GetProcessHeap() -> Handle;
        fn HeapAlloc(heap: Handle, flags: u32, bytes: usize) -> *mut c_void;
        fn HeapFree(heap: Handle, flags: u32, memory: *mut c_void) -> i32;
        fn CreateProcessW(
            application_name: *const u16,
            command_line: *mut u16,
            process_attributes: *const c_void,
            thread_attributes: *const c_void,
            inherit_handles: i32,
            creation_flags: u32,
            environment: *mut c_void,
            current_directory: *const u16,
            startup_info: *const StartupInfoW,
            process_information: *mut ProcessInformation,
        ) -> i32;
        fn CreateJobObjectW(job_attributes: *const c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(
            job: Handle,
            information_class: i32,
            information: *const c_void,
            information_length: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn IsProcessInJob(process: Handle, job: Handle, result: *mut i32) -> i32;
        fn ResumeThread(thread: Handle) -> u32;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn GetExitCodeProcess(process: Handle, exit_code: *mut u32) -> i32;
        fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
        fn CreatePipe(
            read_pipe: *mut Handle,
            write_pipe: *mut Handle,
            attributes: *const c_void,
            size: u32,
        ) -> i32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
        fn SetHandleInformation(handle: Handle, mask: u32, flags: u32) -> i32;
        fn PeekNamedPipe(
            pipe: Handle,
            buffer: *mut c_void,
            size: u32,
            read: *mut u32,
            available: *mut u32,
            left: *mut u32,
        ) -> i32;
        fn ReadFile(
            pipe: Handle,
            buffer: *mut c_void,
            size: u32,
            read: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn QueryInformationJobObject(
            job: Handle,
            information_class: i32,
            information: *mut c_void,
            length: u32,
            return_length: *mut u32,
        ) -> i32;
        fn TerminateJobObject(job: Handle, exit_code: u32) -> i32;
        fn Sleep(milliseconds: u32);
        fn TerminateProcess(process: Handle, exit_code: u32) -> i32;
        fn CloseHandle(object: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    struct OwnedHandle(Handle);

    impl OwnedHandle {
        fn new(handle: Handle, operation: &str) -> Result<Self, ExecutionPlanError> {
            if handle.is_null() || handle as isize == -1 {
                Err(last_error(operation))
            } else {
                Ok(Self(handle))
            }
        }

        fn raw(&self) -> Handle {
            self.0
        }
    }

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    struct AppContainerProfile {
        name: Vec<u16>,
        sid: Psid,
        deleted: bool,
    }

    impl AppContainerProfile {
        fn create(name: &str) -> Result<Self, ExecutionPlanError> {
            validate_appcontainer_name(name)?;
            let name_wide = wide(OsStr::new(name));
            let display = wide(OsStr::new("Quntal contained launch probe"));
            let description = wide(OsStr::new(
                "Temporary zero-capability Quntal AppContainer launch probe",
            ));
            let mut sid: Psid = ptr::null_mut();

            let result = unsafe {
                CreateAppContainerProfile(
                    name_wide.as_ptr(),
                    display.as_ptr(),
                    description.as_ptr(),
                    ptr::null(),
                    0,
                    &mut sid,
                )
            };
            if result != 0 {
                return Err(hresult_error("CreateAppContainerProfile", result));
            }
            if sid.is_null() {
                let _ = unsafe { DeleteAppContainerProfile(name_wide.as_ptr()) };
                return Err(ExecutionPlanError::new(
                    "CreateAppContainerProfile returned a null SID",
                ));
            }

            Ok(Self {
                name: name_wide,
                sid,
                deleted: false,
            })
        }

        fn sid(&self) -> Psid {
            self.sid
        }

        fn delete(&mut self) -> Result<(), ExecutionPlanError> {
            if self.deleted {
                return Ok(());
            }
            let result = unsafe { DeleteAppContainerProfile(self.name.as_ptr()) };
            if result != 0 {
                return Err(hresult_error("DeleteAppContainerProfile", result));
            }
            self.deleted = true;
            Ok(())
        }
    }

    impl Drop for AppContainerProfile {
        fn drop(&mut self) {
            if !self.deleted {
                unsafe {
                    DeleteAppContainerProfile(self.name.as_ptr());
                }
            }
            if !self.sid.is_null() {
                unsafe {
                    FreeSid(self.sid);
                }
            }
        }
    }

    struct AttributeList {
        heap: Handle,
        list: *mut c_void,
        _handle_list: Vec<Handle>,
    }

    impl AttributeList {
        fn with_security_capabilities(
            security: &mut SecurityCapabilities,
            handles: &[Handle],
        ) -> Result<Self, ExecutionPlanError> {
            let heap = unsafe { GetProcessHeap() };
            if heap.is_null() {
                return Err(last_error("GetProcessHeap"));
            }
            let attribute_count = if handles.is_empty() { 1 } else { 2 };
            let mut bytes = 0usize;
            unsafe {
                InitializeProcThreadAttributeList(ptr::null_mut(), attribute_count, 0, &mut bytes);
            }
            if bytes == 0 {
                return Err(last_error("InitializeProcThreadAttributeList(size)"));
            }
            let list = unsafe { HeapAlloc(heap, 0, bytes) };
            if list.is_null() {
                return Err(last_error("HeapAlloc(attribute list)"));
            }
            if unsafe { InitializeProcThreadAttributeList(list, attribute_count, 0, &mut bytes) }
                == 0
            {
                let error = last_error("InitializeProcThreadAttributeList");
                unsafe {
                    HeapFree(heap, 0, list);
                }
                return Err(error);
            }
            let updated = unsafe {
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
                    (security as *mut SecurityCapabilities).cast(),
                    mem::size_of::<SecurityCapabilities>(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if updated == 0 {
                let error = last_error("UpdateProcThreadAttribute(SECURITY_CAPABILITIES)");
                unsafe {
                    DeleteProcThreadAttributeList(list);
                    HeapFree(heap, 0, list);
                }
                return Err(error);
            }
            let mut handle_list = handles.to_vec();
            if !handle_list.is_empty() {
                let listed = unsafe {
                    UpdateProcThreadAttribute(
                        list,
                        0,
                        PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                        handle_list.as_mut_ptr().cast(),
                        mem::size_of::<Handle>() * handle_list.len(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                    )
                };
                if listed == 0 {
                    let error = last_error("UpdateProcThreadAttribute(HANDLE_LIST)");
                    unsafe {
                        DeleteProcThreadAttributeList(list);
                        HeapFree(heap, 0, list);
                    }
                    return Err(error);
                }
            }
            Ok(Self {
                heap,
                list,
                _handle_list: handle_list,
            })
        }

        fn raw(&self) -> *mut c_void {
            self.list
        }
    }

    impl Drop for AttributeList {
        fn drop(&mut self) {
            if !self.list.is_null() {
                unsafe {
                    DeleteProcThreadAttributeList(self.list);
                    HeapFree(self.heap, 0, self.list);
                }
            }
        }
    }

    struct JobObject {
        handle: OwnedHandle,
    }

    impl JobObject {
        fn kill_on_close() -> Result<Self, ExecutionPlanError> {
            let handle = OwnedHandle::new(
                unsafe { CreateJobObjectW(ptr::null(), ptr::null()) },
                "CreateJobObjectW",
            )?;

            let mut information = JobObjectExtendedLimitInformation::default();
            information.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = unsafe {
                SetInformationJobObject(
                    handle.raw(),
                    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                    (&information as *const JobObjectExtendedLimitInformation).cast(),
                    mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
                )
            };
            if configured == 0 {
                return Err(last_error("SetInformationJobObject"));
            }

            Ok(Self { handle })
        }

        fn terminate(&self) -> Result<(), ExecutionPlanError> {
            if unsafe { TerminateJobObject(self.handle.raw(), 1) } == 0 {
                return Err(last_error("TerminateJobObject"));
            }
            Ok(())
        }

        fn wait_for_quiescence(&self) -> Result<bool, ExecutionPlanError> {
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_millis(QUIESCENCE_TIMEOUT_MS);
            loop {
                if self.active_processes()? == 0 {
                    return Ok(true);
                }
                if std::time::Instant::now() >= deadline {
                    return Ok(false);
                }
                unsafe {
                    Sleep(QUIESCENCE_POLL_MS as u32);
                }
            }
        }

        fn active_processes(&self) -> Result<u32, ExecutionPlanError> {
            let mut information = JobObjectBasicAccountingInformation::default();
            let queried = unsafe {
                QueryInformationJobObject(
                    self.handle.raw(),
                    JOB_OBJECT_BASIC_ACCOUNTING_INFORMATION_CLASS,
                    (&mut information as *mut JobObjectBasicAccountingInformation).cast(),
                    mem::size_of::<JobObjectBasicAccountingInformation>() as u32,
                    ptr::null_mut(),
                )
            };
            if queried == 0 {
                return Err(last_error("QueryInformationJobObject"));
            }
            Ok(information.active_processes)
        }

        fn assign_before_resume(&self, child: &ChildProcess) -> Result<(), ExecutionPlanError> {
            let assigned =
                unsafe { AssignProcessToJobObject(self.handle.raw(), child.process.raw()) };
            if assigned == 0 {
                return Err(last_error("AssignProcessToJobObject"));
            }

            let mut in_job = 0i32;
            let checked =
                unsafe { IsProcessInJob(child.process.raw(), self.handle.raw(), &mut in_job) };
            if checked == 0 {
                return Err(last_error("IsProcessInJob"));
            }
            if in_job == 0 {
                return Err(ExecutionPlanError::new(
                    "child process was not associated with the Quntal Job Object",
                ));
            }
            Ok(())
        }
    }

    struct ChildProcess {
        process: OwnedHandle,
        thread: OwnedHandle,
        completed: bool,
    }

    impl ChildProcess {
        fn create_suspended(profile_sid: Psid) -> Result<Self, ExecutionPlanError> {
            let executable = fixed_system_executable()?;

            let mut security = SecurityCapabilities {
                app_container_sid: profile_sid,
                capabilities: ptr::null_mut(),
                capability_count: 0,
                reserved: 0,
            };
            let attributes = AttributeList::with_security_capabilities(&mut security, &[])?;

            let mut startup: StartupInfoExW = unsafe { mem::zeroed() };
            startup.startup_info.cb = mem::size_of::<StartupInfoExW>() as u32;
            startup.attribute_list = attributes.raw();

            let application = wide(executable.as_os_str());
            let mut command_line = command_line_for_cmd(&executable);
            let mut environment = filtered_environment_block()?;

            let mut process_information: ProcessInformation = unsafe { mem::zeroed() };
            let created = unsafe {
                CreateProcessW(
                    application.as_ptr(),
                    command_line.as_mut_ptr(),
                    ptr::null(),
                    ptr::null(),
                    0,
                    CREATE_SUSPENDED
                        | CREATE_UNICODE_ENVIRONMENT
                        | EXTENDED_STARTUPINFO_PRESENT
                        | CREATE_NO_WINDOW,
                    environment.as_mut_ptr().cast(),
                    ptr::null(),
                    (&startup as *const StartupInfoExW).cast(),
                    &mut process_information,
                )
            };
            if created == 0 {
                return Err(last_error("CreateProcessW(AppContainer suspended)"));
            }

            let process = OwnedHandle::new(process_information.process, "CreateProcessW process")?;
            let thread = match OwnedHandle::new(process_information.thread, "CreateProcessW thread")
            {
                Ok(thread) => thread,
                Err(error) => {
                    unsafe {
                        TerminateProcess(process.raw(), 1);
                        WaitForSingleObject(process.raw(), 5_000);
                    }
                    return Err(error);
                }
            };

            Ok(Self {
                process,
                thread,
                completed: false,
            })
        }

        fn create_suspended_with_pipes(
            profile_sid: Psid,
            plan: &ExecutionPlan,
            stdout: &OwnedHandle,
            stderr: &OwnedHandle,
        ) -> Result<Self, ExecutionPlanError> {
            let stdin = null_input()?;
            let handles = [stdin.raw(), stdout.raw(), stderr.raw()];
            let mut security = SecurityCapabilities {
                app_container_sid: profile_sid,
                capabilities: ptr::null_mut(),
                capability_count: 0,
                reserved: 0,
            };
            let attributes = AttributeList::with_security_capabilities(&mut security, &handles)?;
            let mut startup: StartupInfoExW = unsafe { mem::zeroed() };
            startup.startup_info.cb = mem::size_of::<StartupInfoExW>() as u32;
            startup.startup_info.h_std_input = stdin.raw();
            startup.startup_info.h_std_output = stdout.raw();
            startup.startup_info.h_std_error = stderr.raw();
            startup.startup_info.dw_flags = 0x0000_0100;
            startup.attribute_list = attributes.raw();
            let process_executable = path_for_process_api(&plan.executable);
            let application = wide(process_executable.as_os_str());
            let mut command_line = command_line_for_paths(&process_executable, &plan.argv);
            let mut environment = environment_from_plan(plan)?;
            let process_cwd = path_for_process_api(&plan.cwd);
            let current_directory = wide(process_cwd.as_os_str());
            let mut information: ProcessInformation = unsafe { mem::zeroed() };
            let created = unsafe {
                CreateProcessW(
                    application.as_ptr(),
                    command_line.as_mut_ptr(),
                    ptr::null(),
                    ptr::null(),
                    1,
                    CREATE_SUSPENDED
                        | CREATE_UNICODE_ENVIRONMENT
                        | EXTENDED_STARTUPINFO_PRESENT
                        | CREATE_NO_WINDOW,
                    environment.as_mut_ptr().cast(),
                    current_directory.as_ptr(),
                    (&startup as *const StartupInfoExW).cast(),
                    &mut information,
                )
            };
            if created == 0 {
                return Err(last_error("CreateProcessW(contained execution)"));
            }
            let process = OwnedHandle::new(information.process, "CreateProcessW process")?;
            let thread = OwnedHandle::new(information.thread, "CreateProcessW thread")?;
            Ok(Self {
                process,
                thread,
                completed: false,
            })
        }

        fn verify_appcontainer_token(&self) -> Result<(), ExecutionPlanError> {
            let mut raw_token: Handle = ptr::null_mut();
            let opened =
                unsafe { OpenProcessToken(self.process.raw(), TOKEN_QUERY, &mut raw_token) };
            if opened == 0 {
                return Err(last_error("OpenProcessToken"));
            }
            let token = OwnedHandle::new(raw_token, "OpenProcessToken handle")?;

            let mut is_appcontainer = 0u32;
            let mut returned = 0u32;
            let queried = unsafe {
                GetTokenInformation(
                    token.raw(),
                    TOKEN_IS_APP_CONTAINER,
                    (&mut is_appcontainer as *mut u32).cast(),
                    mem::size_of::<u32>() as u32,
                    &mut returned,
                )
            };
            if queried == 0 {
                return Err(last_error("GetTokenInformation(TokenIsAppContainer)"));
            }
            if is_appcontainer == 0 {
                return Err(ExecutionPlanError::new(
                    "contained child token is not marked as AppContainer",
                ));
            }
            Ok(())
        }

        fn resume(&self) -> Result<(), ExecutionPlanError> {
            let previous = unsafe { ResumeThread(self.thread.raw()) };
            if previous == RESUME_FAILED {
                return Err(last_error("ResumeThread"));
            }
            Ok(())
        }

        fn wait_for_completion(&mut self) -> Result<u32, ExecutionPlanError> {
            let wait = unsafe { WaitForSingleObject(self.process.raw(), PROBE_TIMEOUT_MS) };
            match wait {
                WAIT_OBJECT_0 => {}
                WAIT_TIMEOUT => {
                    self.terminate_best_effort();
                    return Err(ExecutionPlanError::new(
                        "contained child did not exit before the probe timeout",
                    ));
                }
                WAIT_FAILED => {
                    let error = last_error("WaitForSingleObject");
                    self.terminate_best_effort();
                    return Err(error);
                }
                other => {
                    self.terminate_best_effort();
                    return Err(ExecutionPlanError::new(format!(
                        "WaitForSingleObject returned unexpected status {other}"
                    )));
                }
            }

            let mut exit_code = 0u32;
            let read = unsafe { GetExitCodeProcess(self.process.raw(), &mut exit_code) };
            if read == 0 {
                return Err(last_error("GetExitCodeProcess"));
            }
            self.completed = true;
            Ok(exit_code)
        }

        fn terminate_best_effort(&mut self) {
            unsafe {
                TerminateProcess(self.process.raw(), 1);
                WaitForSingleObject(self.process.raw(), 5_000);
            }
            self.completed = true;
        }
    }

    impl Drop for ChildProcess {
        fn drop(&mut self) {
            if !self.completed {
                self.terminate_best_effort();
            }
        }
    }

    pub(super) fn probe(name: &str) -> Result<ContainedLaunchProbe, ExecutionPlanError> {
        let mut profile = AppContainerProfile::create(name)?;
        let job = JobObject::kill_on_close()?;
        let mut child = ChildProcess::create_suspended(profile.sid())?;

        if let Err(error) = job.assign_before_resume(&child) {
            child.terminate_best_effort();
            return Err(error);
        }

        child.verify_appcontainer_token()?;
        child.resume()?;
        let exit_code = child.wait_for_completion()?;
        profile.delete()?;

        Ok(ContainedLaunchProbe {
            profile_created: true,
            process_created_suspended: true,
            appcontainer_token_verified: true,
            assigned_to_job_before_resume: true,
            resumed: true,
            child_completed: true,
            exit_code,
            profile_deleted: true,
        })
    }

    struct PrivatePipe {
        read: OwnedHandle,
    }

    impl PrivatePipe {
        fn create() -> Result<(Self, OwnedHandle), ExecutionPlanError> {
            let mut read = ptr::null_mut();
            let mut write = ptr::null_mut();
            let attributes = SecurityAttributes {
                length: mem::size_of::<SecurityAttributes>() as u32,
                security_descriptor: ptr::null_mut(),
                inherit_handle: 1,
            };
            if unsafe {
                CreatePipe(
                    &mut read,
                    &mut write,
                    (&attributes as *const SecurityAttributes).cast(),
                    0,
                )
            } == 0
            {
                return Err(last_error("CreatePipe"));
            }
            let read = OwnedHandle::new(read, "CreatePipe read")?;
            let write = OwnedHandle::new(write, "CreatePipe write")?;
            if unsafe { SetHandleInformation(read.raw(), HANDLE_FLAG_INHERIT, 0) } == 0 {
                return Err(last_error("SetHandleInformation"));
            }
            Ok((Self { read }, write))
        }
    }

    fn null_input() -> Result<OwnedHandle, ExecutionPlanError> {
        let name = wide(OsStr::new("NUL"));
        let share = FILE_SHARE_READ | FILE_SHARE_WRITE;
        let attributes = SecurityAttributes {
            length: mem::size_of::<SecurityAttributes>() as u32,
            security_descriptor: ptr::null_mut(),
            inherit_handle: 1,
        };
        OwnedHandle::new(
            unsafe {
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_READ,
                    share,
                    (&attributes as *const SecurityAttributes).cast(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    ptr::null_mut(),
                )
            },
            "CreateFileW(NUL)",
        )
    }

    fn quote_arg(arg: &str) -> Vec<u16> {
        let mut out = Vec::new();
        out.push('"' as u16);
        let mut backslashes = 0usize;
        for ch in arg.encode_utf16() {
            if ch == '\\' as u16 {
                backslashes += 1;
            } else if ch == '"' as u16 {
                out.extend(std::iter::repeat_n('\\' as u16, backslashes * 2 + 1));
                out.push(ch);
                backslashes = 0;
            } else {
                out.extend(std::iter::repeat_n('\\' as u16, backslashes));
                out.push(ch);
                backslashes = 0;
            }
        }
        out.extend(std::iter::repeat_n('\\' as u16, backslashes * 2));
        out.push('"' as u16);
        out
    }

    fn path_for_process_api(path: &Path) -> PathBuf {
        let raw: Vec<u16> = path.as_os_str().encode_wide().collect();
        if raw.starts_with(&[0x5c, 0x5c, 0x3f, 0x5c]) {
            let mut stripped = raw[4..].to_vec();
            if stripped.starts_with(&[b'U' as u16, b'N' as u16, b'C' as u16, 0x5c]) {
                stripped.drain(0..3);
                stripped.insert(0, 0x5c);
            }
            return PathBuf::from(OsString::from_wide(&stripped));
        }
        path.to_path_buf()
    }

    fn command_line_for_paths(executable: &Path, argv: &[String]) -> Vec<u16> {
        let mut command = quote_arg(&executable.to_string_lossy());
        for arg in argv {
            command.push(' ' as u16);
            command.extend(quote_arg(arg));
        }
        command.push(0);
        command
    }

    fn environment_from_plan(plan: &ExecutionPlan) -> Result<Vec<u16>, ExecutionPlanError> {
        let mut entries = Vec::new();
        for (key, value) in &plan.env {
            if key.is_empty() || key.contains('\0') || value.contains('\0') || key.contains('=') {
                return Err(ExecutionPlanError::new("invalid environment entry"));
            }
            let upper = key.to_ascii_uppercase();
            if SECRETISH.iter().any(|needle| upper.contains(needle))
                || FORBIDDEN_QUNTAL_ENV
                    .iter()
                    .any(|forbidden| upper == *forbidden)
                || !allowed_execution_env(key)
            {
                continue;
            }
            entries.push((key.clone(), value.clone()));
        }
        for required in ["LOCALAPPDATA", "SystemRoot", "TEMP", "TMP"] {
            if !entries
                .iter()
                .any(|(key, _)| key.eq_ignore_ascii_case(required))
            {
                return Err(ExecutionPlanError::new(format!(
                    "required execution environment variable is unavailable: {required}"
                )));
            }
        }
        entries.sort_by_key(|entry| entry.0.to_ascii_lowercase());
        let mut block = Vec::new();
        for (key, value) in entries {
            block.extend(key.encode_utf16());
            block.push('=' as u16);
            block.extend(value.encode_utf16());
            block.push(0);
        }
        block.push(0);
        Ok(block)
    }

    enum PipeRead {
        Open,
        Closed,
        LimitExceeded,
    }

    fn read_pipe(
        pipe: &PrivatePipe,
        output: &mut Vec<u8>,
        limit: usize,
    ) -> Result<PipeRead, ExecutionPlanError> {
        let mut available = 0u32;
        let peeked = unsafe {
            PeekNamedPipe(
                pipe.read.raw(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                &mut available,
                ptr::null_mut(),
            )
        };
        if peeked == 0 {
            let code = unsafe { GetLastError() };
            if code == ERROR_BROKEN_PIPE {
                return Ok(PipeRead::Closed);
            }
            return Err(last_error("PeekNamedPipe"));
        }
        if available == 0 {
            return Ok(PipeRead::Open);
        }
        let remaining = limit.saturating_sub(output.len());
        if available as usize > remaining {
            return Ok(PipeRead::LimitExceeded);
        }
        let mut buffer = vec![0u8; available as usize];
        let mut read = 0u32;
        if unsafe {
            ReadFile(
                pipe.read.raw(),
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
                ptr::null_mut(),
            )
        } == 0
        {
            let code = unsafe { GetLastError() };
            if code == ERROR_BROKEN_PIPE {
                return Ok(PipeRead::Closed);
            }
            return Err(last_error("ReadFile"));
        }
        buffer.truncate(read as usize);
        output.extend_from_slice(&buffer);
        Ok(PipeRead::Open)
    }

    #[cfg(windows)]
    pub(crate) enum DestructiveEvent {
        Timeout,
        OutputLimit(OutputStream),
    }

    #[cfg(windows)]
    pub(crate) fn classify_termination(
        event: DestructiveEvent,
        verified: bool,
    ) -> PrivateExecutionFailure {
        if !verified {
            return PrivateExecutionFailure::TerminationUnverified;
        }
        match event {
            DestructiveEvent::Timeout => PrivateExecutionFailure::ProcessTimeout,
            DestructiveEvent::OutputLimit(stream) => PrivateExecutionFailure::OutputLimit(stream),
        }
    }

    fn terminate_and_verify(job: &JobObject, event: DestructiveEvent) -> PrivateExecutionFailure {
        if job.terminate().is_err() {
            return PrivateExecutionFailure::TerminationUnverified;
        }
        let verified = matches!(job.wait_for_quiescence(), Ok(true));
        classify_termination(event, verified)
    }

    enum PostExitDrain {
        Complete,
        OutputLimit(OutputStream),
        Deadline,
    }

    fn drain_pipes_until(
        stdout_pipe: &PrivatePipe,
        stderr_pipe: &PrivatePipe,
        stdout: &mut Vec<u8>,
        stderr: &mut Vec<u8>,
        stdout_limit: usize,
        stderr_limit: usize,
        deadline: std::time::Instant,
    ) -> Result<PostExitDrain, PrivateExecutionFailure> {
        let mut stdout_closed = false;
        let mut stderr_closed = false;
        loop {
            if !stdout_closed {
                match read_pipe(stdout_pipe, stdout, stdout_limit) {
                    Ok(PipeRead::Closed) => stdout_closed = true,
                    Ok(PipeRead::Open) => {}
                    Ok(PipeRead::LimitExceeded) => {
                        return Ok(PostExitDrain::OutputLimit(OutputStream::Stdout));
                    }
                    Err(error) => {
                        return Err(PrivateExecutionFailure::Provider(error.message));
                    }
                }
            }
            if !stderr_closed {
                match read_pipe(stderr_pipe, stderr, stderr_limit) {
                    Ok(PipeRead::Closed) => stderr_closed = true,
                    Ok(PipeRead::Open) => {}
                    Ok(PipeRead::LimitExceeded) => {
                        return Ok(PostExitDrain::OutputLimit(OutputStream::Stderr));
                    }
                    Err(error) => {
                        return Err(PrivateExecutionFailure::Provider(error.message));
                    }
                }
            }
            if stdout_closed && stderr_closed {
                return Ok(PostExitDrain::Complete);
            }
            if std::time::Instant::now() >= deadline {
                return Ok(PostExitDrain::Deadline);
            }
            unsafe {
                Sleep(QUIESCENCE_POLL_MS as u32);
            }
        }
    }

    pub(super) fn qualify_private_execution(
        plan: ExecutionPlan,
        profile_name: &str,
        mode: PrivateExecutionMode,
    ) -> Result<PrivateExecutionResult, PrivateExecutionFailure> {
        let expected_executable = match mode {
            PrivateExecutionMode::Public => plan.executable.clone(),
            PrivateExecutionMode::Success => fixed_system_executable()
                .map(|cmd| cmd.parent().map(|parent| parent.join("whoami.exe")))
                .map_err(|error| PrivateExecutionFailure::InvalidPlan(error.message))?
                .ok_or_else(|| {
                    PrivateExecutionFailure::InvalidPlan("system directory is unavailable".into())
                })?,
            PrivateExecutionMode::Timeout
            | PrivateExecutionMode::StdoutLimit
            | PrivateExecutionMode::StderrLimit
            | PrivateExecutionMode::DescendantDrain => std::env::current_exe()
                .map_err(|error| PrivateExecutionFailure::InvalidPlan(error.to_string()))?,
        };
        let expected_executable = std::fs::canonicalize(expected_executable)
            .map_err(|error| PrivateExecutionFailure::InvalidPlan(error.to_string()))?;
        let expected_argv: Vec<String> = match mode {
            PrivateExecutionMode::Public => plan.argv.clone(),
            PrivateExecutionMode::Success => Vec::new(),
            PrivateExecutionMode::Timeout => vec![
                "--exact".into(),
                "contained_launch_tests::private_timeout_fixture_child".into(),
                "--nocapture".into(),
            ],
            PrivateExecutionMode::StdoutLimit => vec![
                "--exact".into(),
                "contained_launch_tests::private_stdout_fixture_child".into(),
                "--nocapture".into(),
            ],
            PrivateExecutionMode::StderrLimit => vec![
                "--exact".into(),
                "contained_launch_tests::private_stderr_fixture_child".into(),
                "--nocapture".into(),
            ],
            PrivateExecutionMode::DescendantDrain => vec![
                "--exact".into(),
                "contained_launch_tests::private_descendant_primary_child".into(),
                "--nocapture".into(),
            ],
        };
        if expected_executable != plan.executable || expected_argv != plan.argv {
            return Err(PrivateExecutionFailure::InvalidPlan(
                "contained executor received an executable or arguments outside the normalized plan".into(),
            ));
        }
        let mut profile = AppContainerProfile::create(profile_name)
            .map_err(|e| PrivateExecutionFailure::Provider(e.message))?;
        let job =
            JobObject::kill_on_close().map_err(|e| PrivateExecutionFailure::Provider(e.message))?;
        let (stdout_pipe, stdout_write) =
            PrivatePipe::create().map_err(|e| PrivateExecutionFailure::Provider(e.message))?;
        let (stderr_pipe, stderr_write) =
            PrivatePipe::create().map_err(|e| PrivateExecutionFailure::Provider(e.message))?;
        let mut child = match ChildProcess::create_suspended_with_pipes(
            profile.sid(),
            &plan,
            &stdout_write,
            &stderr_write,
        ) {
            Ok(child) => child,
            Err(error) => return Err(PrivateExecutionFailure::Provider(error.message)),
        };
        let mut descendant = if matches!(
            mode,
            PrivateExecutionMode::Timeout
                | PrivateExecutionMode::StdoutLimit
                | PrivateExecutionMode::StderrLimit
                | PrivateExecutionMode::DescendantDrain
        ) {
            let descendant_argv = match mode {
                PrivateExecutionMode::Timeout => vec![
                    "--exact".to_owned(),
                    "contained_launch_tests::private_timeout_descendant".to_owned(),
                    "--nocapture".to_owned(),
                ],
                PrivateExecutionMode::StdoutLimit
                | PrivateExecutionMode::StderrLimit
                | PrivateExecutionMode::DescendantDrain => vec![
                    "--exact".to_owned(),
                    "contained_launch_tests::private_output_descendant".to_owned(),
                    "--nocapture".to_owned(),
                ],
                PrivateExecutionMode::Public | PrivateExecutionMode::Success => unreachable!(),
            };
            let mut descendant_plan = plan.clone();
            descendant_plan.argv = descendant_argv;
            Some(
                match ChildProcess::create_suspended_with_pipes(
                    profile.sid(),
                    &descendant_plan,
                    &stdout_write,
                    &stderr_write,
                ) {
                    Ok(descendant) => descendant,
                    Err(error) => return Err(PrivateExecutionFailure::Provider(error.message)),
                },
            )
        } else {
            None
        };
        drop(stdout_write);
        drop(stderr_write);
        if let Some(error) = job.assign_before_resume(&child).err() {
            child.terminate_best_effort();
            return Err(PrivateExecutionFailure::Provider(error.message));
        }
        if let Some(descendant_process) = descendant.as_mut() {
            if let Err(error) = job.assign_before_resume(descendant_process) {
                child.terminate_best_effort();
                descendant_process.terminate_best_effort();
                return Err(PrivateExecutionFailure::Provider(error.message));
            }
        }
        if let Err(error) = child.verify_appcontainer_token() {
            child.terminate_best_effort();
            return Err(PrivateExecutionFailure::Provider(error.message));
        }
        if let Some(descendant_process) = descendant.as_mut() {
            if let Err(error) = descendant_process.verify_appcontainer_token() {
                child.terminate_best_effort();
                descendant_process.terminate_best_effort();
                return Err(PrivateExecutionFailure::Provider(error.message));
            }
        }
        if let Err(error) = child.resume() {
            child.terminate_best_effort();
            return Err(PrivateExecutionFailure::Provider(error.message));
        }
        if let Some(descendant_process) = descendant.as_mut() {
            if let Err(error) = descendant_process.resume() {
                child.terminate_best_effort();
                descendant_process.terminate_best_effort();
                return Err(PrivateExecutionFailure::Provider(error.message));
            }
        }
        let started = std::time::Instant::now();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut destructive_event = None;
        loop {
            match read_pipe(&stdout_pipe, &mut stdout, plan.limits.stdout_bytes) {
                Ok(PipeRead::LimitExceeded) if destructive_event.is_none() => {
                    destructive_event = Some(DestructiveEvent::OutputLimit(OutputStream::Stdout));
                }
                Ok(PipeRead::Open) | Ok(PipeRead::Closed) | Ok(PipeRead::LimitExceeded) => {}
                Err(error) => return Err(PrivateExecutionFailure::Provider(error.message)),
            }
            match read_pipe(&stderr_pipe, &mut stderr, plan.limits.stderr_bytes) {
                Ok(PipeRead::LimitExceeded) if destructive_event.is_none() => {
                    destructive_event = Some(DestructiveEvent::OutputLimit(OutputStream::Stderr));
                }
                Ok(PipeRead::Open) | Ok(PipeRead::Closed) | Ok(PipeRead::LimitExceeded) => {}
                Err(error) => return Err(PrivateExecutionFailure::Provider(error.message)),
            }
            if destructive_event.is_some() {
                break;
            }
            let wait = unsafe { WaitForSingleObject(child.process.raw(), 0) };
            if wait == WAIT_OBJECT_0 {
                break;
            }
            if started.elapsed() >= plan.limits.timeout {
                destructive_event = Some(DestructiveEvent::Timeout);
                break;
            }
            unsafe {
                Sleep(10);
            }
        }
        if let Some(event) = destructive_event {
            return Err(terminate_and_verify(&job, event));
        }
        let exit_code = child
            .wait_for_completion()
            .map_err(|e| PrivateExecutionFailure::Provider(e.message))?;
        let drain_deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(POST_EXIT_DRAIN_TIMEOUT_MS);
        match drain_pipes_until(
            &stdout_pipe,
            &stderr_pipe,
            &mut stdout,
            &mut stderr,
            plan.limits.stdout_bytes,
            plan.limits.stderr_bytes,
            drain_deadline,
        )? {
            PostExitDrain::Complete => {}
            PostExitDrain::OutputLimit(stream) => {
                return Err(terminate_and_verify(
                    &job,
                    DestructiveEvent::OutputLimit(stream),
                ));
            }
            PostExitDrain::Deadline => {
                let active = job
                    .active_processes()
                    .map_err(|_| PrivateExecutionFailure::TerminationUnverified)?;
                if active == 0 || job.terminate().is_err() {
                    return Err(PrivateExecutionFailure::TerminationUnverified);
                }
                if !matches!(job.wait_for_quiescence(), Ok(true)) {
                    return Err(PrivateExecutionFailure::TerminationUnverified);
                }
                if let Some(descendant_process) = descendant.as_mut() {
                    descendant_process.completed = true;
                }
                let cleanup_deadline = std::time::Instant::now()
                    + std::time::Duration::from_millis(POST_TERMINATION_DRAIN_TIMEOUT_MS);
                match drain_pipes_until(
                    &stdout_pipe,
                    &stderr_pipe,
                    &mut stdout,
                    &mut stderr,
                    plan.limits.stdout_bytes,
                    plan.limits.stderr_bytes,
                    cleanup_deadline,
                )? {
                    PostExitDrain::Complete => {}
                    PostExitDrain::OutputLimit(stream) => {
                        return Err(classify_termination(
                            DestructiveEvent::OutputLimit(stream),
                            true,
                        ));
                    }
                    PostExitDrain::Deadline => {
                        return Err(PrivateExecutionFailure::TerminationUnverified);
                    }
                }
            }
        }
        let quiescent = job
            .wait_for_quiescence()
            .map_err(|_| PrivateExecutionFailure::TerminationUnverified)?;
        profile
            .delete()
            .map_err(|e| PrivateExecutionFailure::Provider(e.message))?;
        if !quiescent {
            return Err(PrivateExecutionFailure::TerminationUnverified);
        }
        Ok(PrivateExecutionResult {
            exit_code,
            stdout,
            stderr,
            appcontainer_verified: true,
            assigned_to_job_before_resume: true,
            job_quiescent: true,
        })
    }

    fn command_line_for_cmd(executable: &Path) -> Vec<u16> {
        let mut command = Vec::new();
        command.push('"' as u16);
        command.extend(executable.as_os_str().encode_wide());
        command.push('"' as u16);
        command.extend(" /d /q /c exit 0".encode_utf16());
        command.push(0);
        command
    }

    fn filtered_environment_block() -> Result<Vec<u16>, ExecutionPlanError> {
        const SAFE_KEYS: &[&str] = &[
            "APPDATA",
            "ComSpec",
            "HOMEDRIVE",
            "HOMEPATH",
            "LOCALAPPDATA",
            "NUMBER_OF_PROCESSORS",
            "OS",
            "PATH",
            "PATHEXT",
            "PROCESSOR_ARCHITECTURE",
            "SystemDrive",
            "SystemRoot",
            "TEMP",
            "TMP",
            "USERPROFILE",
            "WINDIR",
        ];

        let mut entries = Vec::<(String, std::ffi::OsString)>::new();
        for key in SAFE_KEYS {
            if let Some(value) = std::env::var_os(key) {
                entries.push(((*key).to_owned(), value));
            }
        }

        for required in ["LOCALAPPDATA", "SystemRoot", "TEMP", "TMP"] {
            if !entries
                .iter()
                .any(|(key, _)| key.eq_ignore_ascii_case(required))
            {
                return Err(ExecutionPlanError::new(format!(
                    "required Windows launch environment variable is unavailable: {required}"
                )));
            }
        }

        entries.sort_by(|left, right| {
            left.0
                .to_ascii_lowercase()
                .cmp(&right.0.to_ascii_lowercase())
        });

        let mut environment = Vec::<u16>::new();
        for (key, value) in entries {
            environment.extend(key.encode_utf16());
            environment.push('=' as u16);
            environment.extend(value.encode_wide());
            environment.push(0);
        }
        environment.push(0);
        Ok(environment)
    }

    pub(super) fn fixed_system_executable() -> Result<std::path::PathBuf, ExecutionPlanError> {
        const INITIAL_BUFFER_CHARS: usize = 260;
        const MAX_SYSTEM_DIRECTORY_CHARS: usize = 32_767;

        let mut buffer = vec![0u16; INITIAL_BUFFER_CHARS];
        let copied =
            unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), INITIAL_BUFFER_CHARS as u32) };
        let copied = if copied == 0 {
            return Err(last_error("GetSystemDirectoryW"));
        } else if copied < INITIAL_BUFFER_CHARS as u32 {
            usize::try_from(copied).map_err(|_| {
                ExecutionPlanError::new("GetSystemDirectoryW returned an invalid length")
            })?
        } else {
            let capacity = usize::try_from(copied)
                .ok()
                .filter(|capacity| *capacity <= MAX_SYSTEM_DIRECTORY_CHARS)
                .ok_or_else(|| {
                    ExecutionPlanError::new("GetSystemDirectoryW returned an invalid size")
                })?;
            buffer.resize(capacity, 0);
            let copied = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), capacity as u32) };
            if copied == 0 {
                return Err(last_error("GetSystemDirectoryW(resized)"));
            }
            let copied = usize::try_from(copied).map_err(|_| {
                ExecutionPlanError::new("GetSystemDirectoryW returned an invalid length")
            })?;
            if copied + 1 > capacity {
                return Err(ExecutionPlanError::new(
                    "GetSystemDirectoryW returned an invalid length",
                ));
            }
            copied
        };
        buffer.truncate(copied);

        let executable = Path::new(OsString::from_wide(&buffer).as_os_str()).join("cmd.exe");
        if !executable.is_file() {
            return Err(ExecutionPlanError::new(format!(
                "contained probe executable does not exist: {}",
                executable.display()
            )));
        }
        Ok(executable)
    }

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn last_error(operation: &str) -> ExecutionPlanError {
        let code = unsafe { GetLastError() };
        ExecutionPlanError::new(format!("{operation} failed with Win32 error {code}"))
    }

    fn hresult_error(operation: &str, code: Hresult) -> ExecutionPlanError {
        ExecutionPlanError::new(format!(
            "{operation} failed with HRESULT 0x{:08X}",
            code as u32
        ))
    }
}

#[cfg(all(test, windows))]
mod contained_launch_tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

    struct EnvironmentVarGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvironmentVarGuard {
        fn replace(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvironmentVarGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn fixed_system_executable_ignores_caller_controlled_system_root() {
        let _environment_lock = ENVIRONMENT_LOCK.lock().expect("environment lock");
        let expected = windows_contained_launch::fixed_system_executable()
            .expect("resolve fixed system executable");
        let _environment = EnvironmentVarGuard::replace("SystemRoot", r"C:\caller-controlled");
        let actual = windows_contained_launch::fixed_system_executable()
            .expect("resolve fixed system executable");
        assert_eq!(actual, expected);
    }

    fn bounded_child_diagnostic(label: &str, bytes: &[u8]) -> String {
        let mut text = String::new();
        for byte in bytes.iter().copied().take(512) {
            match byte {
                b'\n' => text.push_str("\\n"),
                b'\r' => text.push_str("\\r"),
                b'\t' => text.push_str("\\t"),
                0x20..=0x7e => text.push(char::from(byte)),
                _ => text.push_str(&format!("\\x{byte:02x}")),
            }
        }
        let lower = text.to_ascii_lowercase();
        for marker in [
            "password",
            "token",
            "secret",
            "credential",
            "api_key",
            "private_key",
        ] {
            if lower.contains(marker) {
                return format!("{label}=<redacted:{}>", bytes.len());
            }
        }
        format!("{label}[{}]={text}", bytes.len())
    }

    fn private_qualification_env() -> std::collections::BTreeMap<String, String> {
        let mut env = std::collections::BTreeMap::new();
        for key in [
            "LOCALAPPDATA",
            "SystemRoot",
            "TEMP",
            "TMP",
            "PATH",
            "ComSpec",
        ] {
            env.insert(
                key.to_owned(),
                std::env::var(key).expect("required qualification environment"),
            );
        }
        env
    }

    #[test]
    fn windows_private_qualification_executes_bounded_fixed_child_with_quiescent_job() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!("Quntal.Private.Exec.{suffix}"));
        std::fs::create_dir_all(&workspace).expect("qualification workspace");
        let executable = windows_contained_launch::fixed_system_executable()
            .expect("system executable")
            .parent()
            .expect("system directory")
            .join("whoami.exe");
        let env = private_qualification_env();
        let plan = build_execution_plan(
            &workspace,
            &executable,
            &[],
            ".",
            &env,
            ExecutionLimits::default(),
        )
        .expect("bounded private plan");
        let profile = format!("Quntal.Private.Exec.{suffix}");
        let result = qualify_private_execution(plan, &profile).expect("private execution");
        assert!(result.appcontainer_verified);
        assert!(result.assigned_to_job_before_resume);
        assert!(result.job_quiescent);
        assert_eq!(
            result.exit_code,
            0,
            "private fixture diagnostic: {} {}",
            bounded_child_diagnostic("stdout", &result.stdout),
            bounded_child_diagnostic("stderr", &result.stderr),
        );
        assert!(!result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        let _ = std::fs::remove_dir_all(workspace);
    }

    fn private_fixture_plan(
        suffix: u128,
        test_name: &str,
        limits: ExecutionLimits,
    ) -> (PathBuf, ExecutionPlan) {
        let workspace = std::env::temp_dir().join(format!("Quntal.Private.Failure.{suffix}"));
        std::fs::create_dir_all(&workspace).expect("qualification workspace");
        let executable = std::env::current_exe().expect("fixture executable");
        let plan = build_execution_plan(
            &workspace,
            &executable,
            &[
                "--exact".to_owned(),
                test_name.to_owned(),
                "--nocapture".to_owned(),
            ],
            ".",
            &private_qualification_env(),
            limits,
        )
        .expect("bounded private plan");
        (workspace, plan)
    }

    #[test]
    fn private_timeout_fixture_child() {
        if std::env::args().any(|argument| argument == "--exact") {
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn private_timeout_descendant() {
        if std::env::args().any(|argument| argument == "--exact") {
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn private_stdout_fixture_child() {
        if std::env::args().any(|argument| argument == "--exact") {
            let mut stdout = std::io::stdout().lock();
            use std::io::Write;
            let _ = stdout.write_all(&vec![b'x'; 4096]);
            let _ = stdout.flush();
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn private_stderr_fixture_child() {
        if std::env::args().any(|argument| argument == "--exact") {
            let mut stderr = std::io::stderr().lock();
            use std::io::Write;
            let _ = stderr.write_all(&vec![b'x'; 4096]);
            let _ = stderr.flush();
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn private_descendant_primary_child() {}

    #[test]
    fn private_output_descendant() {
        if std::env::args().any(|argument| argument == "--exact") {
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn windows_private_timeout_is_verified_and_typed() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let limits = ExecutionLimits {
            timeout: Duration::from_secs(1),
            stdout_bytes: 1024,
            stderr_bytes: 1024,
        };
        let (workspace, plan) = private_fixture_plan(
            suffix,
            "contained_launch_tests::private_timeout_fixture_child",
            limits,
        );
        let result = qualify_private_execution_mode(
            plan,
            &format!("Quntal.Private.Timeout.{suffix}"),
            PrivateExecutionMode::Timeout,
        );
        assert_eq!(result, Err(PrivateExecutionFailure::ProcessTimeout));
        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn windows_private_stdout_limit_is_stream_typed_and_verified() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let limits = ExecutionLimits {
            timeout: Duration::from_secs(5),
            stdout_bytes: 1,
            stderr_bytes: 1024,
        };
        let (workspace, plan) = private_fixture_plan(
            suffix,
            "contained_launch_tests::private_stdout_fixture_child",
            limits,
        );
        let result = qualify_private_execution_mode(
            plan,
            &format!("Quntal.Private.Stdout.{suffix}"),
            PrivateExecutionMode::StdoutLimit,
        );
        assert_eq!(
            result,
            Err(PrivateExecutionFailure::OutputLimit(OutputStream::Stdout))
        );
        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn windows_private_stderr_limit_is_stream_typed_and_verified() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let limits = ExecutionLimits {
            timeout: Duration::from_secs(5),
            stdout_bytes: 1024,
            stderr_bytes: 1,
        };
        let (workspace, plan) = private_fixture_plan(
            suffix,
            "contained_launch_tests::private_stderr_fixture_child",
            limits,
        );
        let result = qualify_private_execution_mode(
            plan,
            &format!("Quntal.Private.Stderr.{suffix}"),
            PrivateExecutionMode::StderrLimit,
        );
        assert_eq!(
            result,
            Err(PrivateExecutionFailure::OutputLimit(OutputStream::Stderr))
        );
        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn windows_private_descendant_retained_stdio_is_bounded_and_quiescent() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let limits = ExecutionLimits {
            timeout: Duration::from_secs(5),
            stdout_bytes: 16 * 1024,
            stderr_bytes: 16 * 1024,
        };
        let (workspace, plan) = private_fixture_plan(
            suffix,
            "contained_launch_tests::private_descendant_primary_child",
            limits,
        );
        let started = std::time::Instant::now();
        let result = qualify_private_execution_mode(
            plan,
            &format!("Quntal.Private.DescendantDrain.{suffix}"),
            PrivateExecutionMode::DescendantDrain,
        )
        .expect("bounded descendant drain");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(result.exit_code, 0);
        assert!(result.appcontainer_verified);
        assert!(result.assigned_to_job_before_resume);
        assert!(result.job_quiescent);
        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn windows_private_unverified_termination_never_promotes_timeout_or_output_limit() {
        assert_eq!(
            windows_contained_launch::classify_termination(
                windows_contained_launch::DestructiveEvent::Timeout,
                false,
            ),
            PrivateExecutionFailure::TerminationUnverified
        );
        assert_eq!(
            windows_contained_launch::classify_termination(
                windows_contained_launch::DestructiveEvent::OutputLimit(OutputStream::Stdout),
                false,
            ),
            PrivateExecutionFailure::TerminationUnverified
        );
        assert_eq!(
            windows_contained_launch::classify_termination(
                windows_contained_launch::DestructiveEvent::OutputLimit(OutputStream::Stderr),
                false,
            ),
            PrivateExecutionFailure::TerminationUnverified
        );
    }

    #[test]
    fn windows_appcontainer_child_is_job_assigned_before_resume() {
        let _guard = ENVIRONMENT_LOCK.lock().expect("environment lock");
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let name = format!("Quntal.Contained.{}.{}", std::process::id(), suffix);

        let result =
            probe_contained_appcontainer_job_launch(&name).expect("contained child launch");
        assert!(result.profile_created);
        assert!(result.process_created_suspended);
        assert!(result.appcontainer_token_verified);
        assert!(result.assigned_to_job_before_resume);
        assert!(result.resumed);
        assert!(result.child_completed);
        assert_eq!(result.exit_code, 0);
        assert!(result.profile_deleted);
    }
}
