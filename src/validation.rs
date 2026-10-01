use crate::{Signal, TelemetryError};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn validate_signal(signal: &Signal) -> Result<(), TelemetryError> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| {
            i64::try_from(value.as_millis()).unwrap_or(i64::MAX)
        });
    crowsi_telemetry_contracts::validate_signal_at(signal, now_ms)
        .map_err(|error| TelemetryError::InvalidInput(error.to_string()))
}

pub fn validate_service_id(value: &str) -> Result<(), TelemetryError> {
    crowsi_telemetry_contracts::validate_service_id(value)
        .map_err(|error| TelemetryError::InvalidInput(error.to_string()))
}
