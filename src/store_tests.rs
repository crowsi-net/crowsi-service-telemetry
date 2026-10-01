use crate::{
    ExpectedRuntime, FreshnessState, HealthObservationDimensions, HealthSignal, OperationState,
    ServiceState, Signal, TelemetryStore, VerificationState,
};

fn health(observed_at_unix_ms: i64) -> Signal {
    Signal::Health(HealthSignal {
        service_id: "service".into(),
        observed_at_unix_ms,
        valid_for_ms: 1_000,
        dimensions: HealthObservationDimensions {
            liveness: ServiceState::Healthy,
            readiness: ServiceState::Healthy,
            dependency: ServiceState::Healthy,
            verification: VerificationState::Verified,
            operation_result: OperationState::Succeeded,
        },
        attributes: vec![],
    })
}

#[test]
fn effective_revision_advances_on_freshness_and_a_new_write() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let store = TelemetryStore::initialize(&path, "service", ExpectedRuntime::Continuous)
        .expect("initialize");
    store.record(health(10_000)).expect("first write");
    let fresh = store.projection_at(10_500).expect("fresh");
    let degraded = store.projection_at(11_500).expect("degraded");
    let stale = store.projection_at(12_500).expect("stale");
    assert_eq!(
        fresh.services[0].dimensions.telemetry_freshness,
        FreshnessState::Fresh
    );
    assert!(fresh.revision < degraded.revision && degraded.revision < stale.revision);
    store.record(health(13_000)).expect("second write");
    let refreshed = store.projection_at(13_500).expect("refreshed");
    assert_eq!(
        refreshed.services[0].dimensions.telemetry_freshness,
        FreshnessState::Fresh
    );
    assert!(stale.revision < refreshed.revision);
}
