//! SG-000108 decision 7 (amended in slice 3b): the worker runtime pin.
//!
//! A `live-browser-dev` build compiles in the SHA-256 of `node.exe` and a
//! digest of the worker's whole runtime tree, and every launch verifies
//! both against the files about to run. The tree, not just `main.js`:
//! `main.js` imports the rest of `dist/` and `playwright-core`, so pinning
//! the entry point alone would pin nothing it loads.
//!
//! The runtime tree is `package.json`, every file under `dist/` except
//! tests, and every file under `node_modules/playwright-core/`. Each file
//! is hashed through a handle opened without write or delete sharing, and
//! those handles stay open for the worker's whole lifetime (Node loads
//! modules lazily), so no file can be changed between the check and its
//! load.
//!
//! What the pin is not: in a dev build the host and `qdrald` binaries are
//! themselves files the user can write, so a same-user process that can
//! rewrite them is outside what it defends. It guarantees that local
//! configuration cannot point the worker at other code, and that the exact
//! pinned files run unmodified; SG-000106's signed payload replaces it.

use crate::error::{HostError, UnavailableReason};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Domain separation for the tree digest.
const TREE_DOMAIN: &str = "deskal-worker-runtime-tree/1";

/// What a build pinned: `node.exe`'s SHA-256 and the runtime tree digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePin {
    pub node_sha256: String,
    pub tree_sha256: String,
}

/// The runtime tree's files, relative to the worker root, sorted, with
/// `/` separators.
pub fn runtime_files(root: &Path) -> Result<Vec<String>, HostError> {
    let mut files = Vec::new();
    let package = root.join("package.json");
    if !plain_file(&package)? {
        return Err(unpinned());
    }
    files.push("package.json".to_owned());
    collect(root, "dist", &mut files, &|relative| {
        !relative.ends_with(".test.js")
    })?;
    collect(root, "node_modules/playwright-core", &mut files, &|_| true)?;
    files.sort();
    Ok(files)
}

fn unpinned() -> HostError {
    HostError::Unavailable(UnavailableReason::UnsupportedRuntime)
}

/// A regular file, not a link or any other reparse point.
fn plain_file(path: &Path) -> Result<bool, HostError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_file() && !reparse(&metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(unpinned()),
    }
}

#[cfg(windows)]
fn reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

/// Every regular file under `root/relative`; a link or reparse point
/// anywhere in it refuses the whole tree.
fn collect(
    root: &Path,
    relative: &str,
    files: &mut Vec<String>,
    keep: &dyn Fn(&str) -> bool,
) -> Result<(), HostError> {
    let dir = root.join(relative);
    let metadata = std::fs::symlink_metadata(&dir).map_err(|_| unpinned())?;
    if !metadata.is_dir() || reparse(&metadata) {
        return Err(unpinned());
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|_| unpinned())?
        .collect::<Result<_, _>>()
        .map_err(|_| unpinned())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().into_string().map_err(|_| unpinned())?;
        let child = format!("{relative}/{name}");
        let metadata = std::fs::symlink_metadata(entry.path()).map_err(|_| unpinned())?;
        if reparse(&metadata) {
            return Err(unpinned());
        }
        if metadata.is_dir() {
            collect(root, &child, files, keep)?;
        } else if metadata.is_file() {
            if keep(&child) {
                files.push(child);
            }
        } else {
            return Err(unpinned());
        }
    }
    Ok(())
}

/// The tree digest over (relative path, file SHA-256) pairs, in order.
pub fn tree_digest(entries: &[(String, String)]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(TREE_DOMAIN.as_bytes());
    hasher.update(b"\n");
    for (path, sha256) in entries {
        hasher.update(path.as_bytes());
        hasher.update(b"\n");
        hasher.update(sha256.as_bytes());
        hasher.update(b"\n");
    }
    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The runtime's files, held open without write or delete sharing. Dropped
/// with the worker.
#[derive(Debug)]
pub struct HeldRuntime {
    _files: Vec<std::fs::File>,
}

impl HeldRuntime {
    /// How many files are held (node.exe and the tree).
    pub fn len(&self) -> usize {
        self._files.len()
    }

    pub fn is_empty(&self) -> bool {
        self._files.is_empty()
    }
}

/// Open for reading, sharing only reads: no write, rename or delete while
/// it is held. Never follows a reparse point at the name.
#[cfg(windows)]
fn hold(path: &Path) -> Result<std::fs::File, HostError> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| unpinned())?;
    let metadata = file.metadata().map_err(|_| unpinned())?;
    if !metadata.is_file() || reparse(&metadata) {
        return Err(unpinned());
    }
    Ok(file)
}

#[cfg(windows)]
fn hash_held(file: &mut std::fs::File) -> Result<String, HostError> {
    use std::io::Read as _;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| unpinned())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

/// The worker root of an entry point `<root>/dist/main.js`.
pub fn worker_root(worker_main: &Path) -> Result<PathBuf, HostError> {
    let dist = worker_main.parent().ok_or_else(unpinned)?;
    if worker_main.file_name().and_then(|name| name.to_str()) != Some("main.js")
        || dist.file_name().and_then(|name| name.to_str()) != Some("dist")
    {
        return Err(unpinned());
    }
    dist.parent().map(Path::to_path_buf).ok_or_else(unpinned)
}

