use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

macro_rules! kebab_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
        #[serde(rename_all = "kebab-case")]
        pub enum $name { $($variant),+ }
    };
}

kebab_enum!(ServiceState {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
    NotApplicable
});
kebab_enum!(FreshnessState {
    Fresh,
    Degraded,
    Stale,
    Unknown
});
kebab_enum!(VerificationState {
    Verified,
    Rejected,
    Unverified,
    NotApplicable
});
kebab_enum!(OperationState {
    Succeeded,
    Rejected,
    Failed,
    InProgress,
    NotApplicable
});
kebab_enum!(ExpectedRuntime {
    Continuous,
    OnDemand,
    External
});
kebab_enum!(EventSeverity {
    Info,
    Warning,
    Error,
    Critical
});

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HealthDimensions {
    pub liveness: ServiceState,
    pub readiness: ServiceState,
    pub dependency: ServiceState,
    pub telemetry_freshness: FreshnessState,
    pub verification: VerificationState,
    pub operation_result: OperationState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HealthObservationDimensions {
    pub liveness: ServiceState,
    pub readiness: ServiceState,
    pub dependency: ServiceState,
    pub verification: VerificationState,
    pub operation_result: OperationState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HealthSignal {
    pub service_id: String,
    pub observed_at_unix_ms: i64,
    pub valid_for_ms: i64,
    pub dimensions: HealthObservationDimensions,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventSignal {
    pub event_id: String,
    pub service_id: String,
    pub occurred_at_unix_ms: i64,
    pub severity: EventSeverity,
    pub name: String,
    pub outcome: OperationState,
    pub reason_code: Option<String>,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricSignal {
    pub service_id: String,
    pub observed_at_unix_ms: i64,
    pub name: String,
    pub value: f64,
    pub unit: String,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "signal", rename_all = "kebab-case")]
pub enum Signal {
    Health(HealthSignal),
    Event(EventSignal),
    Metric(MetricSignal),
}

impl Signal {
    #[must_use]
    pub fn service_id(&self) -> &str {
        match self {
            Self::Health(value) => &value.service_id,
            Self::Event(value) => &value.service_id,
            Self::Metric(value) => &value.service_id,
        }
    }
}
