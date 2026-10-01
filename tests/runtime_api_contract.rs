use crowsi_service_telemetry::{TelemetryError, TelemetryRuntime};

#[test]
fn serving_cannot_replace_the_bind_validated_during_load() {
    let serve: fn(TelemetryRuntime) -> Result<(), TelemetryError> = TelemetryRuntime::serve;
    let _ = serve;
}
