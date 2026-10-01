use crowsi_service_telemetry::{
    ExpectedRuntime, HealthObservationDimensions, HealthSignal, OperationState, ServiceState,
    Signal, TelemetryStore, VerificationState,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn older_and_equal_health_replays_cannot_replace_the_first_observation() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let store =
        TelemetryStore::initialize(&path, "service", ExpectedRuntime::Continuous).expect("store");
    let observed = now_ms();
    let initial = health(observed, ServiceState::Healthy);
    assert_eq!(
        store
            .record(Signal::Health(initial.clone()))
            .expect("initial"),
        4
    );
    assert_eq!(store.record(Signal::Health(initial)).expect("duplicate"), 4);
    assert_eq!(
        store
            .record(Signal::Health(health(observed, ServiceState::Unavailable)))
            .expect("equal conflict"),
        4
    );
    assert_eq!(
        store
            .record(Signal::Health(health(observed - 1, ServiceState::Degraded)))
            .expect("older replay"),
        4
    );
    let service = store.projection().expect("projection").services.remove(0);
    assert_eq!(service.dimensions.readiness, ServiceState::Healthy);
}

fn health(observed: i64, state: ServiceState) -> HealthSignal {
    HealthSignal {
        service_id: "service".into(),
        observed_at_unix_ms: observed,
        valid_for_ms: 15_000,
        dimensions: HealthObservationDimensions {
            liveness: state,
            readiness: state,
            dependency: ServiceState::Unknown,
            verification: VerificationState::Verified,
            operation_result: OperationState::Succeeded,
        },
        attributes: vec![],
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| {
            i64::try_from(value.as_millis()).unwrap_or(i64::MAX)
        })
}
