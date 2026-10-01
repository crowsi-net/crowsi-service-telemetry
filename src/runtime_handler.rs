use crate::diagnostic::DiagnosticEventV1;
use crate::error::Result;
use crate::http::{Request, authorized, read_request, response};
use crate::interface_contract::{
    HEALTH_PATH, INGEST_RECEIPT_SCHEMA, PROJECTION_PATH, SIGNALS_PATH, SSE_EVENT, STREAM_PATH,
};
use crate::{Signal, TelemetryError, TelemetryRuntime};
use serde::Serialize;
use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

const SSE_HEADERS: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nX-Accel-Buffering: no\r\nX-Content-Type-Options: nosniff\r\nConnection: keep-alive\r\n\r\n";

pub fn reject_capacity(stream: &mut TcpStream) {
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let _ = write_error(
        stream,
        "503 Service Unavailable",
        "connection-capacity-exhausted",
    );
}

pub fn handle(runtime: &TelemetryRuntime, stream: &mut TcpStream) -> Result<()> {
    let Ok(request) = read_request(stream) else {
        return write_error(stream, "400 Bad Request", "request-rejected");
    };
    if request.method == "GET" && request.path == STREAM_PATH {
        if !authorized(&request, runtime.read_token()) {
            return write_error(stream, "401 Unauthorized", "unauthorized");
        }
        return serve_sse(runtime, stream);
    }
    if let Err(error) = dispatch(runtime, stream, &request) {
        let (status, reason) = safe_error(&error);
        return write_error(stream, status, reason);
    }
    Ok(())
}

fn dispatch(runtime: &TelemetryRuntime, stream: &mut TcpStream, request: &Request) -> Result<()> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", HEALTH_PATH) => {
            write_json(stream, "200 OK", &serde_json::json!({"status":"ready"}))
        }
        ("GET", PROJECTION_PATH) => {
            if !authorized(request, runtime.read_token()) {
                return write_error(stream, "401 Unauthorized", "unauthorized");
            }
            write_json(stream, "200 OK", &runtime.projection()?)
        }
        ("POST", SIGNALS_PATH) => {
            if !authorized(request, runtime.ingest_token()) {
                return write_error(stream, "401 Unauthorized", "unauthorized");
            }
            ingest(runtime, stream, request)
        }
        _ => write_error(stream, "404 Not Found", "route-not-found"),
    }
}

fn ingest(runtime: &TelemetryRuntime, stream: &mut TcpStream, request: &Request) -> Result<()> {
    if request.headers.get("content-type").map(String::as_str) != Some("application/json") {
        return write_error(
            stream,
            "415 Unsupported Media Type",
            "content-type-required",
        );
    }
    let value: serde_json::Value = serde_json::from_slice(&request.body)?;
    let signal = if value.get("signal").is_some() {
        serde_json::from_value::<Signal>(value)?
    } else {
        Signal::Event(serde_json::from_value::<DiagnosticEventV1>(value)?.try_into()?)
    };
    let revision = runtime.ingest(signal)?;
    write_json(
        stream,
        "202 Accepted",
        &serde_json::json!({
            "schema":INGEST_RECEIPT_SCHEMA, "accepted":true, "revision":revision
        }),
    )
}

fn serve_sse(runtime: &TelemetryRuntime, stream: &mut TcpStream) -> Result<()> {
    stream.write_all(SSE_HEADERS.as_bytes())?;
    stream.write_all(b"retry: 1500\n\n")?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut fingerprint = String::new();
    let mut heartbeat = Instant::now();
    loop {
        let projection = runtime.projection()?;
        let next = projection_fingerprint(&projection)?;
        if next != fingerprint {
            let data = serde_json::to_string(&projection)?;
            stream.write_all(
                format!(
                    "id: {}:{}\nevent: {SSE_EVENT}\ndata: {data}\n\n",
                    projection.runtime_instance_id, projection.revision
                )
                .as_bytes(),
            )?;
            stream.flush()?;
            fingerprint = next;
            heartbeat = Instant::now();
        } else if heartbeat.elapsed() >= Duration::from_secs(15) {
            stream.write_all(b": heartbeat\n\n")?;
            stream.flush()?;
            heartbeat = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn projection_fingerprint(projection: &crate::TelemetryProjection) -> Result<String> {
    let freshness = projection
        .services
        .iter()
        .map(|service| (&service.service_id, service.dimensions.telemetry_freshness))
        .collect::<Vec<_>>();
    Ok(serde_json::to_string(&(
        &projection.runtime_instance_id,
        projection.revision,
        freshness,
    ))?)
}

fn safe_error(error: &TelemetryError) -> (&'static str, &'static str) {
    match error {
        TelemetryError::Unauthorized => ("401 Unauthorized", "unauthorized"),
        TelemetryError::InvalidInput(_) => ("400 Bad Request", "signal-rejected"),
        TelemetryError::Json(_) => ("400 Bad Request", "invalid-json"),
        TelemetryError::Storage(_) | TelemetryError::Io(_) | TelemetryError::InvalidPath(_) => {
            ("503 Service Unavailable", "telemetry-unavailable")
        }
    }
}

fn write_json<T: Serialize>(stream: &mut TcpStream, status: &str, value: &T) -> Result<()> {
    let body = serde_json::to_string(value)?;
    stream.write_all(&response(status, "application/json; charset=utf-8", &body))?;
    Ok(())
}

fn write_error(stream: &mut TcpStream, status: &str, reason: &str) -> Result<()> {
    write_json(stream, status, &serde_json::json!({"error":reason}))
}
