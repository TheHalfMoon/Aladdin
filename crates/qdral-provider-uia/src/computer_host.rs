// Copyright 2026 TheHalfMoon contributors
// SPDX-License-Identifier: Apache-2.0
//
// SG-000094 private Windows Computer Host adapter. The adapter is deliberately
// not the default NativeAdapter and adds no caller-facing MCP surface. It only
// translates host OS facts/pixels into the existing UiaAdapter contract.

use crate::{CapturedImage, NativeElement, NativeProcess, NativeWindow, UiaAdapter, UiaError};
use qdral_contracts::FailureCode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const HOST_PROTOCOL: &str = "deskal-computer-host/1";
const HOST_IDENTITY: &str = "deskal-windows-computer-host";
const MAX_HOST_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Serialize)]
struct HostRequest<'a> {
    id: u64,
    protocol: &'a str,
    op: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    hwnd: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_nodes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_depth: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct HostError {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize, Clone)]
struct HostProcess {
    pid: u32,
    exe_name: String,
    exe_id: String,
    session_id: u32,
    session_verified: bool,
    start_generation: u64,
    generation_source: String,
}

#[derive(Debug, Deserialize, Clone)]
struct HostWindow {
    process: HostProcess,
    hwnd: u64,
    title: String,
    class: String,
    visible: bool,
    window_nonce: u64,
}

#[derive(Debug, Deserialize)]
struct HostElement {
    runtime_id: String,
    control_type: String,
    automation_id: String,
    name: String,
    enabled: bool,
    selected: bool,
    toggled: bool,
    scroll_horizontal_percent: u8,
    scroll_vertical_percent: u8,
    patterns: Vec<String>,
    value: Option<String>,
    value_is_password: bool,
    children: Vec<HostElement>,
}

#[derive(Debug, Deserialize)]
struct HostCapture {
    width: u32,
    height: u32,
    pixel_format: String,
    pixels_b64: String,
}

#[derive(Debug, Deserialize)]
struct HostResponse {
    id: u64,
    ok: bool,
    protocol: String,
    host: String,
    #[serde(default)]
    pong: bool,
    #[serde(default)]
    windows: Vec<HostWindow>,
    #[serde(default)]
    elements: Vec<HostElement>,
    capture: Option<HostCapture>,
    error: Option<HostError>,
}

fn map_host_error(error: HostError) -> UiaError {
    let code = match error.code.as_str() {
        "invalid_request" => FailureCode::InvalidRequest,
        "capability_denied" => FailureCode::CapabilityDenied,
        "target_stale" => FailureCode::TargetStale,
        "output_limit" => FailureCode::OutputLimit,
        "provider_unavailable" => FailureCode::ProviderUnavailable,
        _ => FailureCode::ProviderUnavailable,
    };
    UiaError::new(code, error.message)
}

fn decode_base64(input: &str, max_bytes: usize) -> Result<Vec<u8>, UiaError> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let bytes = input.as_bytes();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() % 4 != 0 {
        return Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "computer host returned malformed base64 pixels",
        ));
    }
    let estimated = bytes.len() / 4 * 3;
    if estimated > max_bytes.saturating_add(2) {
        return Err(UiaError::new(
            FailureCode::OutputLimit,
            "computer host pixel payload exceeds the bounded size",
        ));
    }

    let mut out = Vec::with_capacity(estimated.min(max_bytes));
    for (chunk_index, chunk) in bytes.chunks_exact(4).enumerate() {
        let last = chunk_index + 1 == bytes.len() / 4;
        let pad2 = chunk[2] == b'=';
        let pad3 = chunk[3] == b'=';
        if pad2 && !pad3 {
            return Err(UiaError::new(
                FailureCode::ProviderUnavailable,
                "computer host returned invalid base64 padding",
            ));
        }
        if (pad2 || pad3) && !last {
            return Err(UiaError::new(
                FailureCode::ProviderUnavailable,
                "computer host returned non-final base64 padding",
            ));
        }
        let a = value(chunk[0]);
        let b = value(chunk[1]);
        let c = if pad2 { Some(0) } else { value(chunk[2]) };
        let d = if pad3 { Some(0) } else { value(chunk[3]) };
        let (Some(a), Some(b), Some(c), Some(d)) = (a, b, c, d) else {
            return Err(UiaError::new(
                FailureCode::ProviderUnavailable,
                "computer host returned non-base64 pixel bytes",
            ));
        };
        out.push((a << 2) | (b >> 4));
        if !pad2 {
            out.push((b << 4) | (c >> 2));
        }
        if !pad3 {
            out.push((c << 6) | d);
        }
        if out.len() > max_bytes {
            return Err(UiaError::new(
                FailureCode::OutputLimit,
                "computer host pixel payload exceeds the bounded size",
            ));
        }
    }
    Ok(out)
}

