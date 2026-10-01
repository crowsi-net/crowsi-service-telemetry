//! Single source of truth for the owner-local HTTP/SSE projection boundary.

use crate::TelemetryError;
use crate::diagnostic::DIAGNOSTIC_EVENT_SCHEMA;
use crate::projection::PROJECTION_SCHEMA;
pub use crowsi_telemetry_contracts::INGEST_RECEIPT_SCHEMA;
use serde::Serialize;
use std::collections::BTreeSet;

pub const CONTRACT_CHECK_SCHEMA: &str = "crowsi://service-telemetry/contract-check/v1";
pub const HEALTH_PATH: &str = "/health";
pub const PROJECTION_PATH: &str = "/v1/telemetry/projection";
pub const SIGNALS_PATH: &str = "/v1/telemetry/signals";
pub const STREAM_PATH: &str = "/v1/telemetry/stream";
pub const SSE_EVENT: &str = "projection";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteContract {
    method: &'static str,
    path: &'static str,
    authorization: &'static str,
    transport: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractCheckReport {
    schema: &'static str,
    status: &'static str,
    mode: &'static str,
    execution: ExecutionBoundary,
    authentication: AuthenticationBoundary,
    projection_schema: &'static str,
    diagnostic_schema: &'static str,
    ingest_receipt_schema: &'static str,
    sse_event: &'static str,
    routes: Vec<RouteContract>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ActivityState {
    None,
    NotAccessed,
    NotResolved,
    NotStarted,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionBoundary {
    runtime: ActivityState,
    network: ActivityState,
    file_writes: ActivityState,
    secret_resolution: ActivityState,
    external_actions: ActivityState,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum LocalityPolicy {
    Ipv4LoopbackRequired,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum TokenPolicy {
    ReadIngestSeparated,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticationBoundary {
    locality: LocalityPolicy,
    token_policy: TokenPolicy,
}

/// Validates the finite interface declaration without starting a listener or opening state.
///
/// # Errors
/// Returns an error if route identities collide or read and ingest authority are not separated.
pub fn contract_check() -> Result<ContractCheckReport, TelemetryError> {
    let routes = vec![
        route("GET", HEALTH_PATH, "none", "json"),
        route("GET", PROJECTION_PATH, "read-token", "json"),
        route("GET", STREAM_PATH, "read-token", "sse"),
        route("POST", SIGNALS_PATH, "ingest-token", "json"),
    ];
    validate_routes(&routes)?;
    Ok(ContractCheckReport {
        schema: CONTRACT_CHECK_SCHEMA,
        status: "ok",
        mode: "validate",
        execution: ExecutionBoundary {
            runtime: ActivityState::NotStarted,
            network: ActivityState::NotAccessed,
            file_writes: ActivityState::None,
            secret_resolution: ActivityState::NotResolved,
            external_actions: ActivityState::None,
        },
        authentication: AuthenticationBoundary {
            locality: LocalityPolicy::Ipv4LoopbackRequired,
            token_policy: TokenPolicy::ReadIngestSeparated,
        },
        projection_schema: PROJECTION_SCHEMA,
        diagnostic_schema: DIAGNOSTIC_EVENT_SCHEMA,
        ingest_receipt_schema: INGEST_RECEIPT_SCHEMA,
        sse_event: SSE_EVENT,
        routes,
    })
}

fn route(
    method: &'static str,
    path: &'static str,
    authorization: &'static str,
    transport: &'static str,
) -> RouteContract {
    RouteContract {
        method,
        path,
        authorization,
        transport,
    }
}

fn validate_routes(routes: &[RouteContract]) -> Result<(), TelemetryError> {
    let unique = routes
        .iter()
        .map(|route| (route.method, route.path))
        .collect::<BTreeSet<_>>();
    let split_auth = routes
        .iter()
        .any(|route| route.path == STREAM_PATH && route.authorization == "read-token")
        && routes
            .iter()
            .any(|route| route.path == SIGNALS_PATH && route.authorization == "ingest-token");
    if unique.len() != routes.len() || !split_auth {
        return Err(TelemetryError::InvalidInput(
            "telemetry interface contract is internally inconsistent".into(),
        ));
    }
    Ok(())
}
