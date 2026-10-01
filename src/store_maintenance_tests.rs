use crate::{
    EventSeverity, EventSignal, ExpectedRuntime, MetricSignal, OperationState, RetentionPolicy,
    Signal, TelemetryStore,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn projection_removes_expired_rows_and_advances_revision_once() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let store = TelemetryStore::initialize(&path, "service", ExpectedRuntime::OnDemand)
        .expect("store")
        .with_retention(RetentionPolicy {
            max_events: 10,
            max_metrics: 10,
            max_age_ms: 60_000,
        });
    let observed = now_ms();
    store.record(event(observed, 1)).expect("event");
    let before = store.record(metric(observed, 1)).expect("metric");
    assert_eq!(
        store.projection_at(observed + 1).expect("fresh").revision,
        before
    );
    let expired = store.projection_at(observed + 60_001).expect("expired");
    assert!(expired.recent_events.is_empty() && expired.recent_metrics.is_empty());
    assert!(expired.revision > before);
    let repeated = store.projection_at(observed + 60_002).expect("repeated");
    assert_eq!(repeated.revision, expired.revision);
}

#[test]
fn projection_enforces_reduced_count_limits() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("state/telemetry.sqlite3");
    let store =
        TelemetryStore::initialize(&path, "service", ExpectedRuntime::OnDemand).expect("store");
    let observed = now_ms();
    for sequence in 1..=3 {
        store.record(event(observed, sequence)).expect("event");
        store.record(metric(observed, sequence)).expect("metric");
    }
    let before = store.revision().expect("revision");
    let bounded = store.with_retention(RetentionPolicy {
        max_events: 1,
        max_metrics: 1,
        max_age_ms: 60_000,
    });
    let projection = bounded
        .projection_at(observed + 1)
        .expect("bounded projection");
    assert_eq!(projection.recent_events.len(), 1);
    assert_eq!(projection.recent_metrics.len(), 1);
    assert!(projection.revision > before);
}

fn event(observed: i64, sequence: u8) -> Signal {
    Signal::Event(EventSignal {
        event_id: format!("event-{sequence}"),
        service_id: "service".into(),
        occurred_at_unix_ms: observed,
        severity: EventSeverity::Warning,
        name: "maintenance".into(),
        outcome: OperationState::Rejected,
        reason_code: Some("bounded-retention".into()),
        attributes: vec![],
    })
}

fn metric(observed: i64, sequence: u8) -> Signal {
    Signal::Metric(MetricSignal {
        service_id: "service".into(),
        observed_at_unix_ms: observed,
        name: format!("sample-{sequence}"),
        value: f64::from(sequence),
        unit: "count".into(),
        attributes: vec![],
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| {
            i64::try_from(value.as_millis()).unwrap_or(i64::MAX)
        })
}
