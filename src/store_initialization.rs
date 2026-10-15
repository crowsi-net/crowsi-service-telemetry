//! Schema and owner registration are one durable initialization transaction.
use crate::store_metadata::{assert_expected_runtime, assert_service, metadata_i64, table_exists};
use crate::{ExpectedRuntime, TelemetryError, error::Result};
use rusqlite::{Connection, TransactionBehavior};

const MIGRATION: &str = include_str!("../migrations/0001_initial.sql");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boundary {
    BeforeSchema,
    AfterSchema,
    AfterService,
    AfterRuntime,
    BeforeCommit,
    AfterCommit,
}

pub(crate) fn initialize(
    connection: &mut Connection,
    service: &str,
    runtime: ExpectedRuntime,
) -> Result<()> {
    initialize_checked(connection, service, runtime, |_, _| Ok(()))
}

fn initialize_checked(
    connection: &mut Connection,
    service: &str,
    runtime: ExpectedRuntime,
    checkpoint: impl Fn(Boundary, &Connection) -> Result<()>,
) -> Result<()> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if !table_exists(&tx)? {
        let existing: i64 = tx.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if existing != 0 {
            return Err(TelemetryError::InvalidInput(
                "unrecognized partial telemetry schema".into(),
            ));
        }
        checkpoint(Boundary::BeforeSchema, &tx)?;
        tx.execute_batch(MIGRATION)?;
        checkpoint(Boundary::AfterSchema, &tx)?;
        tx.execute(
            "INSERT INTO metadata(key,value) VALUES('service_id',?1)",
            [service],
        )?;
        checkpoint(Boundary::AfterService, &tx)?;
        tx.execute(
            "INSERT INTO metadata(key,value) VALUES('expected_runtime',?1)",
            [serde_json::to_string(&runtime)?],
        )?;
        checkpoint(Boundary::AfterRuntime, &tx)?;
    }
    validate(&tx)?;
    assert_service(&tx, service)?;
    assert_expected_runtime(&tx, runtime)?;
    checkpoint(Boundary::BeforeCommit, &tx)?;
    tx.commit()?;
    checkpoint(Boundary::AfterCommit, connection)?;
    Ok(())
}

pub(crate) fn validate(connection: &Connection) -> Result<()> {
    if !table_exists(connection)?
        || metadata_i64(connection, "schema_version")? != 1
        || metadata_i64(connection, "revision")? < 0
    {
        return Err(TelemetryError::InvalidInput(
            "telemetry database is not completely initialized".into(),
        ));
    }
    let service: String = connection.query_row(
        "SELECT value FROM metadata WHERE key='service_id'",
        [],
        |r| r.get(0),
    )?;
    crate::validation::validate_service_id(&service)?;
    let runtime: String = connection.query_row(
        "SELECT value FROM metadata WHERE key='expected_runtime'",
        [],
        |r| r.get(0),
    )?;
    let _: ExpectedRuntime = serde_json::from_str(&runtime)?;
    for sql in [
        "SELECT service_id,observed_at_unix_ms,valid_for_ms,dimensions_json,attributes_json FROM service_health LIMIT 0",
        "SELECT sequence,event_id,service_id,occurred_at_unix_ms,severity,name,outcome,reason_code,attributes_json FROM diagnostic_events LIMIT 0",
        "SELECT sequence,service_id,observed_at_unix_ms,name,value,unit,attributes_json FROM metric_samples LIMIT 0",
    ] {
        connection.prepare(sql)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "store_initialization_tests.rs"]
mod tests;
