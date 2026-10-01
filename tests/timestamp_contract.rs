use crowsi_service_telemetry::{
    EventSeverity, EventSignal, ExpectedRuntime, HealthObservationDimensions, HealthSignal,
    MetricSignal, OperationState, ServiceState, Signal, TelemetryStore, VerificationState,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn rejects_future_timestamps_for_every_signal_kind() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let store = TelemetryStore::initialize(&path, "clock", ExpectedRuntime::Continuous)
        .expect("initialize");
    let future = now_ms() + 600_000;
    let health = Signal::Health(HealthSignal {
        service_id: "clock".into(),
        observed_at_unix_ms: future,
        valid_for_ms: 15_000,
        dimensions: dimensions(),
        attributes: vec![],
    });
    let event = Signal::Event(EventSignal {
        event_id: "future-event".into(),
        service_id: "clock".into(),
        occurred_at_unix_ms: future,
        severity: EventSeverity::Warning,
        name: "clock-check".into(),
        outcome: OperationState::Rejected,
        reason_code: Some("future-clock".into()),
        attributes: vec![],
    });
    let metric = Signal::Metric(MetricSignal {
        service_id: "clock".into(),
        observed_at_unix_ms: future,
        name: "clock-offset".into(),
        value: 600_000.0,
        unit: "ms".into(),
        attributes: vec![],
    });
    for signal in [health, event, metric] {
        assert!(store.record(signal).is_err());
    }
}

fn dimensions() -> HealthObservationDimensions {
    HealthObservationDimensions {
        liveness: ServiceState::Healthy,
        readiness: ServiceState::Healthy,
        dependency: ServiceState::Unknown,
        verification: VerificationState::Verified,
        operation_result: OperationState::Succeeded,
    }
}

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis(),
    )
    .expect("timestamp")
}
