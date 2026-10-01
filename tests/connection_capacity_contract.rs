use crowsi_service_telemetry::{ExpectedRuntime, TelemetryStore, generate_runtime_token};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires an owner-local loopback socket"]
fn excess_connection_is_rejected_without_allocating_another_worker() {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = directory.path().join("state/telemetry.sqlite3");
    TelemetryStore::initialize(&database, "service", ExpectedRuntime::Continuous)
        .expect("database");
    let read = directory.path().join("state/read.token");
    let ingest = directory.path().join("state/ingest.token");
    generate_runtime_token(&read).expect("read token");
    generate_runtime_token(&ingest).expect("ingest token");
    let address = free_address();
    let child = Command::new(env!("CARGO_BIN_EXE_crowsi-service-telemetry"))
        .args(["serve", "--bind", &address.to_string(), "--read-token-file"])
        .arg(read)
        .args(["--ingest-token-file"])
        .arg(ingest)
        .args(["--service-db"])
        .arg(format!("service={}", database.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("runtime");
    let _server = Server(child);
    wait_ready(address);
    let mut held = Vec::new();
    for _ in 0..32 {
        let mut stream = TcpStream::connect(address).expect("held connection");
        stream.write_all(b"G").expect("partial request");
        held.push(stream);
    }
    std::thread::sleep(Duration::from_millis(200));
    let mut overflow = TcpStream::connect(address).expect("overflow connection");
    overflow
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");
    let mut response = String::new();
    overflow
        .read_to_string(&mut response)
        .expect("capacity response");
    assert!(response.starts_with("HTTP/1.1 503 Service Unavailable"));
    assert!(response.contains("connection-capacity-exhausted"));
    drop(held);
}

fn free_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral port");
    listener.local_addr().expect("address")
}

fn wait_ready(address: SocketAddr) {
    for _ in 0..50 {
        if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            let _ = stream.write_all(b"GET /health HTTP/1.1\r\nHost: local\r\n\r\n");
            let mut response = String::new();
            let _ = stream.read_to_string(&mut response);
            if response.starts_with("HTTP/1.1 200 OK") {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("runtime did not become ready");
}
