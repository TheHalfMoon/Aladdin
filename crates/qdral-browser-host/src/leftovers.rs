//! #309: what a host that died leaves behind, removed at the next launch.
//!
//! Each browser lifetime stages downloads in its own folder in the temp
//! directory, `qdral-sg108-staging-<pid>-<hex>`, and removes it at the end
//! of the lifetime. A host that is killed never gets there. At launch, the
//! next host removes every such folder whose owning process is no longer
//! running; a folder of a live host (this one included) is never touched.

use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE, STILL_ACTIVE};
use windows_sys::Win32::Storage::FileSystem::{
    FindClose, FindFirstFileW, FindNextFileW, FILE_ATTRIBUTE_REPARSE_POINT, WIN32_FIND_DATAW,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// The staging folder prefix (see `BrowserWorker`'s staging directory).
pub(crate) const STAGING_PREFIX: &str = "qdral-sg108-staging-";
/// Staging-named entries of the temp directory looked at (only names
/// matching the prefix are listed, however crowded the directory), and
/// folders removed, per launch.
const MAX_ENTRIES: usize = 4096;
const MAX_REMOVED: usize = 64;

/// Whether `pid` may still be running. A process that cannot be queried
/// for any other reason than not existing counts as running, so a folder
/// is removed only when its owner is certainly gone.
fn may_be_running(pid: u32) -> bool {
    // SAFETY: OpenProcess takes no pointers; a null result is handled.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        // ERROR_INVALID_PARAMETER (87): no such process.
        return std::io::Error::last_os_error().raw_os_error() != Some(87);
    }
    let mut code = 0u32;
    // SAFETY: `handle` is a process handle opened above; `code` is writable.
    let queried = unsafe { GetExitCodeProcess(handle, &mut code) };
    // SAFETY: `handle` was opened above and is closed exactly once.
    unsafe { CloseHandle(handle) };
    queried == 0 || code == STILL_ACTIVE as u32
}

/// The owning process of a staging folder name, if it is one.
fn owner(name: &str) -> Option<u32> {
    let rest = name.strip_prefix(STAGING_PREFIX)?;
    let (pid, suffix) = rest.split_once('-')?;
    if pid.is_empty()
        || !pid.bytes().all(|b| b.is_ascii_digit())
        || suffix.is_empty()
        || !suffix.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    pid.parse().ok()
}

/// The names in `dir` that match `STAGING_PREFIX*`, at most MAX_ENTRIES.
fn staging_names(dir: &Path) -> Vec<String> {
    let mut pattern: Vec<u16> = dir
        .join(format!("{STAGING_PREFIX}*"))
        .as_os_str()
        .encode_wide()
        .collect();
    pattern.push(0);
    // SAFETY: an all-zero WIN32_FIND_DATAW is a valid output buffer.
    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    // SAFETY: `pattern` is NUL-terminated and outlives the call; `data` is
    // writable.
    let find = unsafe { FindFirstFileW(pattern.as_ptr(), &mut data) };
    if find == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let mut names = Vec::new();
    loop {
        let length = data
            .cFileName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(data.cFileName.len());
        names.push(
            std::ffi::OsString::from_wide(&data.cFileName[..length])
                .to_string_lossy()
                .into_owned(),
        );
        // SAFETY: `find` is the open search handle; `data` is writable.
        if names.len() >= MAX_ENTRIES || unsafe { FindNextFileW(find, &mut data) } == 0 {
            break;
        }
    }
    // SAFETY: `find` was opened above and is closed exactly once.
    unsafe { FindClose(find) };
    names
}

/// Remove the staging folders of hosts that are no longer running; returns
/// how many were removed. Bounded, and never follows a reparse point: a
/// junction named like a staging folder is left alone.
pub(crate) fn sweep_stale_staging(temp_dir: &Path) -> usize {
    use std::os::windows::fs::MetadataExt;
    let own = std::process::id();
    let mut removed = 0;
    for name in staging_names(temp_dir) {
        if removed >= MAX_REMOVED {
            break;
        }
        let Some(pid) = owner(&name) else {
            continue;
        };
        if pid == own || may_be_running(pid) {
            continue;
        }
        let path: PathBuf = temp_dir.join(&name);
        let plain_folder = std::fs::symlink_metadata(&path).is_ok_and(|meta| {
            meta.is_dir() && meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
        });
        // `remove_dir_all` does not follow reparse points inside the folder.
        if plain_folder && std::fs::remove_dir_all(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pid no process can have: Windows pids are multiples of four far
    /// below this one (no test process is spawned; that lives only in
    /// worker_launch_tests.rs).
    fn dead_pid() -> u32 {
        0xFFFF_FFFC
    }

    #[test]
    fn staging_names_are_parsed_strictly() {
        assert_eq!(owner("qdral-sg108-staging-1234-ab12"), Some(1234));
        for name in [
            "qdral-sg108-staging-1234",
            "qdral-sg108-staging--ab",
            "qdral-sg108-staging-12x-ab",
            "qdral-sg108-staging-12-xyz",
            "other-1234-ab",
        ] {
            assert_eq!(owner(name), None, "{name}");
        }
    }

    #[test]
    fn only_folders_of_hosts_that_are_gone_are_removed() {
        let temp = std::env::temp_dir().join(format!(
            "qdral-309-sweep-{}-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).unwrap();
        let dead = dead_pid();
        assert!(!may_be_running(dead), "the probe pid is still running");
        let stale = temp.join(format!("{STAGING_PREFIX}{dead}-ab12"));
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("dl-1.part"), b"half").unwrap();
        let mine = temp.join(format!("{STAGING_PREFIX}{}-cd34", std::process::id()));
        std::fs::create_dir_all(&mine).unwrap();
        let unrelated = temp.join("someone-else");
        std::fs::create_dir_all(&unrelated).unwrap();
        // A junction named like a stale staging folder, to a folder outside:
        // never followed, never removed.
        let outside = temp.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("keep.txt"), b"keep").unwrap();
        let link = temp.join(format!("{STAGING_PREFIX}{dead}-ef56"));
        crate::approved_folder::tests::junction(&link, &outside);

        // A crowded directory does not starve the sweep: entries that sort
        // before the prefix are never listed.
        for i in 0..MAX_ENTRIES + 10 {
            std::fs::write(temp.join(format!("a-{i:05}")), b"").unwrap();
        }
        assert_eq!(sweep_stale_staging(&temp), 1);
        assert!(!stale.exists(), "the dead host's folder stayed");
        assert!(mine.exists(), "a live host's folder was removed");
        assert!(unrelated.exists());
        assert!(link.exists());
        assert_eq!(std::fs::read(outside.join("keep.txt")).unwrap(), b"keep");
        std::fs::remove_dir(&link).unwrap();
        std::fs::remove_dir_all(&temp).unwrap();
    }
}
