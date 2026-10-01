#![forbid(unsafe_code)]
#![doc = "Owner-local, privacy-bounded service telemetry and SSE projections."]

mod attribute_policy;
mod connection_limit;
mod diagnostic;
mod error;
mod freshness;
mod http;
mod interface_contract;
mod path_chain;
mod privacy;
mod projection;
mod runtime;
mod runtime_handler;
mod runtime_instance;
mod runtime_server;
mod secure_path;
mod store;
mod store_maintenance;
#[cfg(test)]
mod store_maintenance_tests;
mod store_metadata;
mod store_read;
#[cfg(test)]
mod store_tests;
mod store_write;
mod validation;

pub use crowsi_telemetry_contracts::INGEST_RECEIPT_SCHEMA;
pub use crowsi_telemetry_contracts::{
    Attribute, EventSeverity, EventSignal, ExpectedRuntime, FreshnessState, HealthDimensions,
    HealthObservationDimensions, HealthSignal, MetricSignal, OperationState, ServiceState, Signal,
    TelemetryIngestReceiptV1, TelemetrySink, VerificationState,
};
pub use diagnostic::{DIAGNOSTIC_EVENT_SCHEMA, DiagnosticEventV1};
pub use error::TelemetryError;
pub use interface_contract::{ContractCheckReport, contract_check};
pub use privacy::{PrivacyReport, sanitize_attributes};
pub use projection::{EventView, MetricView, ServiceProjection, TelemetryProjection};
pub use runtime::{RuntimeConfig, TelemetryRuntime};
pub use store::{RetentionPolicy, TelemetryStore};

/// Creates a new cryptographically random owner-only runtime token.
///
/// # Errors
/// Returns an error when the path exists, is unsafe, or randomness is unavailable.
pub fn generate_runtime_token(path: &std::path::Path) -> Result<(), TelemetryError> {
    secure_path::generate_token(path)
}
