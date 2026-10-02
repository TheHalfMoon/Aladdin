#![cfg(windows)]

use core::ffi::c_void;
use quntal_provider_process::{build_execution_plan, execute_contained, ExecutionLimits};
use std::collections::BTreeMap;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

type Psid = *mut c_void;
type Pacl = *mut c_void;
type PsecurityDescriptor = *mut c_void;

const SE_FILE_OBJECT: u32 = 1;
const DACL_SECURITY_INFORMATION: u32 = 0x0000_0004;
const GRANT_ACCESS: u32 = 1;
const NO_MULTIPLE_TRUSTEE: u32 = 0;
const TRUSTEE_IS_SID: u32 = 0;
const TRUSTEE_IS_UNKNOWN: u32 = 0;
const SUB_CONTAINERS_AND_OBJECTS_INHERIT: u32 = 0x0000_0003;
const FILE_GENERIC_READ_ACCESS: u32 = 0x0012_0089;
const FILE_GENERIC_WRITE_ACCESS: u32 = 0x0012_0116;
const FILE_GENERIC_EXECUTE_ACCESS: u32 = 0x0012_00A0;

#[repr(C)]
struct TrusteeW {
    multiple_trustee: *mut TrusteeW,
    multiple_trustee_operation: u32,
    trustee_form: u32,
    trustee_type: u32,
    name: *mut u16,
}

#[repr(C)]
struct ExplicitAccessW {
    access_permissions: u32,
    access_mode: u32,
    inheritance: u32,
    trustee: TrusteeW,
}

#[link(name = "userenv")]
unsafe extern "system" {
    fn DeriveAppContainerSidFromAppContainerName(
        app_container_name: *const u16,
        app_container_sid: *mut Psid,
    ) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn FreeSid(sid: Psid) -> *mut c_void;
    fn GetNamedSecurityInfoW(
        object_name: *mut u16,
        object_type: u32,
        security_info: u32,
        owner: *mut Psid,
        group: *mut Psid,
        dacl: *mut Pacl,
        sacl: *mut Pacl,
        security_descriptor: *mut PsecurityDescriptor,
    ) -> u32;
    fn SetEntriesInAclW(
        explicit_entry_count: u32,
        explicit_entries: *const ExplicitAccessW,
        old_acl: Pacl,
        new_acl: *mut Pacl,
    ) -> u32;
    fn SetNamedSecurityInfoW(
        object_name: *mut u16,
        object_type: u32,
        security_info: u32,
        owner: Psid,
        group: Psid,
        dacl: Pacl,
        sacl: Pacl,
    ) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
}

struct AppContainerSid(Psid);

impl AppContainerSid {
    fn derive(profile_name: &str) -> Result<Self, String> {
        let name = wide_str(profile_name);
        let mut sid = ptr::null_mut();
        let result = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut sid) };
        if result != 0 {
            return Err(format!(
                "DeriveAppContainerSidFromAppContainerName failed with HRESULT 0x{:08X}",
                result as u32
            ));
        }
        if sid.is_null() {
            return Err("DeriveAppContainerSidFromAppContainerName returned a null SID".into());
        }
        Ok(Self(sid))
    }

    fn raw(&self) -> Psid {
        self.0
    }
}

impl Drop for AppContainerSid {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                FreeSid(self.0);
            }
        }
    }
}

struct WorkspaceAclGrant {
    path: Vec<u16>,
    original_descriptor: PsecurityDescriptor,
    original_dacl: Pacl,
    granted_dacl: Pacl,
    restored: bool,
}

