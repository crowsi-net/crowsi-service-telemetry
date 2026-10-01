#![forbid(unsafe_code)]
#![doc = "Transport-neutral, privacy-bounded telemetry producer contracts."]

mod port;
mod signal;
mod validation;

pub use port::{
    DeliveryFailure, DeliveryFailureKind, TelemetryDeliveryFuture, TelemetryIngestReceiptV1,
    TelemetrySink,
};
pub use signal::{
    Attribute, EventSeverity, EventSignal, ExpectedRuntime, FreshnessState, HealthDimensions,
    HealthObservationDimensions, HealthSignal, MetricSignal, OperationState, ServiceState, Signal,
    VerificationState,
};
pub use validation::{ContractError, validate_service_id, validate_signal_at};

pub const INGEST_RECEIPT_SCHEMA: &str = "crowsi://service-telemetry/ingest-receipt/v1";
