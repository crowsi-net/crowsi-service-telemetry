use crate::error::Result;
use crate::projection::{PROJECTION_SCHEMA, TelemetryProjection};
use crate::secure_path::prepare_database;
use crate::store_maintenance::apply as apply_retention;
use crate::store_metadata::{
    assert_expected_runtime, assert_service, increment_revision, metadata_i64, table_exists,
};
use crate::store_read::{read_events, read_metrics, read_services};
use crate::store_write::{prune, sanitize_signal, write_event, write_health, write_metric};
use crate::validation::validate_signal;
use crate::{ExpectedRuntime, Signal, TelemetryError};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATION: &str = include_str!("../migrations/0001_initial.sql");

#[derive(Clone, Copy, Debug)]
pub struct RetentionPolicy {
    pub max_events: u32,
    pub max_metrics: u32,
    pub max_age_ms: i64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_events: 1_000,
            max_metrics: 1_000,
            max_age_ms: 30 * 24 * 60 * 60 * 1_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TelemetryStore {
    path: PathBuf,
    runtime_instance_id: String,
    retention: RetentionPolicy,
}

impl TelemetryStore {
    /// Creates a schema and permanently binds it to one service identifier.
    ///
    /// # Errors
    /// Returns an error for invalid identity, unsafe path, or migration failure.
    pub fn initialize(
        path: &Path,
        service_id: &str,
        expected_runtime: ExpectedRuntime,
    ) -> Result<Self> {
        crate::validation::validate_service_id(service_id)?;
        prepare_database(path)?;
        let store = Self {
            path: path.into(),
            runtime_instance_id: crate::runtime_instance::generate()?,
            retention: RetentionPolicy::default(),
        };
        let connection = store.connect()?;
        if !table_exists(&connection)? {
            connection.execute_batch(MIGRATION)?;
            connection.execute(
                "INSERT INTO metadata(key,value) VALUES('service_id',?1)",
                [service_id],
            )?;
            connection.execute(
                "INSERT INTO metadata(key,value) VALUES('expected_runtime',?1)",
                [serde_json::to_string(&expected_runtime)?],
            )?;
        }
        assert_service(&connection, service_id)?;
        assert_expected_runtime(&connection, expected_runtime)?;
        Ok(store)
    }

    /// Opens an initialized owner-local store without performing migration fallback.
    ///
    /// # Errors
    /// Returns an error for an unsafe path, absent schema, or `SQLite` failure.
    pub fn open(path: &Path) -> Result<Self> {
        prepare_database(path)?;
        let store = Self {
            path: path.into(),
            runtime_instance_id: crate::runtime_instance::generate()?,
            retention: RetentionPolicy::default(),
        };
        let connection = store.connect()?;
        if !table_exists(&connection)? {
            return Err(TelemetryError::InvalidInput(
                "telemetry database is not initialized".into(),
            ));
        }
        Ok(store)
    }

    #[must_use]
    pub fn with_retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Validates, minimizes, persists, and prunes one signal atomically.
    ///
    /// # Errors
    /// Returns an error for invalid data, wrong service ownership, or storage failure.
    pub fn record(&self, mut signal: Signal) -> Result<i64> {
        validate_signal(&signal)?;
        sanitize_signal(&mut signal)?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        assert_service(&transaction, signal.service_id())?;
        let changed = match signal {
            Signal::Health(value) => write_health(&transaction, &value)?,
            Signal::Event(value) => write_event(&transaction, &value)?,
            Signal::Metric(value) => write_metric(&transaction, &value)?,
        };
        let pruned = prune(&transaction, self.retention, now_ms())?;
        if changed || pruned {
            increment_revision(&transaction)?;
        }
        transaction.commit()?;
        Ok(self.projection()?.revision)
    }

    /// Reads the bounded projection for this service-owned database.
    ///
    /// # Errors
    /// Returns an error if stored state cannot be queried or decoded.
    pub fn projection(&self) -> Result<TelemetryProjection> {
        self.projection_at(now_ms())
    }

    pub(crate) fn projection_at(&self, projected_at_ms: i64) -> Result<TelemetryProjection> {
        let mut connection = self.connect()?;
        apply_retention(&mut connection, self.retention, projected_at_ms)?;
        let services = read_services(&connection, projected_at_ms)?;
        let phase = services.first().map_or(0, |service| {
            crate::freshness::phase(service.dimensions.telemetry_freshness)
        });
        Ok(TelemetryProjection {
            schema: PROJECTION_SCHEMA.into(),
            runtime_instance_id: self.runtime_instance_id.clone(),
            generated_at_unix_ms: projected_at_ms,
            revision: metadata_i64(&connection, "revision")?
                .saturating_mul(4)
                .saturating_add(phase),
            services,
            recent_events: read_events(&connection, 100)?,
            recent_metrics: read_metrics(&connection, 100)?,
        })
    }

    /// Reads the current monotonic database revision.
    ///
    /// # Errors
    /// Returns an error when revision metadata is absent or invalid.
    pub fn revision(&self) -> Result<i64> {
        Ok(self.projection()?.revision)
    }

    /// Reads the immutable service owner identifier.
    ///
    /// # Errors
    /// Returns an error when ownership metadata cannot be read.
    pub fn service_id(&self) -> Result<String> {
        Ok(self.connect()?.query_row(
            "SELECT value FROM metadata WHERE key='service_id'",
            [],
            |row| row.get(0),
        )?)
    }

    fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=3000;")?;
        Ok(connection)
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
