use crate::error::Result;
use crate::store::RetentionPolicy;
use crate::store_metadata::increment_revision;
use crate::store_write::prune;
use rusqlite::{Connection, params};

/// Performs a write transaction only when age or count bounds are exceeded.
pub fn apply(connection: &mut Connection, policy: RetentionPolicy, now_ms: i64) -> Result<bool> {
    if !required(connection, policy, now_ms)? {
        return Ok(false);
    }
    let transaction = connection.transaction()?;
    let changed = prune(&transaction, policy, now_ms)?;
    if changed {
        increment_revision(&transaction)?;
    }
    transaction.commit()?;
    Ok(changed)
}

fn required(connection: &Connection, policy: RetentionPolicy, now_ms: i64) -> Result<bool> {
    let cutoff = now_ms.saturating_sub(policy.max_age_ms);
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM diagnostic_events WHERE occurred_at_unix_ms < ?1)
          OR EXISTS(SELECT 1 FROM metric_samples WHERE observed_at_unix_ms < ?1)
          OR EXISTS(SELECT 1 FROM diagnostic_events LIMIT 1 OFFSET ?2)
          OR EXISTS(SELECT 1 FROM metric_samples LIMIT 1 OFFSET ?3)",
        params![
            cutoff,
            i64::from(policy.max_events),
            i64::from(policy.max_metrics)
        ],
        |row| row.get(0),
    )?)
}
