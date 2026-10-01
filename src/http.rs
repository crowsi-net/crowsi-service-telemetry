use crate::{TelemetryError, error::Result};
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;

pub const MAX_BODY: usize = 64 * 1024;

pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub fn read_request(stream: &mut TcpStream) -> Result<Request> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut bytes = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Err(TelemetryError::InvalidInput(
                "incomplete HTTP request".into(),
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > MAX_BODY + 16_384 {
            return Err(TelemetryError::InvalidInput(
                "HTTP request exceeds limit".into(),
            ));
        }
        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let head = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| TelemetryError::InvalidInput("HTTP headers are not UTF-8".into()))?;
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_owned();
    let path = request_line.next().unwrap_or_default().to_owned();
    if request_line.next() != Some("HTTP/1.1") || request_line.next().is_some() {
        return Err(TelemetryError::InvalidInput(
            "HTTP/1.1 request line required".into(),
        ));
    }
    let mut headers = HashMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| TelemetryError::InvalidInput("malformed HTTP header".into()))?;
        let key = key.trim().to_ascii_lowercase();
        if headers.insert(key, value.trim().to_owned()).is_some() {
            return Err(TelemetryError::InvalidInput("duplicate HTTP header".into()));
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err(TelemetryError::InvalidInput(
            "transfer encoding is unsupported".into(),
        ));
    }
    let length = headers.get("content-length").map_or(Ok(0), |v| {
        v.parse::<usize>()
            .map_err(|_| TelemetryError::InvalidInput("invalid content length".into()))
    })?;
    if length > MAX_BODY {
        return Err(TelemetryError::InvalidInput(
            "HTTP body exceeds limit".into(),
        ));
    }
    while bytes.len() - header_end < length {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Err(TelemetryError::InvalidInput("incomplete HTTP body".into()));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(Request {
        method,
        path,
        headers,
        body: bytes[header_end..header_end + length].to_vec(),
    })
}

pub fn authorized(request: &Request, token: &str) -> bool {
    let Some(value) = request.headers.get("authorization") else {
        return false;
    };
    let Some(candidate) = value.strip_prefix("Bearer ") else {
        return false;
    };
    constant_time_equal(candidate.as_bytes(), token.as_bytes())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

pub fn response(status: &str, content_type: &str, body: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
}
