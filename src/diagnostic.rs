use crate::{Attribute, EventSeverity, EventSignal, OperationState, TelemetryError};
use serde::{Deserialize, Serialize};

pub const DIAGNOSTIC_EVENT_SCHEMA: &str = "crowsi://service-telemetry/diagnostic-event/v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticEventV1 {
    pub schema: String,
    pub event_id: String,
    pub occurred_at_unix_ms: i64,
    pub component: String,
    pub operation: String,
    pub phase: String,
    pub outcome: OperationState,
    pub reason_code: String,
    pub contains_sensitive_values: bool,
}

impl TryFrom<DiagnosticEventV1> for EventSignal {
    type Error = TelemetryError;

    fn try_from(value: DiagnosticEventV1) -> Result<Self, Self::Error> {
        if value.schema != DIAGNOSTIC_EVENT_SCHEMA {
            return Err(TelemetryError::InvalidInput(
                "unsupported diagnostic schema".into(),
            ));
        }
        if value.contains_sensitive_values {
            return Err(TelemetryError::InvalidInput(
                "diagnostic event declares sensitive values".into(),
            ));
        }
        let severity = match value.outcome {
            OperationState::Succeeded
            | OperationState::InProgress
            | OperationState::NotApplicable => EventSeverity::Info,
            OperationState::Rejected => EventSeverity::Warning,
            OperationState::Failed => EventSeverity::Error,
        };
        Ok(Self {
            event_id: value.event_id,
            service_id: value.component,
            occurred_at_unix_ms: value.occurred_at_unix_ms,
            severity,
            name: value.phase,
            outcome: value.outcome,
            reason_code: Some(value.reason_code),
            attributes: vec![Attribute {
                key: "operation".into(),
                value: value.operation,
            }],
        })
    }
}
