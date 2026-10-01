use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("invalid telemetry input: {0}")]
    InvalidInput(String),
    #[error("telemetry path violates the owner-local policy: {0}")]
    InvalidPath(String),
    #[error("telemetry runtime authorization failed")]
    Unauthorized,
    #[error("telemetry storage failed: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("telemetry JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("telemetry I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TelemetryError>;
