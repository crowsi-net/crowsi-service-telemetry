use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Child;
use std::time::Duration;

pub struct Server(pub Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn free_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral port");
    listener.local_addr().expect("address")
}

pub fn wait_ready(address: SocketAddr) {
    for _ in 0..50 {
        if request(address, "GET /health HTTP/1.1\r\nHost: local\r\n\r\n").contains("200 OK") {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("server did not become ready");
}

pub fn request(address: SocketAddr, request: &str) -> String {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) else {
        return String::new();
    };
    stream.write_all(request.as_bytes()).expect("request");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("response");
    String::from_utf8(bytes).expect("UTF-8 response")
}

pub fn stream_head(address: SocketAddr, token: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("SSE connection");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");
    stream.write_all(format!("GET /v1/telemetry/stream HTTP/1.1\r\nHost: local\r\nAuthorization: Bearer {token}\r\n\r\n").as_bytes()).expect("SSE request");
    let mut collected = Vec::new();
    let mut bytes = [0_u8; 4096];
    loop {
        match stream.read(&mut bytes) {
            Ok(0) => break,
            Ok(count) => {
                collected.extend_from_slice(&bytes[..count]);
                let current = String::from_utf8_lossy(&collected);
                if current.contains("event: projection")
                    && current.contains("\ndata: ")
                    && current.ends_with("\n\n")
                {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => panic!("SSE read failed: {error}"),
        }
    }
    String::from_utf8(collected).expect("UTF-8 SSE")
}

pub fn is_instance_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
