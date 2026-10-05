//! SG-000074 `qdral-browser-host` binary.
//!
//! Launched only by Deskal supervision with `--engine`, `--profile-dir`,
//! and `--host-id`. The binary re-verifies the engine and profile, spawns
//! the engine supervised with the frozen command line, and serves the
//! closed hello/ping/shutdown vocabulary over piped stdio. End of input,
//! shutdown, or any protocol violation stops the engine and exits. There
//! is no listener, no tool, and no other input surface.

use qdral_browser_host::{
    adopt_profile, assert_argv_clean, build_argv, decode_frame, discover_engine, Engine, HostReply,
    HostRequest, SearchConfig, SupervisedChild, PROTOCOL_GENERATION,
};
use std::io::{BufReader, Write};
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("qdral-browser-host: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut engine: Option<PathBuf> = None;
    let mut profile_dir: Option<PathBuf> = None;
    let mut host_id: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {arg}"))?;
        match arg.as_str() {
            "--engine" => engine = Some(PathBuf::from(value)),
            "--profile-dir" => profile_dir = Some(PathBuf::from(value)),
            "--host-id" => host_id = Some(value),
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let engine_path = engine.ok_or_else(|| "--engine is required".to_string())?;
    let profile_dir = profile_dir.ok_or_else(|| "--profile-dir is required".to_string())?;
    let host_id = host_id.ok_or_else(|| "--host-id is required".to_string())?;
    if host_id.is_empty() || host_id.len() > 64 {
        return Err("invalid --host-id".to_string());
    }
    // Re-verify: the engine must still be a supported verified executable
    // and the profile must still be Deskal-owned for it.
    let verified: Engine = discover_engine(&SearchConfig::production())
        .map_err(|err| format!("engine verification failed: {err}"))?;
    if verified.path() != engine_path {
        return Err("engine path is not the verified engine".to_string());
    }
    let profile = adopt_profile(&profile_dir, &verified.fingerprint())
        .map_err(|err| format!("profile verification failed: {err}"))?;
    let argv = build_argv(profile.dir()).map_err(|err| format!("argv refused: {err}"))?;
    assert_argv_clean(&argv).map_err(|err| format!("argv refused: {err}"))?;
    let env = scrubbed_env();
    let engine_child = qdral_browser_host::spawn_engine_null(verified.path(), &argv, &env)
        .map_err(|err| format!("engine launch failed: {err}"))?;
    serve(host_id, engine_child)
}

/// Scrubbed environment for the engine: OS locations plus profile-scoped
/// temp only. No caller, user-profile, or Deskal secret variable is
/// inherited: USERPROFILE, APPDATA, and LOCALAPPDATA are deliberately
/// absent so the engine can never resolve the personal profile.
fn scrubbed_env() -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    #[cfg(windows)]
    {
        let system_root = std::env::var_os("SystemRoot")
            .unwrap_or_else(|| std::ffi::OsString::from("C:\\Windows"));
        let system32 = PathBuf::from(&system_root).join("System32");
        // TEMP/TMP stay inside Deskal state in the supervised launch path;
        // the host binary itself has no profile yet, so system temp applies.
        let system_temp = PathBuf::from(&system_root).join("Temp");
        vec![
            (std::ffi::OsString::from("SystemRoot"), system_root.clone()),
            (
                std::ffi::OsString::from("SystemDrive"),
                std::env::var_os("SystemDrive").unwrap_or_else(|| std::ffi::OsString::from("C:")),
            ),
            (
                std::ffi::OsString::from("PATH"),
                std::ffi::OsString::from(system32.to_string_lossy().into_owned()),
            ),
            (
                std::ffi::OsString::from("TEMP"),
                system_temp.clone().into_os_string(),
            ),
            (
                std::ffi::OsString::from("TMP"),
                system_temp.into_os_string(),
            ),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![
            (
                std::ffi::OsString::from("PATH"),
                std::ffi::OsString::from("/usr/bin:/bin"),
            ),
            (
                std::ffi::OsString::from("LANG"),
                std::ffi::OsString::from("C"),
            ),
        ]
    }
}

fn serve(host_id: String, engine_child: SupervisedChild) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut input = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    loop {
        let request: HostRequest =
            decode_frame(&mut input).map_err(|_| "protocol violation".to_string())?;
        match request {
            HostRequest::Hello { generation } => {
                if generation != PROTOCOL_GENERATION {
                    return Err("protocol generation mismatch".to_string());
                }
                write_reply(
                    &mut output,
                    &HostReply::Hello {
                        generation: PROTOCOL_GENERATION,
                        host: host_id.clone(),
                    },
                )?;
            }
            HostRequest::Ping { nonce } => {
                if nonce.is_empty() || nonce.len() > 256 {
                    write_reply(
                        &mut output,
                        &HostReply::Error {
                            code: "bad_nonce".into(),
                        },
                    )?;
                    continue;
                }
                write_reply(&mut output, &HostReply::Pong { nonce })?;
            }
            HostRequest::Shutdown => {
                let _ = write_reply(&mut output, &HostReply::Bye);
                let _ = engine_child.shutdown();
                return Ok(());
            }
        }
    }
    // `decode_frame` enforces the closed vocabulary with
    // `deny_unknown_fields`: anything reaching the match above is known.
    // Any I/O or framing failure returns early; the supervised engine dies
    // with this process through job and drop cleanup.
}

fn write_reply(output: &mut impl Write, reply: &HostReply) -> Result<(), String> {
    qdral_browser_host::write_frame(output, reply).map_err(|_| "reply failed".to_string())?;
    output.flush().map_err(|_| "reply flush failed".to_string())
}
