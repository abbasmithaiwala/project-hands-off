//! Liveness endpoint.

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

/// Liveness report.
#[derive(Debug, Serialize, ToSchema)]
pub struct Health {
    /// Always `ok` while the server is running.
    pub status: &'static str,
    /// Server version.
    pub version: &'static str,
}

/// Report that the server is up.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "system",
    responses((status = 200, description = "Server is healthy", body = Health))
)]
pub async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
