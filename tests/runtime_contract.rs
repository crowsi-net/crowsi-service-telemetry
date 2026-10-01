use crowsi_service_telemetry::{
    ExpectedRuntime, HealthObservationDimensions, HealthSignal, OperationState, RuntimeConfig,
    ServiceState, Signal, TelemetryRuntime, TelemetryStore, VerificationState,
    generate_runtime_token,
};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

fn health(id: &str) -> Signal {
    Signal::Health(HealthSignal {
        service_id: id.into(),
        observed_at_unix_ms: i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_millis(),
        )
        .expect("timestamp"),
        valid_for_ms: 15_000,
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
fn aggregates_explicit_service_databases_and_rejects_unknown_services() {
    let directory = tempfile::tempdir().expect("tempdir");
    let first = directory.path().join("state/first.sqlite3");
    let second = directory.path().join("state/second.sqlite3");
    TelemetryStore::initialize(&first, "first", ExpectedRuntime::Continuous)
        .expect("first database");
    TelemetryStore::initialize(&second, "second", ExpectedRuntime::External)
        .expect("second database");
    let read = directory.path().join("state/read.token");
    let ingest = directory.path().join("state/ingest.token");
    generate_runtime_token(&read).expect("read token");
    generate_runtime_token(&ingest).expect("ingest token");
    let config = RuntimeConfig {
        bind: "127.0.0.1:4212".parse::<SocketAddr>().expect("bind"),
        service_databases: vec![("first".into(), first), ("second".into(), second)],
        read_token_file: read,
        ingest_token_file: ingest,
    };
    let runtime = TelemetryRuntime::load(&config).expect("runtime");
    runtime.ingest(health("first")).expect("first health");
    runtime.ingest(health("second")).expect("second health");
    assert_eq!(runtime.projection().expect("projection").services.len(), 2);
    assert!(runtime.ingest(health("unregistered")).is_err());
}

#[test]
fn refuses_a_database_bound_under_the_wrong_service_id() {
    let directory = tempfile::tempdir().expect("tempdir");
    let db = directory.path().join("state/telemetry.sqlite3");
    TelemetryStore::initialize(&db, "actual", ExpectedRuntime::OnDemand).expect("database");
    let read = directory.path().join("state/read.token");
    let ingest = directory.path().join("state/ingest.token");
    generate_runtime_token(&read).expect("read token");
    generate_runtime_token(&ingest).expect("ingest token");
    let config = RuntimeConfig {
        bind: "127.0.0.1:4212".parse().expect("bind"),
        service_databases: vec![("wrong".into(), db)],
        read_token_file: read,
        ingest_token_file: ingest,
    };
    assert!(TelemetryRuntime::load(&config).is_err());
}

#[test]
fn runtime_instance_is_stable_per_load_and_changes_between_loads() {
    let directory = tempfile::tempdir().expect("tempdir");
    let db = directory.path().join("state/telemetry.sqlite3");
    TelemetryStore::initialize(&db, "service", ExpectedRuntime::Continuous).expect("database");
    let read = directory.path().join("state/read.token");
    let ingest = directory.path().join("state/ingest.token");
    generate_runtime_token(&read).expect("read token");
    generate_runtime_token(&ingest).expect("ingest token");
    let config = RuntimeConfig {
        bind: "127.0.0.1:4212".parse().expect("bind"),
        service_databases: vec![("service".into(), db)],
        read_token_file: read,
        ingest_token_file: ingest,
    };
    let first = TelemetryRuntime::load(&config).expect("first runtime");
    let first_id = first
        .projection()
        .expect("first projection")
        .runtime_instance_id;
    assert_eq!(
        first.projection().expect("repeat").runtime_instance_id,
        first_id
    );
    let second_id = TelemetryRuntime::load(&config)
        .expect("second runtime")
        .projection()
        .expect("second projection")
        .runtime_instance_id;
    assert!(is_instance_id(&first_id));
    assert!(is_instance_id(&second_id));
    assert_ne!(first_id, second_id);
}

#[test]
fn refuses_more_than_512_service_database_bindings_before_path_access() {
    let bindings = (0..513)
        .map(|index| {
            (
                format!("service-{index}"),
                std::path::PathBuf::from(format!("/absent/service-{index}.sqlite3")),
            )
        })
        .collect();
    let config = RuntimeConfig {
        bind: "127.0.0.1:4212".parse().expect("bind"),
        service_databases: bindings,
        read_token_file: "/absent/read.token".into(),
        ingest_token_file: "/absent/ingest.token".into(),
    };
    match TelemetryRuntime::load(&config) {
        Err(crowsi_service_telemetry::TelemetryError::InvalidInput(message)) => {
            assert!(message.contains("at most 512"));
        }
        _ => panic!("service binding limit must fail before path access"),
    }
}

fn is_instance_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
