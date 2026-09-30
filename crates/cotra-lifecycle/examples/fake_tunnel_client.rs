//! Test fixture standing in for the official OpenAI tunnel client in
//! lifecycle qualification. It accepts the same arguments as the closed
//! launch plan, reads the runtime key through its `file:` reference,
//! publishes a loopback health URL, runs the MCP command over stdio, and
//! reports what it observed on stdout (which the supervisor logs). It proves
//! Cotra's supervision and launch path only; it does not prove the OpenAI
//! service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};

fn value(args: &[String], name: &str) -> String {
    let index = args
        .iter()
        .position(|arg| arg == name)
        .unwrap_or_else(|| panic!("missing {name}"));
    args[index + 1].clone()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(args.first().map(String::as_str), Some("run"));
    let key_ref = value(&args, "--control-plane.api-key");
    let key_path = key_ref.strip_prefix("file:").expect("file: reference");
    let key = std::fs::read_to_string(key_path).expect("read runtime key");
    println!("fake-tunnel: key-file-read={}", !key.trim().is_empty());
    // Deliberately emit the key so the supervisor's redaction is exercised.
    println!("fake-tunnel: debug api_key={}", key.trim());
    let leaked = std::env::vars()
        .filter(|(name, value)| {
            let upper = name.to_ascii_uppercase();
            ["KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL"]
                .iter()
                .any(|needle| upper.contains(needle))
                || value.contains(key.trim())
        })
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    println!("fake-tunnel: env-clean={}", leaked.is_empty());

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind health");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
        }
    });
    let health_file = value(&args, "--health.url-file");
    std::fs::write(&health_file, format!("http://127.0.0.1:{port}/healthz\n")).unwrap();

    let binding = value(&args, "--mcp.command");
    let quoted = binding
        .strip_prefix("channel=main,command=\"")
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("mcp command binding");
    let command = quoted.replace("\\\"", "\"").replace("\\\\", "\\");
    let mut child = Command::new(&command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start MCP command");
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{{\"probe\":\"system.status\"}}").unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    println!("fake-tunnel: mcp-response {}", line.trim());
    // Keep the MCP session open, like the real client, until the supervisor's
    // job terminates this process tree.
    let _session_input = stdin;
    let _ = child.wait();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
