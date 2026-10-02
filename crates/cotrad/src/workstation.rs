//! SG-000055 interactive workstation state for the remote-session lease.
//!
//! The lease binds the Windows session and its logon time. Dispatch is
//! allowed only while that exact session is active and unlocked. A locked,
//! disconnected, re-logged-on, or unknown session fails closed. Other
//! platforms report `Unknown`, so remote dispatch never runs there.

use cotra_policy::remote_session::WorkstationState;

#[cfg(windows)]
pub fn current() -> WorkstationState {
    use cotra_policy::remote_session::WorkstationBinding;
    use windows_sys::Win32::System::RemoteDesktop::{
        ProcessIdToSessionId, WTSActive, WTSFreeMemory, WTSQuerySessionInformationW,
        WTSSessionInfoEx, WTSINFOEXW, WTS_CURRENT_SERVER_HANDLE, WTS_SESSIONSTATE_UNLOCK,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;

    let mut session_id = 0u32;
    if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id) } == 0 {
        return WorkstationState::Unknown;
    }
    let mut buffer: *mut u16 = std::ptr::null_mut();
    let mut bytes = 0u32;
    let ok = unsafe {
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            WTSSessionInfoEx,
            &mut buffer,
            &mut bytes,
        )
    };
    if ok == 0 || buffer.is_null() {
        return WorkstationState::Unknown;
    }
    let state = if (bytes as usize) < std::mem::size_of::<WTSINFOEXW>() {
        WorkstationState::Unknown
    } else {
        let info = unsafe { &*(buffer as *const WTSINFOEXW) };
        if info.Level != 1 {
            WorkstationState::Unknown
        } else {
            let level1 = unsafe { info.Data.WTSInfoExLevel1 };
            if level1.SessionId != session_id || level1.SessionState != WTSActive {
                WorkstationState::Locked
            } else if level1.SessionFlags == WTS_SESSIONSTATE_UNLOCK as i32 {
                WorkstationState::Unlocked(WorkstationBinding {
                    session_id,
                    logon_id: level1.LogonTime,
                })
            } else {
                // WTS_SESSIONSTATE_LOCK and WTS_SESSIONSTATE_UNKNOWN both fail closed.
                WorkstationState::Locked
            }
        }
    };
    unsafe { WTSFreeMemory(buffer.cast()) };
    state
}

#[cfg(not(windows))]
pub fn current() -> WorkstationState {
    WorkstationState::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workstation_probe_never_panics_and_reports_a_state() {
        match current() {
            WorkstationState::Unlocked(binding) => {
                assert!(binding.logon_id != 0 || binding.session_id != u32::MAX)
            }
            WorkstationState::Locked | WorkstationState::Unknown => {}
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_hosts_never_permit_remote_dispatch() {
        assert_eq!(current(), WorkstationState::Unknown);
    }
}
