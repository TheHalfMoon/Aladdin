//! Host operations the installer depends on. Every result is derived from a
//! real OS query; a check that cannot run is an error, never a success.

use crate::LifecycleError;
use std::path::Path;

/// Minimum supported Windows build (Windows 10 1809).
pub const MIN_WINDOWS_BUILD: u32 = 17_763;
/// Minimum supported Node.js major version.
pub const MIN_NODE_MAJOR: u64 = 20;

pub trait Platform {
    /// Windows build number of the running OS.
    fn windows_build(&self) -> Result<u32, LifecycleError>;
    /// Whether the process runs with an avoidable elevated token: a UAC
    /// split-token administrator who chose "Run as administrator". Systems
    /// without a split token (UAC disabled) cannot drop elevation and are not
    /// reported as avoidably elevated.
    fn is_avoidably_elevated(&self) -> Result<bool, LifecycleError>;
    /// Applies the owner-only protected DACL to `root` and resets every
    /// descendant to inherit it.
    fn protect_tree(&self, root: &Path) -> Result<(), LifecycleError>;
    /// Verifies that `root` has a protected DACL and that `root` and every
    /// descendant grant access only to the current user and SYSTEM.
    fn verify_tree_acl(&self, root: &Path) -> Result<(), LifecycleError>;
    /// Adds `dir` to the user `PATH`; returns whether an entry was added.
    fn add_user_path(&self, dir: &Path) -> Result<bool, LifecycleError>;
    /// Removes `dir` from the user `PATH`; returns whether an entry was removed.
    fn remove_user_path(&self, dir: &Path) -> Result<bool, LifecycleError>;
    /// Runs `node --version` and returns the parsed version.
    fn node_version(&self, node: &Path) -> Result<crate::version::Version, LifecycleError>;
}

/// Parses `node --version` output such as `v24.19.0`.
pub fn parse_node_version(output: &str) -> Result<crate::version::Version, LifecycleError> {
    let trimmed = output.trim();
    let text = trimmed.strip_prefix('v').ok_or_else(|| {
        LifecycleError::prerequisite(format!("unexpected Node.js version output {trimmed:?}"))
    })?;
    crate::version::Version::parse(text).map_err(|_| {
        LifecycleError::prerequisite(format!("unexpected Node.js version {trimmed:?}"))
    })
}

/// Locates `node.exe` (or `node` off Windows) on `PATH`.
pub fn find_node_on_path() -> Option<std::path::PathBuf> {
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

pub(crate) fn run_node_version(node: &Path) -> Result<crate::version::Version, LifecycleError> {
    if !node.is_absolute() || !node.is_file() {
        return Err(LifecycleError::prerequisite(format!(
            "Node.js runtime is not an existing absolute file: {}",
            node.display()
        )));
    }
    let output = std::process::Command::new(node)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|error| LifecycleError::prerequisite(format!("run Node.js: {error}")))?;
    if !output.status.success() {
        return Err(LifecycleError::prerequisite("Node.js --version failed"));
    }
    parse_node_version(&String::from_utf8_lossy(&output.stdout))
}

/// Normalizes a `PATH` entry for comparison.
pub fn path_entry_key(entry: &str) -> String {
    entry
        .trim()
        .trim_matches('"')
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase()
}

/// Returns `path` with `dir` appended unless an equivalent entry exists.
pub fn path_with_entry(path: &str, dir: &str) -> Option<String> {
    let key = path_entry_key(dir);
    if path.split(';').any(|entry| path_entry_key(entry) == key) {
        return None;
    }
    let trimmed = path.trim_end_matches(';');
    Some(if trimmed.is_empty() {
        dir.to_string()
    } else {
        format!("{trimmed};{dir}")
    })
}

/// Returns `path` without entries equivalent to `dir`, or `None` if absent.
pub fn path_without_entry(path: &str, dir: &str) -> Option<String> {
    let key = path_entry_key(dir);
    let entries = path.split(';').collect::<Vec<_>>();
    let kept = entries
        .iter()
        .copied()
        .filter(|entry| path_entry_key(entry) != key)
        .collect::<Vec<_>>();
    (kept.len() != entries.len()).then(|| kept.join(";"))
}