/// Hold `node.exe` and the runtime tree, hash each through its held handle,
/// and measure (node SHA-256, tree digest).
#[cfg(windows)]
pub fn measure_and_hold(
    node: &Path,
    worker_main: &Path,
) -> Result<(RuntimePin, HeldRuntime), HostError> {
    let root = worker_root(worker_main)?;
    let mut held = Vec::new();
    let mut node_file = hold(node)?;
    let node_sha256 = hash_held(&mut node_file)?;
    held.push(node_file);
    let mut entries = Vec::new();
    for relative in runtime_files(&root)? {
        let mut file = hold(&root.join(&relative))?;
        entries.push((relative, hash_held(&mut file)?));
        held.push(file);
    }
    Ok((
        RuntimePin {
            node_sha256,
            tree_sha256: tree_digest(&entries),
        },
        HeldRuntime { _files: held },
    ))
}

impl RuntimePin {
    /// Hold the runtime and require it to be exactly this pin. A changed
    /// node.exe or tree is refused, typed, until the build is redone.
    #[cfg(windows)]
    pub fn verify_and_hold(
        &self,
        node: &Path,
        worker_main: &Path,
    ) -> Result<HeldRuntime, HostError> {
        let (measured, held) = measure_and_hold(node, worker_main)?;
        if &measured != self {
            return Err(unpinned());
        }
        Ok(held)
    }
}

#[cfg(all(test, windows))]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn scratch(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "qdral-runtime-pin-{label}-{}-{nanos}",
            std::process::id()
        ));
        for dir in [
            "dist/extract",
            "node_modules/playwright-core/lib",
            "node_modules/typescript",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for (file, body) in [
            ("package.json", "{}"),
            ("dist/main.js", "import './worker.js';"),
            ("dist/worker.js", "export {};"),
            ("dist/worker.test.js", "test"),
            ("dist/extract/clean.js", "export {};"),
            ("node_modules/playwright-core/package.json", "{}"),
            ("node_modules/playwright-core/lib/index.js", "x"),
            ("node_modules/typescript/index.js", "not loaded"),
            ("node.exe", "MZ fake node"),
        ] {
            std::fs::write(root.join(file), body).unwrap();
        }
        root
    }

    #[test]
    fn the_tree_is_the_loadable_runtime_and_nothing_else() {
        let root = scratch("files");
        assert_eq!(
            runtime_files(&root).unwrap(),
            [
                "dist/extract/clean.js",
                "dist/main.js",
                "dist/worker.js",
                "node_modules/playwright-core/lib/index.js",
                "node_modules/playwright-core/package.json",
                "package.json",
            ]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_matching_runtime_is_held_and_any_change_is_refused() {
        let root = scratch("verify");
        let node = root.join("node.exe");
        let main = root.join("dist/main.js");
        let (pin, held) = measure_and_hold(&node, &main).unwrap();
        assert_eq!(held.len(), 7, "node.exe and six tree files");
        // Held: no file of the runtime can be written, renamed or deleted.
        assert!(std::fs::write(&main, "changed").is_err());
        assert!(std::fs::remove_file(root.join("dist/worker.js")).is_err());
        assert!(std::fs::rename(&node, root.join("other.exe")).is_err());
        drop(held);
        let again = pin.verify_and_hold(&node, &main).unwrap();
        drop(again);
        // Any change to a pinned file, or a new loadable file, is refused.
        for (file, body) in [
            ("dist/worker.js", "export const changed = 1;"),
            ("node_modules/playwright-core/lib/added.js", "x"),
            ("node.exe", "MZ other node"),
        ] {
            let saved = std::fs::read(root.join(file)).ok();
            std::fs::write(root.join(file), body).unwrap();
            assert!(pin.verify_and_hold(&node, &main).is_err(), "{file}");
            match saved {
                Some(bytes) => std::fs::write(root.join(file), bytes).unwrap(),
                None => std::fs::remove_file(root.join(file)).unwrap(),
            }
        }
        // Tests and other packages are not part of the runtime.
        std::fs::write(root.join("dist/worker.test.js"), "changed").unwrap();
        std::fs::write(root.join("node_modules/typescript/index.js"), "changed").unwrap();
        assert!(pin.verify_and_hold(&node, &main).is_ok());
        std::fs::remove_dir_all(&root).unwrap();
    }

    // A junction inside the tree is refused: tested in worker_launch_tests.rs
    // (`runtime_pin_refuses_a_linked_tree`), the one place test processes
    // are spawned (SG-000085).

    #[test]
    fn the_digest_frames_paths_and_hashes() {
        let a = tree_digest(&[("a".into(), "1".into()), ("b".into(), "2".into())]);
        assert_ne!(
            a,
            tree_digest(&[("ab".into(), "1".into()), ("".into(), "2".into())])
        );
        assert_ne!(
            a,
            tree_digest(&[("b".into(), "2".into()), ("a".into(), "1".into())])
        );
        // Fixed vector, independently computed (Python sha256).
        assert_eq!(
            tree_digest(&[("package.json".into(), "00".into())]),
            FIXED_VECTOR
        );
    }

    const FIXED_VECTOR: &str = "59c95add357adeef22d144b646ccdd87857db7b68c3975f8d8ce9824831f2ae4";
}
