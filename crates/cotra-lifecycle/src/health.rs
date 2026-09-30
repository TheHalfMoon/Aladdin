//! Loopback-only probe of the health endpoint advertised by the official
//! tunnel client through its health URL file.

use crate::LifecycleError;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthUrl {
    pub port: u16,
    pub path: String,
}

/// Parses `http://127.0.0.1:<port>/<path>`; any other host, scheme, or shape
/// is refused so the probe can never leave the loopback interface.
pub fn parse_health_url(text: &str) -> Result<HealthUrl, LifecycleError> {
    let text = text.trim();
    let rest = text
        .strip_prefix("http://127.0.0.1:")
        .ok_or_else(|| LifecycleError::state("health URL is not an http://127.0.0.1 URL"))?;
    let (port, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let port: u16 = port
        .parse()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| LifecycleError::state("health URL has an invalid port"))?;
    if path.len() > 256 || !path.bytes().all(|b| b.is_ascii_graphic()) || path.contains('@') {
        return Err(LifecycleError::state("health URL has an invalid path"));
    }
    Ok(HealthUrl {
        port,
        path: path.to_string(),
    })
}

/// Sends one bounded HTTP/1.1 GET to the loopback health endpoint and returns
/// the status code.
pub fn probe(url: &HealthUrl, timeout: Duration) -> Result<u16, LifecycleError> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, url.port));
    let mut stream = TcpStream::connect_timeout(&address, timeout)
        .map_err(|error| LifecycleError::state(format!("health endpoint unreachable: {error}")))?;
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| LifecycleError::state(format!("health probe setup: {error}")))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        url.path, url.port
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| LifecycleError::state(format!("health probe write: {error}")))?;
    let mut buffer = [0u8; 64];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| LifecycleError::state(format!("health probe read: {error}")))?;
    let head = String::from_utf8_lossy(&buffer[..read]);
    head.strip_prefix("HTTP/1.")
        .and_then(|rest| rest.get(2..5))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| LifecycleError::state("health endpoint returned a non-HTTP response"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn only_loopback_http_urls_are_accepted() {
        let url = parse_health_url("http://127.0.0.1:4555/healthz\n").unwrap();
        assert_eq!(
            url,
            HealthUrl {
                port: 4555,
                path: "/healthz".into()
            }
        );
        assert_eq!(parse_health_url("http://127.0.0.1:80").unwrap().path, "/");
        for bad in [
            "https://127.0.0.1:1/",
            "http://localhost:1/",
            "http://127.0.0.2:1/",
            "http://127.0.0.1:0/",
            "http://127.0.0.1:99999/",
            "http://127.0.0.1:1@evil/",
            "http://127.0.0.1:1/a b",
            "http://10.0.0.1:1/",
        ] {
            assert!(parse_health_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn probe_reads_the_status_code() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 256];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = socket.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
        });
        let url = HealthUrl {
            port,
            path: "/healthz".into(),
        };
        assert_eq!(probe(&url, Duration::from_secs(5)).unwrap(), 200);
    }

    #[test]
    fn closed_port_is_unreachable_not_healthy() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let url = HealthUrl {
            port,
            path: "/".into(),
        };
        assert!(probe(&url, Duration::from_millis(500)).is_err());
    }
}
