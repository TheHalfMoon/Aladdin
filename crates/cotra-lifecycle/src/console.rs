//! Reading secret input from an interactive console without echo.

use crate::LifecycleError;

/// Prompts on stderr and reads one line from the console with echo disabled.
/// Fails when stdin is not an interactive console or echo cannot be
/// disabled, so a secret is never read while it could be echoed or captured
/// from a pipe; use `--key-file` instead. The original console mode is
/// restored on return, on error, and on Ctrl+C or Ctrl+Break.
#[cfg(windows)]
pub fn read_secret_line(prompt: &str) -> Result<String, LifecycleError> {
    use std::sync::atomic::{AtomicU32, Ordering};
    use windows_sys::Win32::Foundation::BOOL;
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleCtrlHandler, SetConsoleMode, ENABLE_ECHO_INPUT,
        STD_INPUT_HANDLE,
    };

    const NO_MODE: u32 = u32::MAX;
    static SAVED_MODE: AtomicU32 = AtomicU32::new(NO_MODE);

    unsafe extern "system" fn restore_on_signal(_signal: u32) -> BOOL {
        let mode = SAVED_MODE.load(Ordering::SeqCst);
        if mode != NO_MODE {
            // SAFETY: restores the console input mode saved before reading.
            unsafe { SetConsoleMode(GetStdHandle(STD_INPUT_HANDLE), mode) };
        }
        // Not handled: the default handler still terminates the process.
        0
    }

    struct Restore {
        handle: isize,
        mode: u32,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            // SAFETY: restores the saved mode and removes the signal handler.
            unsafe {
                SetConsoleMode(self.handle, self.mode);
                SetConsoleCtrlHandler(Some(restore_on_signal), 0);
            }
            SAVED_MODE.store(NO_MODE, Ordering::SeqCst);
        }
    }

    // SAFETY: GetStdHandle has no preconditions.
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut mode = 0u32;
    // SAFETY: valid out-pointer; failure means stdin is not a console.
    if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
        return Err(LifecycleError::usage(
            "no interactive console for the hidden key prompt; pass --key-file <path>",
        ));
    }
    SAVED_MODE.store(mode, Ordering::SeqCst);
    // SAFETY: registers a handler with the documented signature.
    if unsafe { SetConsoleCtrlHandler(Some(restore_on_signal), 1) } == 0 {
        SAVED_MODE.store(NO_MODE, Ordering::SeqCst);
        return Err(LifecycleError::platform(
            "the console signal handler could not be installed; pass --key-file <path>",
        ));
    }
    let restore = Restore { handle, mode };
    // SAFETY: the handle is the console input handle queried above.
    if unsafe { SetConsoleMode(handle, mode & !ENABLE_ECHO_INPUT) } == 0 {
        return Err(LifecycleError::platform(
            "console echo could not be disabled, so the key was not read; pass --key-file <path>",
        ));
    }
    eprint!("{prompt}");
    let mut line = String::new();
    let read = std::io::stdin().read_line(&mut line);
    drop(restore);
    eprintln!();
    read.map_err(|error| LifecycleError::io("read key", error))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(not(windows))]
pub fn read_secret_line(_prompt: &str) -> Result<String, LifecycleError> {
    Err(LifecycleError::usage(
        "the hidden key prompt is available on Windows only; pass --key-file <path>",
    ))
}