fn convert_element(element: HostElement) -> NativeElement {
    NativeElement {
        runtime_id: element.runtime_id,
        control_type: element.control_type,
        automation_id: element.automation_id,
        name: element.name,
        enabled: element.enabled,
        selected: element.selected,
        toggled: element.toggled,
        scroll_horizontal_percent: element.scroll_horizontal_percent.min(100),
        scroll_vertical_percent: element.scroll_vertical_percent.min(100),
        patterns: element.patterns.into_iter().take(16).collect(),
        value: if element.value_is_password {
            None
        } else {
            element.value
        },
        value_is_password: element.value_is_password,
        children: element.children.into_iter().map(convert_element).collect(),
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
    use std::sync::Mutex;

    struct HostSession {
        child: Child,
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
        next_id: u64,
    }

    impl HostSession {
        fn spawn(path: &Path) -> Result<Self, UiaError> {
            let mut command = Command::new(path);
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .env_clear();
            for name in ["SystemRoot", "WINDIR", "COMSPEC", "TEMP", "TMP", "PATH"] {
                if let Some(value) = std::env::var_os(name) {
                    command.env(name, value);
                }
            }
            let mut child = command.spawn().map_err(|error| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    format!("Windows Computer Host could not start: {error}"),
                )
            })?;
            let stdin = child.stdin.take().ok_or_else(|| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host stdin is unavailable",
                )
            })?;
            let stdout = child.stdout.take().ok_or_else(|| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host stdout is unavailable",
                )
            })?;
            let mut session = Self {
                child,
                stdin,
                stdout: BufReader::new(stdout),
                next_id: 1,
            };
            let hello = session.request("hello", None, None, None)?;
            if !hello.ok || hello.protocol != HOST_PROTOCOL || hello.host != HOST_IDENTITY {
                return Err(UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host handshake identity mismatch",
                ));
            }
            Ok(session)
        }

        fn request(
            &mut self,
            op: &str,
            hwnd: Option<u64>,
            max_nodes: Option<usize>,
            max_depth: Option<usize>,
        ) -> Result<HostResponse, UiaError> {
            let id = self.next_id;
            self.next_id = self.next_id.checked_add(1).ok_or_else(|| {
                UiaError::new(
                    FailureCode::InternalError,
                    "Windows Computer Host request id overflow",
                )
            })?;
            let request = HostRequest {
                id,
                protocol: HOST_PROTOCOL,
                op,
                hwnd,
                max_nodes,
                max_depth,
            };
            let raw = serde_json::to_vec(&request).map_err(|_| {
                UiaError::new(
                    FailureCode::InternalError,
                    "Windows Computer Host request serialization failed",
                )
            })?;
            self.stdin.write_all(&raw).map_err(|_| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host request pipe is unavailable",
                )
            })?;
            self.stdin.write_all(b"\n").map_err(|_| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host request framing failed",
                )
            })?;
            self.stdin.flush().map_err(|_| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host request flush failed",
                )
            })?;

            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).map_err(|_| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host response pipe is unavailable",
                )
            })?;
            if read == 0 {
                return Err(UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host exited before returning a response",
                ));
            }
            if line.len() > MAX_HOST_RESPONSE_BYTES {
                return Err(UiaError::new(
                    FailureCode::OutputLimit,
                    "Windows Computer Host response exceeds the bounded size",
                ));
            }
            let response: HostResponse = serde_json::from_str(&line).map_err(|_| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host returned corrupt JSON",
                )
            })?;
            if response.id != id
                || response.protocol != HOST_PROTOCOL
                || response.host != HOST_IDENTITY
            {
                return Err(UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host response binding mismatch",
                ));
            }
            if !response.ok {
                return Err(response.error.map(map_host_error).unwrap_or_else(|| {
                    UiaError::new(
                        FailureCode::ProviderUnavailable,
                        "Windows Computer Host failed without typed error",
                    )
                }));
            }
            Ok(response)
        }
    }

    impl Drop for HostSession {
        fn drop(&mut self) {
            let _ = self.request("shutdown", None, None, None);
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    pub struct ComputerHostAdapter {
        path: PathBuf,
        session: Mutex<HostSession>,
    }

    impl ComputerHostAdapter {
        pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, UiaError> {
            let path = path.into();
            let session = HostSession::spawn(&path)?;
            Ok(Self {
                path,
                session: Mutex::new(session),
            })
        }

        pub fn from_env() -> Result<Self, UiaError> {
            let path = std::env::var_os("DESKAL_COMPUTER_HOST_BIN").ok_or_else(|| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "DESKAL_COMPUTER_HOST_BIN is not configured",
                )
            })?;
            Self::from_path(PathBuf::from(path))
        }

        pub fn executable(&self) -> &Path {
            &self.path
        }

        fn request(
            &self,
            op: &str,
            hwnd: Option<u64>,
            max_nodes: Option<usize>,
            max_depth: Option<usize>,
        ) -> Result<HostResponse, UiaError> {
            let mut session = self.session.lock().map_err(|_| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host session lock is poisoned",
                )
            })?;
            session.request(op, hwnd, max_nodes, max_depth)
        }
    }

    impl UiaAdapter for ComputerHostAdapter {
        fn list_processes(&self) -> Result<Vec<NativeProcess>, UiaError> {
            let response = self.request("list_windows", None, None, None)?;
            let mut processes = BTreeMap::<u32, HostProcess>::new();
            for window in response.windows {
                processes
                    .entry(window.process.pid)
                    .or_insert(window.process);
            }
            Ok(processes
                .into_values()
                .map(|process| NativeProcess {
                    pid: process.pid,
                    exe_name: process.exe_name,
                    exe_id: process.exe_id,
                    session_id: process.session_id,
                    session_verified: process.session_verified,
                    start_generation: process.start_generation,
                    generation_source: if process.generation_source == "win32-creation-time" {
                        "win32-creation-time"
                    } else {
                        "computer-host"
                    },
                })
                .collect())
        }

        fn list_windows(&self, pid: u32) -> Result<Vec<NativeWindow>, UiaError> {
            let response = self.request("list_windows", None, None, None)?;
            Ok(response
                .windows
                .into_iter()
                .filter(|window| window.process.pid == pid)
                .map(|window| NativeWindow {
                    hwnd: window.hwnd,
                    title: window.title,
                    class: window.class,
                    visible: window.visible,
                    window_nonce: window.window_nonce,
                })
                .collect())
        }

        fn read_tree(&self, hwnd: u64) -> Result<Vec<NativeElement>, UiaError> {
            let response = self.request(
                "observe_window",
                Some(hwnd),
                Some(crate::MAX_TREE_NODES),
                Some(crate::MAX_TREE_DEPTH),
            )?;
            Ok(response.elements.into_iter().map(convert_element).collect())
        }

        fn capture_window(&self, hwnd: u64) -> Result<CapturedImage, UiaError> {
            let response = self.request("capture_window", Some(hwnd), None, None)?;
            let capture = response.capture.ok_or_else(|| {
                UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host omitted capture payload",
                )
            })?;
            if capture.pixel_format != "rgba8" {
                return Err(UiaError::new(
                    FailureCode::ProviderUnavailable,
                    "Windows Computer Host returned an unsupported pixel format",
                ));
            }
            let bytes = decode_base64(&capture.pixels_b64, crate::MAX_CAPTURE_BYTES)?;
            Ok(CapturedImage {
                width: capture.width,
                height: capture.height,
                bytes,
            })
        }
    }
}

