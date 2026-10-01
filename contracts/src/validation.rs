use crate::{EventSignal, HealthSignal, MetricSignal, Signal};
use std::fmt;

const MAX_FUTURE_CLOCK_SKEW_MS: i64 = 5 * 60 * 1_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractError(&'static str);

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ContractError {}

/// Validates one signal against deterministic bounds and a caller-provided clock.
///
/// # Errors
/// Returns an error when an identifier, timestamp, duration, or metric value is invalid.
pub fn validate_signal_at(signal: &Signal, now_ms: i64) -> Result<(), ContractError> {
    match signal {
        Signal::Health(value) => validate_health(value, now_ms),
        Signal::Event(value) => validate_event(value, now_ms),
        Signal::Metric(value) => validate_metric(value, now_ms),
    }
}

fn validate_health(value: &HealthSignal, now_ms: i64) -> Result<(), ContractError> {
    validate_service_id(&value.service_id)?;
    if !(1_000..=300_000).contains(&value.valid_for_ms) {
        return Err(ContractError("validForMs must be between 1000 and 300000"));
    }
    timestamp(value.observed_at_unix_ms, now_ms)
}

fn validate_event(value: &EventSignal, now_ms: i64) -> Result<(), ContractError> {
    identifier(&value.event_id, 128)?;
    validate_service_id(&value.service_id)?;
    identifier(&value.name, 96)?;
    if let Some(reason) = &value.reason_code {
        identifier(reason, 96)?;
    }
    timestamp(value.occurred_at_unix_ms, now_ms)
}

fn validate_metric(value: &MetricSignal, now_ms: i64) -> Result<(), ContractError> {
    validate_service_id(&value.service_id)?;
    identifier(&value.name, 96)?;
    identifier(&value.unit, 32)?;
    if !value.value.is_finite() {
        return Err(ContractError("metric value must be finite"));
    }
    timestamp(value.observed_at_unix_ms, now_ms)
}

fn timestamp(value: i64, now_ms: i64) -> Result<(), ContractError> {
    if value <= 0 {
        return Err(ContractError(
            "timestamp must be positive Unix milliseconds",
        ));
    }
    if value > now_ms.saturating_add(MAX_FUTURE_CLOCK_SKEW_MS) {
        return Err(ContractError(
            "timestamp exceeds permitted future clock skew",
        ));
    }
    Ok(())
}

/// Validates an ecosystem service identifier without resolving external state.
///
/// # Errors
/// Returns an error when the value is empty, unbounded, or contains unsupported bytes.
pub fn validate_service_id(value: &str) -> Result<(), ContractError> {
    identifier(value, 96)
}

fn identifier(value: &str, max: usize) -> Result<(), ContractError> {
    let valid = !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-/".contains(&byte));
    valid
        .then_some(())
        .ok_or(ContractError("invalid bounded identifier"))
}
