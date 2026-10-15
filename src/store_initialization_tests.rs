use super::*;

#[test]
fn initialization_rolls_back_every_precommit_boundary_and_allows_retry() {
    for failed in [
        Boundary::BeforeSchema,
        Boundary::AfterSchema,
        Boundary::AfterService,
        Boundary::AfterRuntime,
        Boundary::BeforeCommit,
    ] {
        let dir = tempfile::tempdir().expect("temporary test directory");
        let path = dir.path().join("state/telemetry.sqlite3");
        crate::secure_path::prepare_database(&path).expect("private test database");
        let mut connection = Connection::open(&path).expect("test SQLite");
        let result = initialize_checked(
            &mut connection,
            "fixture",
            ExpectedRuntime::Continuous,
            |at, _| {
                if at == failed {
                    Err(TelemetryError::InvalidInput("synthetic stop".into()))
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err(), "{failed:?}");
        let count: i64 = connection.query_row("SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |r| r.get(0)).expect("count schema");
        assert_eq!(count, 0, "{failed:?}");
        assert!(crate::TelemetryStore::open(&path).is_err());
        initialize(&mut connection, "fixture", ExpectedRuntime::Continuous).expect("retry");
        crate::TelemetryStore::open(&path).expect("complete reopen");
    }
}

#[test]
fn failure_during_schema_creation_rolls_back_ddl_and_registration() {
    let mut connection = Connection::open_in_memory().expect("test SQLite");
    assert!(
        initialize_checked(
            &mut connection,
            "fixture",
            ExpectedRuntime::Continuous,
            |at, db| {
                if at == Boundary::BeforeSchema {
                    db.execute_batch("CREATE TABLE metric_samples(injected INTEGER);")?;
                }
                Ok(())
            }
        )
        .is_err()
    );
    assert!(!table_exists(&connection).expect("metadata rollback"));
    initialize(&mut connection, "fixture", ExpectedRuntime::Continuous)
        .expect("retry after DDL rollback");
}

#[test]
fn lost_commit_acknowledgement_preserves_registration_and_retry() {
    let dir = tempfile::tempdir().expect("temporary test directory");
    let path = dir.path().join("state/telemetry.sqlite3");
    crate::secure_path::prepare_database(&path).expect("private test database");
    let mut connection = Connection::open(&path).expect("test SQLite");
    assert!(
        initialize_checked(
            &mut connection,
            "fixture",
            ExpectedRuntime::Continuous,
            |at, _| if at == Boundary::AfterCommit {
                Err(TelemetryError::InvalidInput("lost acknowledgement".into()))
            } else {
                Ok(())
            }
        )
        .is_err()
    );
    crate::TelemetryStore::open(&path).expect("durable state survives lost acknowledgement");
    initialize(&mut connection, "fixture", ExpectedRuntime::Continuous).expect("idempotent retry");
    assert!(initialize(&mut connection, "other", ExpectedRuntime::Continuous).is_err());
    assert!(initialize(&mut connection, "fixture", ExpectedRuntime::OnDemand).is_err());
    assert_service(&connection, "fixture").expect("original owner retained");
}

#[test]
fn partial_schema_or_missing_registration_is_rejected_without_repair() {
    for corruption in [
        "DROP TABLE metric_samples",
        "DELETE FROM metadata WHERE key='service_id'",
        "DELETE FROM metadata WHERE key='expected_runtime'",
        "DELETE FROM metadata WHERE key='revision'",
        "UPDATE metadata SET value='99' WHERE key='schema_version'",
    ] {
        let dir = tempfile::tempdir().expect("temporary test directory");
        let path = dir.path().join("state/telemetry.sqlite3");
        crate::secure_path::prepare_database(&path).expect("private test database");
        let mut connection = Connection::open(&path).expect("test SQLite");
        initialize(&mut connection, "fixture", ExpectedRuntime::Continuous).expect("initial state");
        connection
            .execute_batch(corruption)
            .expect("synthetic partial state");
        let before: i64 = connection
            .query_row("SELECT total_changes()", [], |r| r.get(0))
            .expect("write count");
        assert!(initialize(&mut connection, "fixture", ExpectedRuntime::Continuous).is_err());
        assert!(crate::TelemetryStore::open(&path).is_err());
        let after: i64 = connection
            .query_row("SELECT total_changes()", [], |r| r.get(0))
            .expect("write count");
        assert_eq!(
            before, after,
            "partial existing database must not be overwritten"
        );
    }
    let mut connection = Connection::open_in_memory().expect("test SQLite");
    connection
        .execute_batch("CREATE TABLE metric_samples(partial INTEGER);")
        .expect("partial schema");
    assert!(initialize(&mut connection, "fixture", ExpectedRuntime::Continuous).is_err());
    assert!(!table_exists(&connection).expect("no partial migration fallback"));
}
