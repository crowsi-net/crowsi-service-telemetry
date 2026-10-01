mod support;

use crowsi_service_telemetry::{ExpectedRuntime, TelemetryStore, generate_runtime_token};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use support::telemetry_http::{
    Server, free_address, is_instance_id, request, stream_head, wait_ready,
};

#[test]
#[ignore = "requires an owner-local loopback socket"]
fn authenticated_ingest_projection_and_sse_are_reactive() {
    let directory = tempfile::tempdir().expect("tempdir");
    let db = directory.path().join("state/telemetry.sqlite3");
    TelemetryStore::initialize(&db, "coela-control", ExpectedRuntime::Continuous)
        .expect("database");
    let read_path = directory.path().join("state/read.token");
    let ingest_path = directory.path().join("state/ingest.token");
    generate_runtime_token(&read_path).expect("read token");
    generate_runtime_token(&ingest_path).expect("ingest token");
    let read_token = std::fs::read_to_string(&read_path).expect("read token value");
    let ingest_token = std::fs::read_to_string(&ingest_path).expect("ingest token value");
    let address = free_address();
    let child = Command::new(env!("CARGO_BIN_EXE_crowsi-service-telemetry"))
        .args(["serve", "--bind", &address.to_string(), "--read-token-file"])
        .arg(&read_path)
        .args(["--ingest-token-file"])
        .arg(&ingest_path)
        .args(["--service-db"])
        .arg(format!("coela-control={}", db.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("server");
    let _server = Server(child);
    wait_ready(address);

    let unauthorized = request(
        address,
        "GET /v1/telemetry/projection HTTP/1.1\r\nHost: local\r\n\r\n",
    );
    assert!(unauthorized.starts_with("HTTP/1.1 401"));
    let invalid = request(
        address,
        &format!(
            "POST /v1/telemetry/signals HTTP/1.1\r\nHost: local\r\nAuthorization: Bearer {ingest_token}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{{}}"
        ),
    );
    assert!(invalid.starts_with("HTTP/1.1 400"));

    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis(),
    )
    .expect("time");
    let body = serde_json::json!({
        "signal":"health", "serviceId":"coela-control", "observedAtUnixMs":now,
        "validForMs":1000,
        "dimensions":{"liveness":"healthy","readiness":"healthy","dependency":"unknown",
            "verification":"verified","operationResult":"succeeded"}, "attributes":[]
    })
    .to_string();
    let accepted = request(
        address,
        &format!(
            "POST /v1/telemetry/signals HTTP/1.1\r\nHost: local\r\nAuthorization: Bearer {ingest_token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    );
    assert!(accepted.starts_with("HTTP/1.1 202"));
    let projection = request(
        address,
        &format!(
            "GET /v1/telemetry/projection HTTP/1.1\r\nHost: local\r\nAuthorization: Bearer {read_token}\r\n\r\n"
        ),
    );
    assert!(projection.contains("\"telemetryFreshness\":\"fresh\""));
    let stream = stream_head(address, &read_token);
    assert!(stream.contains("Content-Type: text/event-stream"));
    assert!(stream.contains("event: projection"));
    let event_id = stream
        .lines()
        .find_map(|line| line.strip_prefix("id: "))
        .expect("SSE id");
    let payload: serde_json::Value = serde_json::from_str(
        stream
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .expect("SSE data"),
    )
    .expect("projection payload");
    let instance = payload["runtimeInstanceId"]
        .as_str()
        .expect("runtime instance");
    assert_eq!(event_id, format!("{instance}:{}", payload["revision"]));
    assert!(is_instance_id(instance));
}
