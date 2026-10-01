use crowsi_service_telemetry::{
    EventSeverity, EventSignal, ExpectedRuntime, OperationState, RetentionPolicy, Signal,
    TelemetryStore,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn pruning_without_a_new_insert_advances_the_revision() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let event = EventSignal {
        event_id: "expired-event".into(),
        service_id: "pa-agent".into(),
        occurred_at_unix_ms: now_ms() - 10_000,
        severity: EventSeverity::Warning,
        name: "possession-proof".into(),
        outcome: OperationState::Rejected,
        reason_code: Some("expired".into()),
        attributes: vec![],
    };
    let store = TelemetryStore::initialize(&path, "pa-agent", ExpectedRuntime::OnDemand)
        .expect("initialize");
    assert_eq!(
        store
            .record(Signal::Event(event.clone()))
            .expect("initial event"),
        4
    );
    let pruning = TelemetryStore::open(&path)
        .expect("open")
        .with_retention(RetentionPolicy {
            max_events: 10,
            max_metrics: 10,
            max_age_ms: 1_000,
        });
    assert_eq!(
        pruning
            .record(Signal::Event(event))
            .expect("deduplicate and prune"),
        8
    );
    assert!(
        pruning
            .projection()
            .expect("projection")
            .recent_events
            .is_empty()
    );
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
