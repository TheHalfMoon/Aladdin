//! SG-000074 engine discovery and verification.
//!
//! Deskal never bundles a browser and never downloads one. Discovery finds
//! an installed Chromium-family engine, verifies its kind, version, and
//! (on Windows) Authenticode signature, and refuses everything else with
//! typed unavailable. There is no personal-browser fallback.

use crate::error::{HostError, UnavailableReason};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Supported engine kinds. Anything else is unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Edge,
    Chromium,
}

impl EngineKind {
    fn executable_names(&self) -> &'static [&'static str] {
        match self {
            Self::Edge => &["msedge.exe", "msedge"],
            Self::Chromium => &["chrome.exe", "chrome", "chromium", "chromium-browser"],
        }
    }
}

/// A verified engine executable.
#[derive(Debug, Clone)]
pub struct Engine {
    path: PathBuf,
    kind: EngineKind,
    version: String,
    publisher_verified: bool,
}

impl Engine {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> EngineKind {
        self.kind
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Whether the OS publisher check passed. Always true on Windows
    /// (WinVerifyTrust is mandatory there); elsewhere the path and
    /// version checks apply and this reports false.
    pub fn publisher_verified(&self) -> bool {
        self.publisher_verified
    }

    /// Short fingerprint binding kind, version, and path for identities.
    pub fn fingerprint(&self) -> String {
        format!(
            "{:?}:{}:{}",
            self.kind,
            self.version,
            self.path.to_string_lossy()
        )
    }
}

/// Where to look. Production scans isolated extra roots (none) plus the
/// machine. Tests disable the machine scan for deterministic unavailability.
pub struct SearchConfig {
    extra_roots: Vec<PathBuf>,
    system: bool,
}

impl SearchConfig {
    pub fn production() -> Self {
        Self {
            extra_roots: Vec::new(),
            system: true,
        }
    }

    pub fn with_extra_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            extra_roots: roots,
            system: true,
        }
    }

    /// Test-only scope: extra roots alone, never the machine.
    pub fn isolated(roots: Vec<PathBuf>) -> Self {
        Self {
            extra_roots: roots,
            system: false,
        }
    }
}

/// Discover, probe, and verify one engine. Fails closed with
/// [`UnavailableReason`] when nothing suitable exists.
pub fn discover_engine(search: &SearchConfig) -> Result<Engine, HostError> {
    let mut candidates = Vec::new();
    candidates.extend(
        search
            .extra_roots
            .iter()
            .flat_map(|root| candidate_executables(root)),
    );
    if search.system {
        candidates.extend(platform_candidates());
    }
    for path in candidates {
        match verify_candidate(&path) {
            Ok(engine) => return Ok(engine),
            Err(HostError::Unavailable(_)) => continue,
            Err(other) => return Err(other),
        }
    }
    Err(HostError::Unavailable(UnavailableReason::NoEngine))
}

fn candidate_executables(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for kind in [EngineKind::Edge, EngineKind::Chromium] {
        for name in kind.executable_names() {
            out.push(root.join(name));
        }
    }
    out.sort();
    out
}

fn verify_candidate(path: &Path) -> Result<Engine, HostError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let kind = if file_name == "msedge.exe" || file_name == "msedge" {
        EngineKind::Edge
    } else if ["chrome.exe", "chrome", "chromium", "chromium-browser"].contains(&file_name.as_str())
    {
        EngineKind::Chromium
    } else {
        return Err(HostError::Unavailable(UnavailableReason::UnsupportedEngine));
    };
    if !path.is_file() {
        return Err(HostError::Unavailable(UnavailableReason::NoEngine));
    }
    let version = probe_version(path)?;
    let publisher_verified = verify_publisher(path)?;
    Ok(Engine {
        path: path.to_path_buf(),
        kind,
        version,
        publisher_verified,
    })
}

