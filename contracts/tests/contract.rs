use crowsi_telemetry_contracts::{
    EventSeverity, EventSignal, OperationState, Signal, validate_signal_at,
};

#[test]
fn deterministic_validation_accepts_a_bounded_event() {
    let signal = Signal::Event(EventSignal {
        event_id: "event-1".into(),
        service_id: "hatter".into(),
        occurred_at_unix_ms: 1_000,
        severity: EventSeverity::Info,
        name: "hat.invocation.completed".into(),
        outcome: OperationState::Succeeded,
        reason_code: None,
        attributes: Vec::new(),
    });
    validate_signal_at(&signal, 1_000).expect("bounded event");
}

#[test]
fn deterministic_validation_rejects_future_and_unbounded_identity() {
    let signal = Signal::Event(EventSignal {
        event_id: "x".repeat(129),
        service_id: "hatter".into(),
        occurred_at_unix_ms: 301_001,
        severity: EventSeverity::Warning,
        name: "hat.invocation.rejected".into(),
        outcome: OperationState::Rejected,
        reason_code: Some("unknown-term".into()),
        attributes: Vec::new(),
    });
    assert!(validate_signal_at(&signal, 1_000).is_err());
}
