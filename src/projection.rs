use crate::{Attribute, EventSeverity, ExpectedRuntime, HealthDimensions, OperationState};
use serde::{Deserialize, Serialize};

pub const PROJECTION_SCHEMA: &str = "crowsi://service-telemetry/projection/v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProjection {
    pub service_id: String,
    pub observed_at_unix_ms: i64,
    pub valid_until_unix_ms: i64,
    pub expected_runtime: ExpectedRuntime,
    pub dimensions: HealthDimensions,
    pub attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventView {
    pub sequence: i64,
    pub event_id: String,
    pub service_id: String,
    pub occurred_at_unix_ms: i64,
    pub severity: EventSeverity,
    pub name: String,
    pub outcome: OperationState,
    pub reason_code: Option<String>,
    pub attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricView {
    pub sequence: i64,
    pub service_id: String,
    pub observed_at_unix_ms: i64,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryProjection {
    pub schema: String,
    pub runtime_instance_id: String,
    pub generated_at_unix_ms: i64,
    pub revision: i64,
    pub services: Vec<ServiceProjection>,
    pub recent_events: Vec<EventView>,
    pub recent_metrics: Vec<MetricView>,
}