/// Run `<engine> --version` with a bounded wait and parse the version.
/// The probe inherits only a scrubbed environment.
pub fn probe_version(engine: &Path) -> Result<String, HostError> {
    let mut child = std::process::Command::new(engine)
        .arg("--version")
        .env_clear()
        .envs(probe_env())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| HostError::Unavailable(UnavailableReason::NoEngine))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(HostError::Unavailable(UnavailableReason::UnsupportedEngine));
                }
                let output = child
                    .wait_with_output()
                    .map_err(|_| HostError::Unavailable(UnavailableReason::LaunchFailed))?;
                let text = String::from_utf8_lossy(&output.stdout);
                return parse_version(&text)
                    .ok_or(HostError::Unavailable(UnavailableReason::UnsupportedEngine));
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    return Err(HostError::Unavailable(UnavailableReason::LaunchFailed));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err(HostError::Unavailable(UnavailableReason::LaunchFailed)),
        }
    }
}

fn parse_version(text: &str) -> Option<String> {
    let mut run = String::new();
    let mut runs = Vec::new();
    for ch in text.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() || (ch == '.' && !run.is_empty()) {
            run.push(ch);
        } else if !run.is_empty() {
            runs.push(std::mem::take(&mut run));
        }
    }
    runs.into_iter().find(|run| run.matches('.').count() >= 2)
}

/// Minimal scrubbed environment for engine processes. No caller, user, or
/// Deskal secret variable is inherited.
fn probe_env() -> Vec<(OsString, OsString)> {
    platform_probe_env()
}

#[cfg(windows)]
fn platform_probe_env() -> Vec<(OsString, OsString)> {
    let system_root =
        std::env::var_os("SystemRoot").unwrap_or_else(|| OsString::from("C:\\Windows"));
    let system32 = Path::new(&system_root).join("System32");
    let system_temp = Path::new(&system_root).join("Temp");
    vec![
        (OsString::from("SystemRoot"), system_root.clone()),
        (
            OsString::from("SystemDrive"),
            std::env::var_os("SystemDrive").unwrap_or_else(|| OsString::from("C:")),
        ),
        (
            OsString::from("PATH"),
            OsString::from(format!(
                "{};{}",
                system32.to_string_lossy(),
                system_root.to_string_lossy()
            )),
        ),
        (OsString::from("TEMP"), system_temp.clone().into_os_string()),
        (OsString::from("TMP"), system_temp.into_os_string()),
    ]
}

#[cfg(not(windows))]
fn platform_probe_env() -> Vec<(OsString, OsString)> {
    vec![
        (OsString::from("PATH"), OsString::from("/usr/bin:/bin")),
        (OsString::from("LANG"), OsString::from("C")),
    ]
}

