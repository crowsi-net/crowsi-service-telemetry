use crate::Signal;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TelemetryIngestReceiptV1 {
    pub schema: String,
    pub accepted: bool,
    pub revision: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryFailureKind {
    InvalidSignal,
    Unauthorized,
    Unavailable,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryFailure {
    pub kind: DeliveryFailureKind,
    pub reason_code: &'static str,
}

pub type TelemetryDeliveryFuture<'a> =
    Pin<Box<dyn Future<Output = Result<TelemetryIngestReceiptV1, DeliveryFailure>> + Send + 'a>>;

/// Transport-neutral producer port. Endpoint, protocol, credentials, retry,
/// and process placement are supplied by an adapter outside this contract.
pub trait TelemetrySink: Send + Sync {
    fn submit(&self, signal: Signal) -> TelemetryDeliveryFuture<'_>;
}
