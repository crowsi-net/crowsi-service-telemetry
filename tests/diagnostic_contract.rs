use crowsi_service_telemetry::{
    DIAGNOSTIC_EVENT_SCHEMA, DiagnosticEventV1, EventSignal, OperationState,
};

#[test]
fn diagnostic_event_maps_without_raw_authenticator_material() {
    let event = DiagnosticEventV1 {
        schema: DIAGNOSTIC_EVENT_SCHEMA.into(),
        event_id: "550e8400-e29b-41d4-a716-446655440000".into(),
        occurred_at_unix_ms: 1,
        component: "crowsi-pa-key-agent".into(),
        operation: "passkey-registration".into(),
        phase: "possession-proof".into(),
        outcome: OperationState::Rejected,
        reason_code: "authenticator-data-flags-rejected".into(),
        contains_sensitive_values: false,
    };
    let signal = EventSignal::try_from(event).expect("mapping");
    assert_eq!(signal.service_id, "crowsi-pa-key-agent");
    assert_eq!(signal.attributes.len(), 1);
}

#[test]
fn rejects_events_that_declare_sensitive_values() {
    let event = DiagnosticEventV1 {
        schema: DIAGNOSTIC_EVENT_SCHEMA.into(),
        event_id: "opaque-event".into(),
        occurred_at_unix_ms: 1,
        component: "crowsi-pa-key-agent".into(),
        operation: "passkey-registration".into(),
        phase: "possession-proof".into(),
        outcome: OperationState::Rejected,
        reason_code: "rejected".into(),
        contains_sensitive_values: true,
    };
    assert!(EventSignal::try_from(event).is_err());
}