/// Validates an SDDL DACL string: `protected` requires the `P` flag, and
/// every ACE must be an allow ACE for `user_sid` or SYSTEM.
pub fn check_sddl(sddl: &str, user_sid: &str, protected: bool) -> Result<(), String> {
    let body = sddl
        .strip_prefix("D:")
        .ok_or_else(|| "security descriptor has no DACL".to_string())?;
    let flags_end = body.find('(').unwrap_or(body.len());
    let flags = &body[..flags_end];
    if protected && !flags.contains('P') {
        return Err("DACL is not protected from inheritance".into());
    }
    let aces = &body[flags_end..];
    if aces.is_empty() {
        return Err("DACL has no entries".into());
    }
    for ace in aces.split(')').filter(|part| !part.is_empty()) {
        let ace = ace
            .strip_prefix('(')
            .ok_or_else(|| "malformed ACE".to_string())?;
        let fields = ace.split(';').collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err("malformed ACE".into());
        }
        if fields[0] != "A" {
            return Err(format!("unexpected ACE type {}", fields[0]));
        }
        let sid = fields[5];
        if sid != "SY" && !sid.eq_ignore_ascii_case(user_sid) {
            return Err(format!("unexpected principal {sid}"));
        }
    }
    Ok(())
}

#[cfg(windows)]
pub use windows::WindowsPlatform;

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows_sys::Win32::Foundation::{
        CloseHandle, LocalFree, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HANDLE,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
        ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
        ProgressInvokeNever, TreeResetNamedSecurityInfoW, SDDL_REVISION_1, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        GetSecurityDescriptorDacl, GetTokenInformation, TokenElevationType, TokenElevationTypeFull,
        TokenUser, ACL, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, TOKEN_ELEVATION_TYPE, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(info: *mut OSVERSIONINFOW) -> i32;
    }

    /// Windows host operations. `environment_key` is the `HKCU` subkey whose
    /// `Path` value is edited; production uses `Environment`, tests use an
    /// isolated key.
    #[derive(Debug, Clone)]
    pub struct WindowsPlatform {
        environment_key: String,
    }

    impl Default for WindowsPlatform {
        fn default() -> Self {
            Self {
                environment_key: "Environment".into(),
            }
        }
    }

    impl WindowsPlatform {
        pub fn with_environment_key(key: impl Into<String>) -> Self {
            Self {
                environment_key: key.into(),
            }
        }

        fn is_production_environment(&self) -> bool {
            self.environment_key == "Environment"
        }
    }

    fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(Some(0)).collect()
    }

    fn from_wide_ptr(ptr: *const u16) -> String {
        let mut len = 0;
        unsafe {
            while *ptr.add(len) != 0 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
        }
    }

    fn last_error(context: &str) -> LifecycleError {
        LifecycleError::platform(format!("{context}: {}", std::io::Error::last_os_error()))
    }

    fn win32_error(context: &str, code: u32) -> LifecycleError {
        LifecycleError::platform(format!(
            "{context}: {}",
            std::io::Error::from_raw_os_error(code as i32)
        ))
    }

    struct Token(HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    fn process_token() -> Result<Token, LifecycleError> {
        let mut handle: HANDLE = 0;
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) } == 0 {
            return Err(last_error("OpenProcessToken"));
        }
        Ok(Token(handle))
    }

    fn token_info(token: &Token, class: i32) -> Result<Vec<u8>, LifecycleError> {
        let mut needed = 0u32;
        unsafe { GetTokenInformation(token.0, class, ptr::null_mut(), 0, &mut needed) };
        if needed == 0 {
            return Err(last_error("GetTokenInformation size"));
        }
        let mut buffer = vec![0u8; needed as usize];
        if unsafe {
            GetTokenInformation(
                token.0,
                class,
                buffer.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        } == 0
        {
            return Err(last_error("GetTokenInformation"));
        }
        Ok(buffer)
    }

    pub(crate) fn current_user_sid() -> Result<String, LifecycleError> {
        let token = process_token()?;
        let buffer = token_info(&token, TokenUser)?;
        let user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
        let mut text: *mut u16 = ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
            return Err(last_error("ConvertSidToStringSidW"));
        }
        let sid = from_wide_ptr(text);
        unsafe { LocalFree(text as _) };
        Ok(sid)
    }

    fn dacl_sddl(path: &Path) -> Result<String, LifecycleError> {
        let name = wide(path.as_os_str());
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        let status = unsafe {
            GetNamedSecurityInfoW(
                name.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                &mut descriptor,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(win32_error(
                &format!("read ACL of {}", path.display()),
                status,
            ));
        }
        let mut text: *mut u16 = ptr::null_mut();
        let ok = unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut text,
                ptr::null_mut(),
            )
        };
        unsafe { LocalFree(descriptor as _) };
        if ok == 0 {
            return Err(last_error(
                "ConvertSecurityDescriptorToStringSecurityDescriptorW",
            ));
        }
        let sddl = from_wide_ptr(text);
        unsafe { LocalFree(text as _) };
        Ok(sddl)
    }

    fn walk(path: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), LifecycleError> {
        let entries = std::fs::read_dir(path)
            .map_err(|error| LifecycleError::io(format!("read {}", path.display()), error))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| LifecycleError::io(format!("read {}", path.display()), error))?;
            let child = entry.path();
            let metadata = std::fs::symlink_metadata(&child).map_err(|error| {
                LifecycleError::io(format!("inspect {}", child.display()), error)
            })?;
            out.push(child.clone());
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                walk(&child, out)?;
            }
        }
        Ok(())
    }

    impl Platform for WindowsPlatform {
        fn windows_build(&self) -> Result<u32, LifecycleError> {
            let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
            info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
            let status = unsafe { RtlGetVersion(&mut info) };
            if status != 0 {
                return Err(LifecycleError::platform("RtlGetVersion failed"));
            }
            Ok(info.dwBuildNumber)
        }

        fn is_avoidably_elevated(&self) -> Result<bool, LifecycleError> {
            let token = process_token()?;
            let buffer = token_info(&token, TokenElevationType)?;
            let kind = unsafe { *(buffer.as_ptr() as *const TOKEN_ELEVATION_TYPE) };
            Ok(kind == TokenElevationTypeFull)
        }

        fn protect_tree(&self, root: &Path) -> Result<(), LifecycleError> {
            let sid = current_user_sid()?;
            let sddl = format!("D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)");
            let sddl_wide = wide(std::ffi::OsStr::new(&sddl));
            let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
            if unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl_wide.as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            } == 0
            {
                return Err(last_error(
                    "ConvertStringSecurityDescriptorToSecurityDescriptorW",
                ));
            }
            let mut present = 0;
            let mut defaulted = 0;
            let mut dacl: *mut ACL = ptr::null_mut();
            let got = unsafe {
                GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
            };
            if got == 0 || present == 0 || dacl.is_null() {
                unsafe { LocalFree(descriptor as _) };
                return Err(LifecycleError::platform(
                    "owner-only DACL could not be built",
                ));
            }
            let name = wide(root.as_os_str());
            let status = unsafe {
                TreeResetNamedSecurityInfoW(
                    name.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    dacl,
                    ptr::null(),
                    0,
                    None,
                    ProgressInvokeNever,
                    ptr::null::<c_void>(),
                )
            };
            unsafe { LocalFree(descriptor as _) };
            if status != ERROR_SUCCESS {
                return Err(win32_error(&format!("protect {}", root.display()), status));
            }
            Ok(())
        }

        fn verify_tree_acl(&self, root: &Path) -> Result<(), LifecycleError> {
            let sid = current_user_sid()?;
            check_sddl(&dacl_sddl(root)?, &sid, true).map_err(|reason| {
                LifecycleError::platform(format!("{}: {reason}", root.display()))
            })?;
            let mut children = Vec::new();
            walk(root, &mut children)?;
            for child in children {
                check_sddl(&dacl_sddl(&child)?, &sid, false).map_err(|reason| {
                    LifecycleError::platform(format!("{}: {reason}", child.display()))
                })?;
            }
            Ok(())
        }

        fn add_user_path(&self, dir: &Path) -> Result<bool, LifecycleError> {
            let dir = dir.to_string_lossy().into_owned();
            let key = RegKey::open(&self.environment_key)?;
            let (current, kind) = key.read_path()?;
            match path_with_entry(&current.unwrap_or_default(), &dir) {
                Some(updated) => {
                    key.write_path(&updated, kind.unwrap_or(REG_EXPAND_SZ))?;
                    drop(key);
                    if self.is_production_environment() {
                        broadcast_environment_change();
                    }
                    Ok(true)
                }
                None => Ok(false),
            }
        }

        fn remove_user_path(&self, dir: &Path) -> Result<bool, LifecycleError> {
            let dir = dir.to_string_lossy().into_owned();
            let key = RegKey::open(&self.environment_key)?;
            let (current, kind) = key.read_path()?;
            let Some(current) = current else {
                return Ok(false);
            };
            match path_without_entry(&current, &dir) {
                Some(updated) => {
                    key.write_path(&updated, kind.unwrap_or(REG_EXPAND_SZ))?;
                    drop(key);
                    if self.is_production_environment() {
                        broadcast_environment_change();
                    }
                    Ok(true)
                }
                None => Ok(false),
            }
        }

        fn node_version(&self, node: &Path) -> Result<crate::version::Version, LifecycleError> {
            run_node_version(node)
        }
    }

    struct RegKey(HKEY);

    impl RegKey {
        fn open(subkey: &str) -> Result<Self, LifecycleError> {
            let name = wide(std::ffi::OsStr::new(subkey));
            let mut key: HKEY = 0;
            let status = unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    name.as_ptr(),
                    0,
                    ptr::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    ptr::null(),
                    &mut key,
                    ptr::null_mut(),
                )
            };
            if status != ERROR_SUCCESS {
                return Err(win32_error("open user environment key", status));
            }
            Ok(Self(key))
        }

        fn read_path(&self) -> Result<(Option<String>, Option<u32>), LifecycleError> {
            let name = wide(std::ffi::OsStr::new("Path"));
            let mut kind = 0u32;
            let mut size = 0u32;
            let status = unsafe {
                RegQueryValueExW(
                    self.0,
                    name.as_ptr(),
                    ptr::null(),
                    &mut kind,
                    ptr::null_mut(),
                    &mut size,
                )
            };
            if status == ERROR_FILE_NOT_FOUND {
                return Ok((None, None));
            }
            if status != ERROR_SUCCESS {
                return Err(win32_error("read user Path", status));
            }
            if kind != REG_SZ && kind != REG_EXPAND_SZ {
                return Err(LifecycleError::platform("user Path has an unexpected type"));
            }
            let mut buffer = vec![0u16; (size as usize).div_ceil(2) + 1];
            let mut size_bytes = (buffer.len() * 2) as u32;
            let status = unsafe {
                RegQueryValueExW(
                    self.0,
                    name.as_ptr(),
                    ptr::null(),
                    &mut kind,
                    buffer.as_mut_ptr().cast(),
                    &mut size_bytes,
                )
            };
            if status != ERROR_SUCCESS {
                return Err(win32_error("read user Path", status));
            }
            let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            Ok((Some(String::from_utf16_lossy(&buffer[..len])), Some(kind)))
        }

        fn write_path(&self, value: &str, kind: u32) -> Result<(), LifecycleError> {
            let name = wide(std::ffi::OsStr::new("Path"));
            let data = wide(std::ffi::OsStr::new(value));
            let status = unsafe {
                RegSetValueExW(
                    self.0,
                    name.as_ptr(),
                    0,
                    kind,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                )
            };
            if status != ERROR_SUCCESS {
                return Err(win32_error("write user Path", status));
            }
            Ok(())
        }
    }

    impl Drop for RegKey {
        fn drop(&mut self) {
            unsafe { RegCloseKey(self.0) };
        }
    }

    fn broadcast_environment_change() {
        let name = wide(std::ffi::OsStr::new("Environment"));
        let mut result = 0usize;
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                name.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5_000,
                &mut result,
            )
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_version_output_is_parsed_strictly() {
        assert_eq!(parse_node_version("v24.19.0\r\n").unwrap().major, 24);
        assert!(parse_node_version("24.19.0").is_err());
        assert!(parse_node_version("vX").is_err());
    }

    #[test]
    fn path_entries_are_added_once_and_removed_exactly() {
        let dir = r"C:\Users\u\AppData\Local\Cotra\bin";
        assert_eq!(path_with_entry("", dir).unwrap(), dir);
        let added = path_with_entry(r"C:\a;C:\b;", dir).unwrap();
        assert_eq!(added, format!(r"C:\a;C:\b;{dir}"));
        assert!(path_with_entry(&added, &dir.to_ascii_uppercase()).is_none());
        assert!(path_with_entry(&format!(r"C:\a;{dir}\"), dir).is_none());
        assert_eq!(
            path_without_entry(&format!(r"C:\a;{dir};C:\b"), dir).unwrap(),
            r"C:\a;C:\b"
        );
        assert!(path_without_entry(r"C:\a;C:\Cotra\binx", dir).is_none());
    }

    #[test]
    fn sddl_check_requires_protection_and_known_principals() {
        let sid = "S-1-5-21-1-2-3-1001";
        assert!(check_sddl(
            &format!("D:PAI(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)"),
            sid,
            true
        )
        .is_ok());
        assert!(check_sddl(
            &format!("D:AI(A;OICIID;FA;;;{sid})(A;OICIID;FA;;;SY)"),
            sid,
            false
        )
        .is_ok());
        assert!(check_sddl(&format!("D:AI(A;OICI;FA;;;{sid})"), sid, true).is_err());
        assert!(check_sddl(
            &format!("D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;BU)"),
            sid,
            true
        )
        .is_err());
        assert!(check_sddl(&format!("D:P(A;OICI;FA;;;{sid})(A;;FA;;;WD)"), sid, true).is_err());
        assert!(check_sddl(&format!("D:P(D;OICI;FA;;;{sid})"), sid, true).is_err());
        assert!(check_sddl("D:P", sid, true).is_err());
        assert!(check_sddl("O:SY", sid, true).is_err());
    }
}