#[cfg(windows)]
fn platform_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for name in ["msedge.exe", "chrome.exe"] {
        if let Some(path) = app_paths_executable(name) {
            out.push(path);
        }
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        out.push(Path::new(&program_files).join("Microsoft/Edge/Application/msedge.exe"));
        out.push(Path::new(&program_files).join("Google/Chrome/Application/chrome.exe"));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        out.push(Path::new(&program_files_x86).join("Microsoft/Edge/Application/msedge.exe"));
        out.push(Path::new(&program_files_x86).join("Google/Chrome/Application/chrome.exe"));
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(not(windows))]
fn platform_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in [
        "/usr/bin",
        "/usr/local/bin",
        "/opt/google/chrome",
        "/snap/bin",
    ] {
        out.extend(candidate_executables(Path::new(dir)));
    }
    // PATH lookup for the known names.
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for kind in [EngineKind::Edge, EngineKind::Chromium] {
                for name in kind.executable_names() {
                    let candidate = dir.join(name);
                    if candidate.is_file() {
                        out.push(candidate);
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
    use windows_sys::Win32::Security::WinTrust::{
        WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO,
        WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    };
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    pub(super) fn app_paths_executable(file_name: &str) -> Option<PathBuf> {
        let subkey =
            format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{file_name}");
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            let mut size: u32 = 0;
            // SAFETY: query the size first with a null buffer.
            let status = unsafe {
                RegGetValueW(
                    hive,
                    wide(&subkey).as_ptr(),
                    std::ptr::null(),
                    RRF_RT_REG_SZ,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut size,
                )
            };
            if status != ERROR_SUCCESS || size == 0 || size > 32_768 {
                continue;
            }
            let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
            let mut actual = (buffer.len() * 2) as u32;
            // SAFETY: buffer length is passed in `actual`.
            let status = unsafe {
                RegGetValueW(
                    hive,
                    wide(&subkey).as_ptr(),
                    std::ptr::null(),
                    RRF_RT_REG_SZ,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut _,
                    &mut actual,
                )
            };
            if status != ERROR_SUCCESS {
                continue;
            }
            let path = String::from_utf16_lossy(&buffer)
                .trim_matches('\0')
                .trim()
                .trim_matches('"')
                .to_owned();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
        None
    }

    pub(super) fn verify_publisher(path: &Path) -> Result<bool, HostError> {
        let wide_path = wide(&path.to_string_lossy());
        // SAFETY: all structures are zeroed then fully initialized below;
        // `data.u.pFile` matches `dwUnionChoice`, and every borrow outlives
        // the verification call, which performs no mutation.
        unsafe {
            let mut file_info: WINTRUST_FILE_INFO = std::mem::zeroed();
            file_info.cbStruct = std::mem::size_of::<WINTRUST_FILE_INFO>() as u32;
            file_info.pcwszFilePath = wide_path.as_ptr();
            let mut data: WINTRUST_DATA = std::mem::zeroed();
            data.cbStruct = std::mem::size_of::<WINTRUST_DATA>() as u32;
            data.dwUIChoice = WTD_UI_NONE;
            data.fdwRevocationChecks = WTD_REVOKE_NONE;
            data.dwUnionChoice = WTD_CHOICE_FILE;
            data.dwStateAction = WTD_STATEACTION_VERIFY;
            data.hWVTStateData = 0 as HANDLE;
            data.Anonymous.pFile = &mut file_info;
            let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
            Ok(WinVerifyTrust(0 as HANDLE, &mut action, &mut data as *mut _ as *mut c_void) == 0)
        }
    }
}

#[cfg(windows)]
fn verify_publisher(path: &Path) -> Result<bool, HostError> {
    if !windows::verify_publisher(path)? {
        return Err(HostError::Unavailable(UnavailableReason::UnsupportedEngine));
    }
    Ok(true)
}

#[cfg(not(windows))]
fn verify_publisher(_path: &Path) -> Result<bool, HostError> {
    // No OS publisher check is implemented off Windows: path, kind, and
    // version checks apply and the engine reports unverified. Recorded as
    // a platform boundary in the SG-000074 note.
    Ok(false)
}

#[cfg(windows)]
fn app_paths_executable(name: &str) -> Option<PathBuf> {
    windows::app_paths_executable(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn version_parsing_accepts_dotted_runs() {
        assert_eq!(
            parse_version("Microsoft Edge 120.0.2210.61").as_deref(),
            Some("120.0.2210.61")
        );
        assert_eq!(
            parse_version("Chromium 131.0.6778.0").as_deref(),
            Some("131.0.6778.0")
        );
        assert!(parse_version("no version here").is_none());
        assert!(parse_version("v2").is_none());
    }

    #[test]
    fn unknown_executables_are_unsupported() {
        let dir = std::env::temp_dir().join(format!("qdral-host-disc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("firefox.exe"), b"fake").unwrap();
        // Isolated scope: the machine may ship a real engine, which must
        // not satisfy this test either way.
        let search = SearchConfig::isolated(vec![dir.clone()]);
        assert!(matches!(
            discover_engine(&search),
            Err(HostError::Unavailable(
                UnavailableReason::NoEngine | UnavailableReason::UnsupportedEngine
            ))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_roots_report_no_engine_without_touching_the_machine() {
        // Extra roots only: a scratch empty directory proves the typed
        // unavailable path without depending on installed software.
        let dir = std::env::temp_dir().join(format!("qdral-host-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Point PATH-less discovery at the empty root by direct candidate
        // verification of a missing file.
        assert!(matches!(
            verify_candidate(&dir.join("chrome.exe")),
            Err(HostError::Unavailable(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }
}
