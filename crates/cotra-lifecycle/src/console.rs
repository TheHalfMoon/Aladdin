//! Reading secret input from an interactive console without echo.

use crate::LifecycleError;

/// Prompts on stderr and reads one line from the console with echo disabled.
/// Fails when stdin is not an interactive console or echo cannot be
/// disabled, so a secret is never read while it could be echoed or captured
/// from a pipe; use `--key-file` instead. The original console mode is
/// restored on return, on error, and on Ctrl+C or Ctrl+Break. Prompts are
/// serialized: the saved (handle, mode) pair is process-global, so only one
/// hidden prompt can be active at a time.
#[cfg(windows)]
pub fn read_secret_line(prompt: &str) -> Result<String, LifecycleError> {
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::BOOL;
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleCtrlHandler, SetConsoleMode, ENABLE_ECHO_INPUT,
        STD_INPUT_HANDLE,
    };

    /// The console input handle and mode this prompt changed, restored by the
    /// signal handler and by `Restore`.
    static SAVED: Mutex<Option<(isize, u32)>> = Mutex::new(None);
    /// Serializes prompts so `SAVED` always describes the active prompt.
    static PROMPT: Mutex<()> = Mutex::new(());

    unsafe extern "system" fn restore_on_signal(_signal: u32) -> BOOL {
        if let Ok(saved) = SAVED.try_lock() {
            if let Some((handle, mode)) = *saved {
                // SAFETY: restores the exact handle and mode saved by this prompt.
                unsafe { SetConsoleMode(handle, mode) };
            }
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
            if let Ok(mut saved) = SAVED.lock() {
                *saved = None;
            }
        }
    }

    let _prompt = PROMPT
        .lock()
        .map_err(|_| LifecycleError::internal("hidden prompt lock poisoned"))?;
    // SAFETY: GetStdHandle has no preconditions.
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut mode = 0u32;
    // SAFETY: valid out-pointer; failure means stdin is not a console.
    if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
        return Err(LifecycleError::usage(
            "no interactive console for the hidden key prompt; pass --key-file <path>",
        ));
    }
    *SAVED
        .lock()
        .map_err(|_| LifecycleError::internal("console state lock poisoned"))? =
        Some((handle, mode));
    // SAFETY: registers a handler with the documented signature.
    if unsafe { SetConsoleCtrlHandler(Some(restore_on_signal), 1) } == 0 {
        if let Ok(mut saved) = SAVED.lock() {
            *saved = None;
        }
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
