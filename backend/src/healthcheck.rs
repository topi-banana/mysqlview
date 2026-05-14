//! Self-contained TCP health-check probe.
//!
//! Intended for use as `mysqlview-backend --healthcheck`, which is how
//! the scratch container's HEALTHCHECK invokes the same binary as a
//! client. The probe opens a TCP connection to `127.0.0.1:{port}`,
//! issues a minimal HTTP/1.1 `GET /api/health`, and inspects the
//! status line.
//!
//! No tokio, no reqwest — just `std::net` so the probe stays under a
//! few hundred microseconds and pulls in zero extra dependencies.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(3);

/// Returns `true` when the server responded with a `200` status line.
pub fn run(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let mut stream = match TcpStream::connect_timeout(&addr, TIMEOUT) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("healthcheck: connect to {addr} failed: {err}");
            return false;
        }
    };
    let _ = stream.set_read_timeout(Some(TIMEOUT));
    let _ = stream.set_write_timeout(Some(TIMEOUT));

    let request = format!(
        "GET /api/health HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         User-Agent: mysqlview-healthcheck\r\n\
         Connection: close\r\n\
         \r\n"
    );
    if let Err(err) = stream.write_all(request.as_bytes()) {
        eprintln!("healthcheck: write failed: {err}");
        return false;
    }

    // Read just enough bytes to inspect the status line. A response
    // looks like "HTTP/1.1 200 OK\r\nContent-Type: ..." so 64 bytes is
    // plenty for the first line.
    let mut buf = Vec::with_capacity(128);
    let mut chunk = [0u8; 64];
    while buf.len() < 64 {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(err) => {
                eprintln!("healthcheck: read failed: {err}");
                return false;
            }
        }
    }

    is_status_200(&buf)
}

fn is_status_200(response: &[u8]) -> bool {
    let head = std::str::from_utf8(response).unwrap_or("");
    let status_line = head.split("\r\n").next().unwrap_or("");
    status_line.starts_with("HTTP/") && status_line.split_whitespace().nth(1) == Some("200")
}

#[cfg(test)]
mod tests {
    use super::is_status_200;

    #[test]
    fn matches_200_status_lines() {
        assert!(is_status_200(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n"
        ));
        assert!(is_status_200(b"HTTP/1.0 200 OK\r\n"));
    }

    #[test]
    fn rejects_non_200_status_lines() {
        assert!(!is_status_200(b"HTTP/1.1 503 Service Unavailable\r\n"));
        assert!(!is_status_200(b"HTTP/1.1 404 Not Found\r\n"));
        assert!(!is_status_200(b"HTTP/1.1 500 Internal Server Error\r\n"));
    }

    #[test]
    fn rejects_non_http_prefix() {
        assert!(!is_status_200(b"garbage 200 something\r\n"));
        assert!(!is_status_200(b""));
    }
}