#[cfg(windows)]
pub use platform::ComputerHostAdapter;

#[cfg(not(windows))]
pub struct ComputerHostAdapter {
    path: PathBuf,
}

#[cfg(not(windows))]
impl ComputerHostAdapter {
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, UiaError> {
        Ok(Self { path: path.into() })
    }

    pub fn from_env() -> Result<Self, UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "Windows Computer Host is available only on Windows",
        ))
    }

    pub fn executable(&self) -> &Path {
        &self.path
    }
}

#[cfg(not(windows))]
impl UiaAdapter for ComputerHostAdapter {
    fn list_processes(&self) -> Result<Vec<NativeProcess>, UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "Windows Computer Host is available only on Windows",
        ))
    }

    fn list_windows(&self, _pid: u32) -> Result<Vec<NativeWindow>, UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "Windows Computer Host is available only on Windows",
        ))
    }

    fn read_tree(&self, _hwnd: u64) -> Result<Vec<NativeElement>, UiaError> {
        Err(UiaError::new(
            FailureCode::ProviderUnavailable,
            "Windows Computer Host is available only on Windows",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decoder_is_bounded_and_strict() {
        assert_eq!(decode_base64("AQIDBA==", 4).unwrap(), vec![1, 2, 3, 4]);
        assert!(decode_base64("AQIDBA==", 3).is_err());
        assert!(decode_base64("AQI*", 4).is_err());
        assert!(decode_base64("A===", 4).is_err());
    }

    #[test]
    fn password_values_are_never_projected() {
        let element = HostElement {
            runtime_id: "1.2".into(),
            control_type: "Edit".into(),
            automation_id: "password".into(),
            name: "Password".into(),
            enabled: true,
            selected: false,
            toggled: false,
            scroll_horizontal_percent: 0,
            scroll_vertical_percent: 0,
            patterns: vec!["Value".into()],
            value: Some("secret".into()),
            value_is_password: true,
            children: Vec::new(),
        };
        assert_eq!(convert_element(element).value, None);
    }

    #[cfg(windows)]
    #[test]
    fn configured_host_handshake_is_exact() {
        let Some(path) = std::env::var_os("DESKAL_COMPUTER_HOST_BIN") else {
            return;
        };
        let adapter = ComputerHostAdapter::from_path(PathBuf::from(path)).unwrap();
        assert!(adapter.executable().ends_with("deskal-computer-host.exe"));
        let mut session = adapter.session.lock().unwrap();
        let pong = session.request("ping", None, None, None).unwrap();
        assert!(pong.pong);
    }
}
