//! Serving the built web app next to the API.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{StatusCode, header};
use common::*;

fn site() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("index.html"),
        "<!doctype html><title>Hands-Off</title>",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("assets")).unwrap();
    std::fs::write(dir.path().join("assets/app-abc123.js"), "console.log(1)").unwrap();
    std::fs::write(dir.path().join("favicon.svg"), "<svg/>").unwrap();
    dir
}

async fn app_serving(dir: &std::path::Path) -> TestApp {
    let path = dir.to_path_buf();
    app_with(move |config| config.web_dir = Some(path)).await
}

#[tokio::test]
async fn the_root_serves_the_app_and_is_always_revalidated() {
    let dir = site();
    let app = app_serving(dir.path()).await;
    let response = get(&app, "/").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&response.body).contains("Hands-Off"));
    assert_eq!(response.headers[header::CACHE_CONTROL], "no-cache");
}

#[tokio::test]
async fn unknown_pages_get_the_app_so_it_can_route_itself() {
    let dir = site();
    let app = app_serving(dir.path()).await;
    let response = get(&app, "/p/some-project").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&response.body).contains("<title>Hands-Off</title>"));
}

#[tokio::test]
async fn hashed_assets_are_cached_forever() {
    let dir = site();
    let app = app_serving(dir.path()).await;
    let response = get(&app, "/assets/app-abc123.js").await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, b"console.log(1)");
    assert!(
        response.headers[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("immutable")
    );
}

#[tokio::test]
async fn a_missing_asset_is_not_replaced_by_the_app() {
    let dir = site();
    let app = app_serving(dir.path()).await;
    let response = get(&app, "/assets/missing.js").await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_api_still_answers_and_unknown_api_paths_stay_json_errors() {
    let dir = site();
    let app = app_serving(dir.path()).await;
    assert_eq!(get(&app, "/api/v1/health").await.json()["status"], "ok");
    let unknown = get(&app, "/api/v1/nope").await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.json()["code"], "not_found");
}

#[tokio::test]
async fn files_outside_the_directory_are_never_served() {
    let dir = site();
    let secret = dir.path().parent().unwrap().join("hands-off-secret.txt");
    std::fs::write(&secret, "secret").unwrap();
    let app = app_serving(dir.path()).await;
    for path in [
        "/../hands-off-secret.txt",
        "/%2e%2e/hands-off-secret.txt",
        "/assets/..%2f..%2fhands-off-secret.txt",
    ] {
        let response = get(&app, path).await;
        assert!(
            !String::from_utf8_lossy(&response.body).contains("secret"),
            "{path} leaked a file"
        );
    }
    std::fs::remove_file(secret).unwrap();
}

#[tokio::test]
async fn without_a_web_directory_only_the_api_exists() {
    let app = app().await;
    assert_eq!(get(&app, "/").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_directory_without_index_html_is_ignored() {
    let empty = tempfile::tempdir().unwrap();
    let app = app_serving(empty.path()).await;
    assert_eq!(get(&app, "/").await.status, StatusCode::NOT_FOUND);
    assert_eq!(get(&app, "/api/v1/health").await.status, StatusCode::OK);
}
