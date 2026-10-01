use crate::error::Result;
use crate::{EventSignal, HealthSignal, MetricSignal, Signal, sanitize_attributes};
use rusqlite::{Connection, params};

pub fn sanitize_signal(signal: &mut Signal) -> Result<()> {
    let attributes = match signal {
        Signal::Health(value) => &mut value.attributes,
        Signal::Event(value) => &mut value.attributes,
        Signal::Metric(value) => &mut value.attributes,
    };
    let (sanitized, _) = sanitize_attributes(attributes)?;
    *attributes = sanitized;
    Ok(())
}

pub fn write_health(connection: &Connection, value: &HealthSignal) -> Result<bool> {
    let changed = connection.execute(
        "INSERT INTO service_health VALUES(?1,?2,?3,?4,?5) ON CONFLICT(service_id) DO UPDATE SET observed_at_unix_ms=excluded.observed_at_unix_ms,valid_for_ms=excluded.valid_for_ms,dimensions_json=excluded.dimensions_json,attributes_json=excluded.attributes_json WHERE excluded.observed_at_unix_ms > service_health.observed_at_unix_ms",
        params![value.service_id, value.observed_at_unix_ms, value.valid_for_ms, json(&value.dimensions)?, json(&value.attributes)?],
    )?;
    Ok(changed == 1)
}

pub fn write_event(connection: &Connection, value: &EventSignal) -> Result<bool> {
    let changed = connection.execute(
        "INSERT INTO diagnostic_events(event_id,service_id,occurred_at_unix_ms,severity,name,outcome,reason_code,attributes_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(event_id) DO NOTHING",
        params![value.event_id, value.service_id, value.occurred_at_unix_ms, json(&value.severity)?, value.name, json(&value.outcome)?, value.reason_code, json(&value.attributes)?],
    )?;
    Ok(changed == 1)
}

pub fn write_metric(connection: &Connection, value: &MetricSignal) -> Result<bool> {
    connection.execute(
        "INSERT INTO metric_samples(service_id,observed_at_unix_ms,name,value,unit,attributes_json) VALUES(?1,?2,?3,?4,?5,?6)",
        params![value.service_id, value.observed_at_unix_ms, value.name, value.value, value.unit, json(&value.attributes)?],
    )?;
    Ok(true)
}

pub fn prune(connection: &Connection, policy: super::RetentionPolicy, now_ms: i64) -> Result<bool> {
    let cutoff = now_ms.saturating_sub(policy.max_age_ms);
    let mut removed = connection.execute(
        "DELETE FROM diagnostic_events WHERE occurred_at_unix_ms < ?1",
        [cutoff],
    )?;
    removed += connection.execute(
        "DELETE FROM metric_samples WHERE observed_at_unix_ms < ?1",
        [cutoff],
    )?;
    removed += connection.execute(
        "DELETE FROM diagnostic_events WHERE sequence NOT IN (SELECT sequence FROM diagnostic_events ORDER BY sequence DESC LIMIT ?1)",
        [policy.max_events],
    )?;
    removed += connection.execute(
        "DELETE FROM metric_samples WHERE sequence NOT IN (SELECT sequence FROM metric_samples ORDER BY sequence DESC LIMIT ?1)",
        [policy.max_metrics],
    )?;
    Ok(removed > 0)
}

fn json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}
