//! #309: what a host that died leaves behind, removed at the next launch.
//!
//! Each browser lifetime stages downloads in its own folder in the temp
//! directory, `qdral-sg108-staging-<pid>-<hex>`, and removes it at the end
//! of the lifetime. A host that is killed never gets there. At launch, the
//! next host removes every such folder whose owning process is no longer
//! running; a folder of a live host (this one included) is never touched.

use std::path::Path;
use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// The staging folder prefix (see `BrowserWorker`'s staging directory).
pub(crate) const STAGING_PREFIX: &str = "qdral-sg108-staging-";
/// Entries of the temp directory looked at, and folders removed, per launch.
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

/// Remove the staging folders of hosts that are no longer running; returns
/// how many were removed. Bounded, and never follows a reparse point: a
/// junction named like a staging folder is left alone.
pub(crate) fn sweep_stale_staging(temp_dir: &Path) -> usize {
    use std::os::windows::fs::MetadataExt;
    let Ok(entries) = std::fs::read_dir(temp_dir) else {
        return 0;
    };
    let own = std::process::id();
    let mut removed = 0;
    for entry in entries.take(MAX_ENTRIES).flatten() {
        if removed >= MAX_REMOVED {
            break;
        }
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(owner) else {
            continue;
        };
        if pid == own || may_be_running(pid) {
            continue;
        }
        let path = entry.path();
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
