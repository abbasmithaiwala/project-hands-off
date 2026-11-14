//! Serving the built web app next to the API, so one process (and one container) is enough.

use std::path::Path;

use axum::Router;
use axum::http::{HeaderValue, header};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const REVALIDATE: &str = "no-cache";

/// A router serving the built web app from `dir`.
///
/// Files under `assets/` carry a content hash in their name, so they are cached forever. Every
/// other path answers with `index.html` (the app does its own routing), which is always
/// revalidated so a new release is picked up at once.
///
/// Returns `None`, after logging why, when `dir` has no `index.html`.
#[must_use]
pub fn app(dir: &Path) -> Option<Router> {
    let index = dir.join("index.html");
    if !index.is_file() {
        tracing::warn!(dir = %dir.display(), "HANDS_OFF_WEB_DIR has no index.html; not serving the web app");
        return None;
    }
    let assets = Router::new()
        .nest_service("/assets", ServeDir::new(dir.join("assets")))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static(IMMUTABLE),
        ));
    let pages = Router::new()
        .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static(REVALIDATE),
        ));
    Some(assets.merge(pages))
}
