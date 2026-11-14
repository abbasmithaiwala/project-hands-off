//! A dependency-free health probe for container health checks (`hands-off healthcheck`).

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(2);

/// Ask the server listening on `bind` whether it is healthy.
///
/// An unspecified bind address (`0.0.0.0`) is probed through loopback. Returns `true` only for
/// an HTTP 200 answer from the health route.
#[must_use]
pub fn probe(bind: SocketAddr) -> bool {
    let target = if bind.ip().is_unspecified() {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), bind.port())
    } else {
        bind
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&target, TIMEOUT) else {
        return false;
    };
    if stream.set_read_timeout(Some(TIMEOUT)).is_err()
        || stream.set_write_timeout(Some(TIMEOUT)).is_err()
    {
        return false;
    }
    let request = "GET /api/v1/health HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut head = [0_u8; 32];
    let Ok(read) = stream.read(&mut head) else {
        return false;
    };
    head[..read].starts_with(b"HTTP/1.1 200") || head[..read].starts_with(b"HTTP/1.0 200")
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    fn answering(status: &'static str) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 256];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(
                    format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\n\r\n").as_bytes(),
                );
            }
        });
        address
    }

    #[test]
    fn a_healthy_server_passes() {
        assert!(probe(answering("200 OK")));
    }

    #[test]
    fn an_unhealthy_server_fails() {
        assert!(!probe(answering("503 Service Unavailable")));
    }

    #[test]
    fn nothing_listening_fails() {
        let free = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        assert!(!probe(free));
    }
}
