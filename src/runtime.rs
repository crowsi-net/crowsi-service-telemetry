use crate::connection_limit::ConnectionLimiter;
use crate::error::Result;
use crate::projection::{PROJECTION_SCHEMA, TelemetryProjection};
use crate::secure_path::read_token;
use crate::{Signal, TelemetryError, TelemetryStore};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;

const MAX_CONCURRENT_CONNECTIONS: usize = 32;
const MAX_SERVICE_DATABASES: usize = 512;

#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub bind: SocketAddr,
    pub service_databases: Vec<(String, PathBuf)>,
    pub read_token_file: PathBuf,
    pub ingest_token_file: PathBuf,
}

#[derive(Clone)]
pub struct TelemetryRuntime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    bind: SocketAddr,
    connections: ConnectionLimiter,
    runtime_instance_id: String,
    stores: HashMap<String, TelemetryStore>,
    read_token: String,
    ingest_token: String,
}

impl TelemetryRuntime {
    /// Loads only explicitly bound, owner-local service stores and split tokens.
    ///
    /// # Errors
    /// Returns an error for unsafe paths, token policy violations, or mismatched stores.
    pub fn load(config: &RuntimeConfig) -> Result<Self> {
        if !matches!(config.bind.ip(), IpAddr::V4(ip) if ip.is_loopback()) {
            return Err(TelemetryError::InvalidInput(
                "runtime must bind an IPv4 loopback address".into(),
            ));
        }
        if config.service_databases.len() > MAX_SERVICE_DATABASES {
            return Err(TelemetryError::InvalidInput(
                "runtime accepts at most 512 service database bindings".into(),
            ));
        }
        let read_token = read_token(&config.read_token_file)?;
        let ingest_token = crate::secure_path::read_token(&config.ingest_token_file)?;
        if read_token == ingest_token {
            return Err(TelemetryError::InvalidInput(
                "read and ingest tokens must differ".into(),
            ));
        }
        let mut stores = HashMap::new();
        for (service_id, path) in &config.service_databases {
            let store = TelemetryStore::open(path)?;
            if store.service_id()? != *service_id {
                return Err(TelemetryError::InvalidInput(
                    "service database binding does not match its registered owner".into(),
                ));
            }
            if stores.insert(service_id.clone(), store).is_some() {
                return Err(TelemetryError::InvalidInput(
                    "duplicate service database binding".into(),
                ));
            }
        }
        if stores.is_empty() {
            return Err(TelemetryError::InvalidInput(
                "at least one service database is required".into(),
            ));
        }
        Ok(Self {
            inner: Arc::new(RuntimeInner {
                bind: config.bind,
                connections: ConnectionLimiter::new(MAX_CONCURRENT_CONNECTIONS),
                runtime_instance_id: crate::runtime_instance::generate()?,
                stores,
                read_token,
                ingest_token,
            }),
        })
    }

    /// Builds a redacted, bounded projection across the registered stores.
    ///
    /// # Errors
    /// Returns an error when a registered store cannot be queried or decoded.
    pub fn projection(&self) -> Result<TelemetryProjection> {
        let mut combined = TelemetryProjection {
            schema: PROJECTION_SCHEMA.into(),
            runtime_instance_id: self.inner.runtime_instance_id.clone(),
            generated_at_unix_ms: now_ms(),
            revision: 0,
            services: Vec::new(),
            recent_events: Vec::new(),
            recent_metrics: Vec::new(),
        };
        for store in self.inner.stores.values() {
            let mut projection = store.projection()?;
            combined.revision = combined.revision.saturating_add(projection.revision);
            combined.services.append(&mut projection.services);
            combined.recent_events.append(&mut projection.recent_events);
            combined
                .recent_metrics
                .append(&mut projection.recent_metrics);
        }
        combined
            .services
            .sort_by(|a, b| a.service_id.cmp(&b.service_id));
        combined
            .recent_events
            .sort_by_key(|event| std::cmp::Reverse(event.occurred_at_unix_ms));
        combined
            .recent_metrics
            .sort_by_key(|metric| std::cmp::Reverse(metric.observed_at_unix_ms));
        combined.recent_events.truncate(100);
        combined.recent_metrics.truncate(100);
        Ok(combined)
    }

    /// Routes one signal to its pre-registered service-owned store.
    ///
    /// # Errors
    /// Returns an error for unknown services, invalid signals, or storage failures.
    pub fn ingest(&self, signal: Signal) -> Result<i64> {
        let service_id = match &signal {
            Signal::Health(v) => &v.service_id,
            Signal::Event(v) => &v.service_id,
            Signal::Metric(v) => &v.service_id,
        };
        let store =
            self.inner.stores.get(service_id).ok_or_else(|| {
                TelemetryError::InvalidInput("serviceId is not registered".into())
            })?;
        store.record(signal)?;
        Ok(self.projection()?.revision)
    }

    /// Serves the finite local API. The caller owns process lifecycle and shutdown.
    ///
    /// # Errors
    /// Returns an error if the listener cannot bind or accept a connection.
    pub fn serve(self) -> Result<()> {
        crate::runtime_server::serve(&self)
    }

    pub(crate) fn handle(&self, stream: &mut TcpStream) -> Result<()> {
        crate::runtime_handler::handle(self, stream)
    }

    pub(crate) fn bind(&self) -> SocketAddr {
        self.inner.bind
    }

    pub(crate) fn connection_permit(&self) -> Option<crate::connection_limit::ConnectionPermit> {
        self.inner.connections.try_acquire()
    }

    pub(crate) fn read_token(&self) -> &str {
        &self.inner.read_token
    }
    pub(crate) fn ingest_token(&self) -> &str {
        &self.inner.ingest_token
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
