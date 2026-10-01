use crate::{ExpectedRuntime, TelemetryError, error::Result};
use rusqlite::{Connection, OptionalExtension};

pub fn table_exists(connection: &Connection) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='metadata'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn metadata_i64(connection: &Connection, key: &str) -> Result<i64> {
    let value: String =
        connection.query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
            row.get(0)
        })?;
    value
        .parse()
        .map_err(|_| TelemetryError::InvalidInput(format!("metadata {key} is invalid")))
}

pub fn increment_revision(connection: &Connection) -> Result<i64> {
    connection.execute(
        "UPDATE metadata SET value=CAST(value AS INTEGER)+1 WHERE key='revision'",
        [],
    )?;
    metadata_i64(connection, "revision")
}

pub fn assert_service(connection: &Connection, service_id: &str) -> Result<()> {
    let expected: String = connection.query_row(
        "SELECT value FROM metadata WHERE key='service_id'",
        [],
        |row| row.get(0),
    )?;
    if expected != service_id {
        return Err(TelemetryError::InvalidInput(
            "signal serviceId does not own this database".into(),
        ));
    }
    Ok(())
}

pub fn assert_expected_runtime(connection: &Connection, expected: ExpectedRuntime) -> Result<()> {
    let actual: String = connection.query_row(
        "SELECT value FROM metadata WHERE key='expected_runtime'",
        [],
        |row| row.get(0),
    )?;
    if actual != serde_json::to_string(&expected)? {
        return Err(TelemetryError::InvalidInput(
            "expected runtime does not match immutable registration metadata".into(),
        ));
    }
    Ok(())
}
