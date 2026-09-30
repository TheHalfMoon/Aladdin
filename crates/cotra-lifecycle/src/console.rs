//! Reading secret input from an interactive console without echo.

use crate::LifecycleError;

/// Prompts on stderr and reads one line from the console with echo disabled.
/// Fails when stdin is not an interactive console, so secrets are never read
/// from a pipe that might be logged; use `--key-file` instead.
#[cfg(windows)]
pub fn read_secret_line(prompt: &str) -> Result<String, LifecycleError> {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_ECHO_INPUT, STD_INPUT_HANDLE,
    };
    // SAFETY: GetStdHandle has no preconditions.
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut mode = 0u32;
    // SAFETY: valid out-pointer; failure means stdin is not a console.
    if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
        return Err(LifecycleError::usage(
            "no interactive console for the hidden key prompt; pass --key-file <path>",
        ));
    }
    eprint!("{prompt}");
    // SAFETY: the handle is the console input handle queried above.
    unsafe { SetConsoleMode(handle, mode & !ENABLE_ECHO_INPUT) };
    let mut line = String::new();
    let read = std::io::stdin().read_line(&mut line);
    // SAFETY: restores the mode read above.
    unsafe { SetConsoleMode(handle, mode) };
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
