//! Router construction.

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::routing::{delete, get, post};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::routes::{health, images, jobs};
use crate::state::AppState;

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
/// Headroom for multipart framing on top of the configured file size limit.
const MULTIPART_OVERHEAD_BYTES: usize = 64 * 1024;

/// Build the application router with all middleware.
pub fn router(state: AppState) -> Router {
    let upload_limit = state.config.limits.max_upload_bytes + MULTIPART_OVERHEAD_BYTES;
    let cors = cors_layer(&state.config.cors_origins);

    let api = Router::new()
        .route("/health", get(health::health))
        .route(
            "/images",
            post(images::upload).layer(DefaultBodyLimit::max(upload_limit)),
        )
        .route("/images/{id}", get(images::get_image))
        .route("/images/{id}/original", get(images::get_original))
        .route("/images/{id}/thumbnail", get(images::get_thumbnail))
        .route("/images/{id}/analyze", post(images::analyze))
        .route("/images/{id}/objects", get(images::list_objects))
        .route(
            "/images/{id}/objects/{object_id}/mask",
            get(images::get_mask),
        )
        .route("/jobs/{id}", get(jobs::get_job))
        .route("/jobs/{id}", delete(jobs::cancel_job))
        .route("/jobs/{id}/events", get(jobs::job_events));

    Router::new()
        .nest("/api/v1", api)
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                let request_id = request
                    .headers()
                    .get(REQUEST_ID)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("-");
                tracing::info_span!(
                    "request",
                    method = %request.method(),
                    path = %request.uri().path(),
                    request_id,
                )
            }),
        )
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
        .with_state(state)
}

fn cors_layer(origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .expose_headers([REQUEST_ID])
}
