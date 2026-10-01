use crate::error::Result;
use crate::freshness::freshness;
use crate::projection::{EventView, MetricView, ServiceProjection};
use crate::{
    FreshnessState, HealthDimensions, HealthObservationDimensions, OperationState, ServiceState,
    VerificationState,
};
use rusqlite::{Connection, OptionalExtension};

pub fn read_services(connection: &Connection, now_ms: i64) -> Result<Vec<ServiceProjection>> {
    let service_id: String = connection.query_row(
        "SELECT value FROM metadata WHERE key='service_id'",
        [],
        |row| row.get(0),
    )?;
    let runtime: String = connection.query_row(
        "SELECT value FROM metadata WHERE key='expected_runtime'",
        [],
        |row| row.get(0),
    )?;
    let row = connection.query_row(
        "SELECT observed_at_unix_ms,valid_for_ms,dimensions_json,attributes_json FROM service_health WHERE service_id=?1",
        [&service_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
    ).optional()?;
    let (observed, valid_for, reported, attributes) = match row {
        Some((at, valid, dimensions, attrs)) => (
            at,
            valid,
            Some(serde_json::from_str::<HealthObservationDimensions>(
                &dimensions,
            )?),
            serde_json::from_str(&attrs)?,
        ),
        None => (0, 0, None, Vec::new()),
    };
    let valid_until = observed.saturating_add(valid_for);
    let freshness = freshness(now_ms, observed, valid_for);
    let dimensions = reported.map_or_else(unknown_dimensions, |value| HealthDimensions {
        liveness: value.liveness,
        readiness: value.readiness,
        dependency: value.dependency,
        telemetry_freshness: freshness,
        verification: value.verification,
        operation_result: value.operation_result,
    });
    Ok(vec![ServiceProjection {
        service_id,
        observed_at_unix_ms: observed,
        valid_until_unix_ms: valid_until,
        expected_runtime: serde_json::from_str(&runtime)?,
        dimensions,
        attributes,
    }])
}

fn unknown_dimensions() -> HealthDimensions {
    HealthDimensions {
        liveness: ServiceState::Unknown,
        readiness: ServiceState::Unknown,
        dependency: ServiceState::Unknown,
        telemetry_freshness: FreshnessState::Unknown,
        verification: VerificationState::Unverified,
        operation_result: OperationState::NotApplicable,
    }
}

pub fn read_events(connection: &Connection, limit: u32) -> Result<Vec<EventView>> {
    let mut statement = connection.prepare("SELECT sequence,event_id,service_id,occurred_at_unix_ms,severity,name,outcome,reason_code,attributes_json FROM diagnostic_events ORDER BY sequence DESC LIMIT ?1")?;
    let rows = statement.query_map([limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, String>(8)?,
        ))
    })?;
    rows.map(|row| {
        let (
            sequence,
            event_id,
            service_id,
            occurred_at_unix_ms,
            severity,
            name,
            outcome,
            reason_code,
            attributes,
        ) = row?;
        Ok(EventView {
            sequence,
            event_id,
            service_id,
            occurred_at_unix_ms,
            severity: serde_json::from_str(&severity)?,
            name,
            outcome: serde_json::from_str(&outcome)?,
            reason_code,
            attributes: serde_json::from_str(&attributes)?,
        })
    })
    .collect()
}

pub fn read_metrics(connection: &Connection, limit: u32) -> Result<Vec<MetricView>> {
    let mut statement = connection.prepare("SELECT sequence,service_id,observed_at_unix_ms,name,value,unit,attributes_json FROM metric_samples ORDER BY sequence DESC LIMIT ?1")?;
    let rows = statement.query_map([limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, f64>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;
    rows.map(|row| {
        let (sequence, service_id, observed_at_unix_ms, name, value, unit, attributes) = row?;
        Ok(MetricView {
            sequence,
            service_id,
            observed_at_unix_ms,
            name,
            value,
            unit,
            attributes: serde_json::from_str(&attributes)?,
        })
    })
    .collect()
}