impl WorkspaceAclGrant {
    fn apply(path: &Path, sid: Psid) -> Result<Self, String> {
        let mut path_wide = wide_path(path);
        let mut original_descriptor = ptr::null_mut();
        let mut original_dacl = ptr::null_mut();
        let get_result = unsafe {
            GetNamedSecurityInfoW(
                path_wide.as_mut_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut original_dacl,
                ptr::null_mut(),
                &mut original_descriptor,
            )
        };
        if get_result != 0 {
            return Err(format!(
                "GetNamedSecurityInfoW failed with Win32 error {get_result}"
            ));
        }

        let trustee = TrusteeW {
            multiple_trustee: ptr::null_mut(),
            multiple_trustee_operation: NO_MULTIPLE_TRUSTEE,
            trustee_form: TRUSTEE_IS_SID,
            trustee_type: TRUSTEE_IS_UNKNOWN,
            name: sid.cast::<u16>(),
        };
        let access = ExplicitAccessW {
            access_permissions: FILE_GENERIC_READ_ACCESS
                | FILE_GENERIC_WRITE_ACCESS
                | FILE_GENERIC_EXECUTE_ACCESS,
            access_mode: GRANT_ACCESS,
            inheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            trustee,
        };
        let mut granted_dacl = ptr::null_mut();
        let acl_result = unsafe { SetEntriesInAclW(1, &access, original_dacl, &mut granted_dacl) };
        if acl_result != 0 {
            if !original_descriptor.is_null() {
                unsafe { LocalFree(original_descriptor) };
            }
            return Err(format!(
                "SetEntriesInAclW failed with Win32 error {acl_result}"
            ));
        }

        let set_result = unsafe {
            SetNamedSecurityInfoW(
                path_wide.as_mut_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                granted_dacl,
                ptr::null_mut(),
            )
        };
        if set_result != 0 {
            unsafe {
                LocalFree(granted_dacl);
                LocalFree(original_descriptor);
            }
            return Err(format!(
                "SetNamedSecurityInfoW grant failed with Win32 error {set_result}"
            ));
        }

        Ok(Self {
            path: path_wide,
            original_descriptor,
            original_dacl,
            granted_dacl,
            restored: false,
        })
    }

    fn restore(&mut self) -> Result<(), String> {
        if self.restored {
            return Ok(());
        }
        let result = unsafe {
            SetNamedSecurityInfoW(
                self.path.as_mut_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                self.original_dacl,
                ptr::null_mut(),
            )
        };
        if result != 0 {
            return Err(format!(
                "SetNamedSecurityInfoW restore failed with Win32 error {result}"
            ));
        }
        self.restored = true;
        self.free_allocations();
        Ok(())
    }

    fn free_allocations(&mut self) {
        if !self.granted_dacl.is_null() {
            unsafe { LocalFree(self.granted_dacl) };
            self.granted_dacl = ptr::null_mut();
        }
        if !self.original_descriptor.is_null() {
            unsafe { LocalFree(self.original_descriptor) };
            self.original_descriptor = ptr::null_mut();
            self.original_dacl = ptr::null_mut();
        }
    }
}

impl Drop for WorkspaceAclGrant {
    fn drop(&mut self) {
        if !self.restored && !self.original_descriptor.is_null() {
            let _ = unsafe {
                SetNamedSecurityInfoW(
                    self.path.as_mut_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    self.original_dacl,
                    ptr::null_mut(),
                )
            };
        }
        self.free_allocations();
    }
}

fn wide_str(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn required_env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("required Windows environment variable is unavailable: {name}"))
}

fn qualification_env(workspace: &Path) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([
        ("LOCALAPPDATA".to_owned(), required_env("LOCALAPPDATA")),
        ("SystemRoot".to_owned(), required_env("SystemRoot")),
        ("TEMP".to_owned(), workspace.to_string_lossy().into_owned()),
        ("TMP".to_owned(), workspace.to_string_lossy().into_owned()),
    ]);
    for optional in ["PATH", "ComSpec"] {
        if let Ok(value) = std::env::var(optional) {
            env.insert(optional.to_owned(), value);
        }
    }
    env.insert("QUNTAL_TUNNEL_KEY_FILE".into(), "must-not-leak".into());
    env.insert("OPENAI_API_KEY".into(), "must-not-leak".into());
    env
}

fn paths() -> (PathBuf, PathBuf, u128) {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir();
    let workspace = root.join(format!(
        "Quntal.SG000014.Workspace.{}.{}",
        std::process::id(),
        suffix
    ));
    let sibling = root.join(format!(
        "Quntal.SG000014.Sibling.{}.{}",
        std::process::id(),
        suffix
    ));
    (workspace, sibling, suffix)
}

