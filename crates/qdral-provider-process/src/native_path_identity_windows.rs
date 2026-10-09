//! Read-only SG96 Windows file-handle qualification: test-only, no ShellProcess.
use sha2::{Digest, Sha256};
use std::ffi::c_void;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SHARE_READ: u32 = 1;
const OPEN_REPARSE: u32 = 0x0020_0000;
const BACKUP_SEMANTICS: u32 = 0x0200_0000;
const REPARSE_ATTRIBUTE: u32 = 0x400;
const MAX_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileKey {
    volume: u32,
    index: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NativeFileTime {
    low: u32,
    high: u32,
}

#[repr(C)]
#[derive(Default)]
struct NativeHandleInfo {
    attributes: u32,
    created: NativeFileTime,
    accessed: NativeFileTime,
    written: NativeFileTime,
    volume: u32,
    size_high: u32,
    size_low: u32,
    link_count: u32,
    index_high: u32,
    index_low: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileInformationByHandle(handle: *mut c_void, data: *mut NativeHandleInfo) -> i32;
}

fn key(file: &File) -> Result<FileKey, String> {
    let mut info = NativeHandleInfo::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(format!(
            "native handle file ID query failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(FileKey {
        volume: info.volume,
        index: ((info.index_high as u64) << 32) | info.index_low as u64,
    })
}

fn checked_components(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("target path must be absolute".into());
    }
    let mut prefix = PathBuf::new();
    for part in path.components() {
        prefix.push(part.as_os_str());
        if !prefix.is_absolute() {
            continue;
        }
        let md = fs::symlink_metadata(&prefix).map_err(|e| e.to_string())?;
        if md.file_attributes() & REPARSE_ATTRIBUTE != 0 {
            return Err("reparse or junction component is not authorized".into());
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    File,
    Directory,
}

struct HeldPath {
    path: PathBuf,
    kind: Kind,
    key: FileKey,
    sha: Option<[u8; 32]>,
    handle: File,
}

fn digest(file: &mut File) -> Result<[u8; 32], String> {
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut sum = Sha256::new();
    let mut buf = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .ok_or("digest byte counter overflow")?;
        if total > MAX_BYTES {
            return Err("unbounded executable hash denied".into());
        }
        sum.update(&buf[..n]);
    }
    Ok(sum.finalize().into())
}

impl HeldPath {
    fn open(path: &Path, kind: Kind) -> Result<Self, String> {
        checked_components(path)?;
        let mut handle = OpenOptions::new()
            .read(true)
            .share_mode(SHARE_READ)
            .custom_flags(
                OPEN_REPARSE
                    | if kind == Kind::Directory {
                        BACKUP_SEMANTICS
                    } else {
                        0
                    },
            )
            .open(path)
            .map_err(|e| e.to_string())?;
        let md = handle.metadata().map_err(|e| e.to_string())?;
        if md.file_attributes() & REPARSE_ATTRIBUTE != 0
            || (kind == Kind::File && !md.is_file())
            || (kind == Kind::Directory && !md.is_dir())
        {
            return Err("wrong native target kind or reparse handle".into());
        }
        let native_key = key(&handle)?;
        let sha = if kind == Kind::File {
            Some(digest(&mut handle)?)
        } else {
            None
        };
        Ok(Self {
            path: path.to_path_buf(),
            kind,
            key: native_key,
            sha,
            handle,
        })
    }

    fn verify(
        &mut self,
        path: &Path,
        expected: FileKey,
        hash: Option<[u8; 32]>,
    ) -> Result<(), String> {
        if path != self.path {
            return Err("lexical native path drift".into());
        }
        checked_components(path)?;
        let held = key(&self.handle)?;
        if held != self.key || held != expected {
            return Err("held native file ID drift".into());
        }
        let reopened = Self::open(path, self.kind)?;
        if reopened.key != self.key {
            return Err("path rebound to a different file".into());
        }
        if self.kind == Kind::File {
            let actual = digest(&mut self.handle)?;
            if Some(actual) != hash || Some(actual) != self.sha || reopened.sha != self.sha {
                return Err("executable hash mismatch".into());
            }
        } else if hash.is_some() {
            return Err("directory cannot have executable hash".into());
        }
        Ok(())
    }
}

struct TemporaryRoot(PathBuf);
impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn sg000096_windows_native_file_identity_and_digest_deny_drift() {
    let ticks = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("deskal-native-path-{}-{ticks}", std::process::id()));
    fs::create_dir_all(&root).expect("create isolated test root");
    let _cleanup = TemporaryRoot(root.clone());
    let exe = root.join("disposable.bin");
    let cwd = root.join("cwd");
    fs::write(&exe, b"disposable fixture bytes a").expect("create synthetic binary");
    fs::create_dir(&cwd).expect("create cwd");

    let mut pinned = HeldPath::open(&exe, Kind::File).expect("open dummy file");
    let mut directory = HeldPath::open(&cwd, Kind::Directory).expect("open native directory");
    let native_key = pinned.key;
    let cwd_key = directory.key;
    let hash = pinned.sha.expect("hash is present");
    pinned
        .verify(&exe, native_key, Some(hash))
        .expect("matching file ID and bytes");
    directory
        .verify(&cwd, cwd_key, None)
        .expect("matching working directory ID");
    let mut wrong_hash = hash;
    wrong_hash[0] ^= 1;
    assert!(pinned.verify(&exe, native_key, Some(wrong_hash)).is_err());
    let wrong_key = FileKey {
        index: native_key.index ^ 1,
        ..native_key
    };
    assert!(pinned.verify(&exe, wrong_key, Some(hash)).is_err());
    assert!(pinned.verify(&cwd, native_key, Some(hash)).is_err());
    assert!(HeldPath::open(&exe, Kind::Directory).is_err());
    assert!(HeldPath::open(&cwd, Kind::File).is_err());
    assert!(HeldPath::open(Path::new("relative.bin"), Kind::File).is_err());
    assert!(directory.verify(&cwd, cwd_key, Some(hash)).is_err());
    drop(pinned);
    drop(directory);
    fs::write(&exe, b"disposable fixture bytes b").expect("mutate only after handle release");
    let mut changed = HeldPath::open(&exe, Kind::File).expect("reopen altered file");
    let changed_key = changed.key;
    assert!(changed.verify(&exe, changed_key, Some(hash)).is_err());
    // A native handle is measured; no process or ShellProcess is launched.
}
