use crowsi_service_telemetry::{
    Attribute, EventSeverity, EventSignal, ExpectedRuntime, HealthObservationDimensions,
    HealthSignal, OperationState, RetentionPolicy, ServiceState, Signal, TelemetryStore,
    VerificationState,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis(),
    )
    .expect("timestamp")
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

#[test]
fn stores_only_the_registered_service_and_sanitized_attributes() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let store = TelemetryStore::initialize(&path, "coela-control", ExpectedRuntime::Continuous)
        .expect("initialize");
    store
        .record(Signal::Health(HealthSignal {
            service_id: "coela-control".into(),
            observed_at_unix_ms: now_ms(),
            valid_for_ms: 15_000,
            dimensions: dimensions(),
            attributes: vec![
                Attribute {
                    key: "environment".into(),
                    value: "local".into(),
                },
                Attribute {
                    key: "secret".into(),
                    value: "not retained".into(),
                },
                Attribute {
                    key: "endpoint_id".into(),
                    value: "https://internal/path?key=value".into(),
                },
                Attribute {
                    key: "operation".into(),
                    value: "github_pat_abcdef012345".into(),
                },
                Attribute {
                    key: "protocol".into(),
                    value: "http".into(),
                },
            ],
        }))
        .expect("record");
    let projection = store.projection().expect("projection");
    assert_eq!(projection.revision, 4);
    assert_eq!(projection.runtime_instance_id.len(), 32);
    assert_eq!(
        store
            .projection()
            .expect("repeat projection")
            .runtime_instance_id,
        projection.runtime_instance_id
    );
    assert_eq!(projection.services[0].attributes.len(), 2);
    assert_eq!(projection.services[0].attributes[0].key, "environment");
    assert_eq!(projection.services[0].attributes[1].key, "protocol");

    let wrong = Signal::Health(HealthSignal {
        service_id: "other".into(),
        observed_at_unix_ms: now_ms(),
        valid_for_ms: 15_000,
        dimensions: dimensions(),
        attributes: vec![],
    });
    assert!(store.record(wrong).is_err());
}

#[test]
fn event_id_is_idempotent_and_retention_is_bounded() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let store = TelemetryStore::initialize(&path, "pa-agent", ExpectedRuntime::OnDemand)
        .expect("initialize")
        .with_retention(RetentionPolicy {
            max_events: 2,
            max_metrics: 2,
            max_age_ms: 60_000,
        });
    for index in 0..3 {
        let signal = Signal::Event(EventSignal {
            event_id: format!("event-{index}"),
            service_id: "pa-agent".into(),
            occurred_at_unix_ms: now_ms(),
            severity: EventSeverity::Warning,
            name: "possession-proof".into(),
            outcome: OperationState::Rejected,
            reason_code: Some("authenticator-data-rejected".into()),
            attributes: vec![],
        });
        store.record(signal).expect("record event");
    }
    let duplicate = Signal::Event(EventSignal {
        event_id: "event-2".into(),
        service_id: "pa-agent".into(),
        occurred_at_unix_ms: now_ms(),
        severity: EventSeverity::Warning,
        name: "possession-proof".into(),
        outcome: OperationState::Rejected,
        reason_code: None,
        attributes: vec![],
    });
    assert_eq!(store.record(duplicate).expect("deduplicate"), 12);
    let projection = store.projection().expect("projection");
    assert_eq!(projection.recent_events.len(), 2);
    assert_eq!(projection.revision, 12);
}

#[test]
fn initialize_is_idempotent_only_for_the_registered_expected_runtime() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    TelemetryStore::initialize(&path, "service", ExpectedRuntime::Continuous).expect("initial");
    TelemetryStore::initialize(&path, "service", ExpectedRuntime::Continuous)
        .expect("idempotent initialize");
    assert!(TelemetryStore::initialize(&path, "service", ExpectedRuntime::OnDemand).is_err());
}