fn sibling_from_workspace(workspace: &Path) -> PathBuf {
    let name = workspace
        .file_name()
        .expect("workspace has final component")
        .to_string_lossy();
    let sibling_name = name.replacen(".Workspace.", ".Sibling.", 1);
    workspace
        .parent()
        .expect("workspace has parent")
        .join(sibling_name)
}

#[test]
fn workspace_authority_child() {
    if !std::env::args().any(|argument| argument == "--exact") {
        return;
    }

    assert!(std::env::var_os("QUNTAL_TUNNEL_KEY_FILE").is_none());
    assert!(std::env::var_os("OPENAI_API_KEY").is_none());
    assert!(std::env::var_os("QUNTAL_DAEMON").is_none());

    use std::io::Read;
    let mut byte = [0u8; 1];
    assert_eq!(std::io::stdin().read(&mut byte).expect("read stdin"), 0);

    let workspace = std::env::current_dir().expect("read qualified workspace cwd");
    let input = std::fs::read(workspace.join("input.txt")).expect("read workspace fixture");
    assert_eq!(input, b"QUNTAL-SG-000014-READ");
    std::fs::write(workspace.join("output.txt"), b"QUNTAL-SG-000014-WRITE")
        .expect("write workspace fixture");

    let sibling = sibling_from_workspace(&workspace).join("outside.txt");
    assert!(
        std::fs::read(&sibling).is_err(),
        "workspace grant leaked to sibling resource"
    );
}

#[test]
fn windows_appcontainer_workspace_authority_is_scoped_and_reversible() {
    let (workspace, sibling, suffix) = paths();
    std::fs::create_dir_all(&workspace).expect("create qualification workspace");
    std::fs::create_dir_all(&sibling).expect("create sibling resource");
    std::fs::write(workspace.join("input.txt"), b"QUNTAL-SG-000014-READ")
        .expect("seed workspace fixture");
    std::fs::write(sibling.join("outside.txt"), b"QUNTAL-SG-000014-DENY")
        .expect("seed sibling fixture");

    let profile = format!("Quntal.SG000014.{}.{}", std::process::id(), suffix);
    let sid = AppContainerSid::derive(&profile).expect("derive deterministic AppContainer SID");
    let mut grant = WorkspaceAclGrant::apply(&workspace, sid.raw())
        .expect("apply workspace-scoped AppContainer ACL");

    let executable = std::env::current_exe().expect("integration-test executable");
    let plan = build_execution_plan(
        &workspace,
        &executable,
        &[
            "--exact".to_owned(),
            "workspace_authority_child".to_owned(),
            "--nocapture".to_owned(),
        ],
        ".",
        &qualification_env(&workspace),
        ExecutionLimits::default(),
    )
    .expect("workspace-authority execution plan");

    assert!(!plan.env.contains_key("QUNTAL_TUNNEL_KEY_FILE"));
    assert!(!plan.env.contains_key("OPENAI_API_KEY"));
    assert!(!plan.env.contains_key("QUNTAL_DAEMON"));

    let result = execute_contained(plan, &profile).expect("contained workspace qualification");
    assert_eq!(
        result.exit_code,
        0,
        "contained child failed; stdout={} stderr={}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.appcontainer_verified);
    assert!(result.assigned_to_job_before_resume);
    assert!(result.job_quiescent);
    assert_eq!(
        std::fs::read(workspace.join("output.txt")).expect("host verifies workspace output"),
        b"QUNTAL-SG-000014-WRITE"
    );
    assert_eq!(
        std::fs::read(sibling.join("outside.txt")).expect("host retains sibling access"),
        b"QUNTAL-SG-000014-DENY"
    );

    grant.restore().expect("restore original workspace DACL");
    std::fs::remove_dir_all(&sibling).expect("remove sibling fixture");
    std::fs::remove_dir_all(&workspace).expect("remove workspace fixture");
}
