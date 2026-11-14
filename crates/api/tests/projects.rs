//! Version history, saved projects, rename/delete and cleanup of unaccepted results.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use hands_off_api::cleanup::sweep_jobs;
use serde_json::{Value, json};

const WIDTH: u32 = 120;
const HEIGHT: u32 = 80;

fn request(method: &str, path: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder().method(method).uri(path);
    match body {
        Some(json) => builder
            .header("content-type", "application/json")
            .body(Body::from(json.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

/// Run one edit on `id` and accept it, returning the new image's record.
async fn edit_and_accept(app: &EditApp, id: &str, shade: u8) -> Value {
    app.llm.push_ok(SUMMARY_JSON);
    app.generator
        .push_image(solid_png(WIDTH, HEIGHT, [shade, shade, shade]));
    let (_, json, _) = run_edit(&app.app, id, &slider_edit_body(None)).await;
    let job = json["job_id"].as_str().unwrap();
    let accepted = send(
        &app.app,
        request("POST", &format!("/api/v1/jobs/{job}/accept"), None),
    )
    .await;
    assert_eq!(accepted.status, StatusCode::CREATED);
    accepted.json()
}

async fn analysed_and_edited(app: &EditApp) -> (String, Value) {
    let id = analysed_image(&app.app).await;
    let version = edit_and_accept(app, &id, 250).await;
    (id, version)
}

#[tokio::test]
async fn an_accepted_edit_records_where_it_came_from_and_what_it_did() {
    let app = edit_app().await;
    let (id, version) = analysed_and_edited(&app).await;
    assert_eq!(version["parent_id"], id);
    assert!(
        version["edit"]["summary"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );

    let history = get(
        &app.app,
        &format!("/api/v1/images/{}/history", version["id"].as_str().unwrap()),
    )
    .await;
    let versions = history.json();
    let versions = versions.as_array().unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["id"], id);
    assert!(versions[0]["edit"].is_null());
    assert_eq!(versions[1]["id"], version["id"]);
}

#[tokio::test]
async fn history_of_an_unknown_image_is_404() {
    let app = edit_app().await;
    let missing = hands_off_core::ImageId::new();
    let response = get(&app.app, &format!("/api/v1/images/{missing}/history")).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn projects_list_their_latest_version_once() {
    let app = edit_app().await;
    let (_, version) = analysed_and_edited(&app).await;
    let response = get(&app.app, "/api/v1/images").await;
    let tips = response.json();
    let tips = tips.as_array().unwrap();
    assert_eq!(tips.len(), 1);
    assert_eq!(tips[0]["id"], version["id"]);
}

#[tokio::test]
async fn renaming_names_every_version_and_trims_the_name() {
    let app = edit_app().await;
    let (id, version) = analysed_and_edited(&app).await;
    let version_id = version["id"].as_str().unwrap();
    let renamed = send(
        &app.app,
        request(
            "PATCH",
            &format!("/api/v1/images/{version_id}"),
            Some(json!({ "name": "  Living room  " })),
        ),
    )
    .await;
    assert_eq!(renamed.status, StatusCode::OK);
    assert_eq!(renamed.json()["name"], "Living room");
    let original = get(&app.app, &format!("/api/v1/images/{id}")).await.json();
    assert_eq!(original["name"], "Living room");
}

#[tokio::test]
async fn names_must_be_short_and_printable() {
    let app = edit_app().await;
    let id = analysed_image(&app.app).await;
    for name in ["", "   ", "tab\there", &"x".repeat(81)] {
        let response = send(
            &app.app,
            request(
                "PATCH",
                &format!("/api/v1/images/{id}"),
                Some(json!({ "name": name })),
            ),
        )
        .await;
        assert_eq!(response.status, StatusCode::BAD_REQUEST, "{name:?}");
    }
}

#[tokio::test]
async fn deleting_removes_the_image_its_versions_and_their_files() {
    let app = edit_app().await;
    let (id, version) = analysed_and_edited(&app).await;
    let version_id = version["id"].as_str().unwrap();

    let deleted = send(
        &app.app,
        request("DELETE", &format!("/api/v1/images/{id}"), None),
    )
    .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    for gone in [id.as_str(), version_id] {
        let original = get(&app.app, &format!("/api/v1/images/{gone}/original")).await;
        assert_eq!(original.status, StatusCode::NOT_FOUND, "{gone}");
        assert_eq!(
            get(&app.app, &format!("/api/v1/images/{gone}"))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    let again = send(
        &app.app,
        request("DELETE", &format!("/api/v1/images/{id}"), None),
    )
    .await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deleting_a_later_version_keeps_the_earlier_ones() {
    let app = edit_app().await;
    let (id, version) = analysed_and_edited(&app).await;
    let version_id = version["id"].as_str().unwrap();
    send(
        &app.app,
        request("DELETE", &format!("/api/v1/images/{version_id}"), None),
    )
    .await;
    assert_eq!(
        get(&app.app, &format!("/api/v1/images/{id}")).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn a_discarded_result_is_removed() {
    let app = edit_app().await;
    app.llm.push_ok(SUMMARY_JSON);
    app.generator
        .push_image(solid_png(WIDTH, HEIGHT, [250, 250, 250]));
    let id = analysed_image(&app.app).await;
    let (_, json, _) = run_edit(&app.app, &id, &slider_edit_body(None)).await;
    let result = format!("/api/v1/jobs/{}/result", json["job_id"].as_str().unwrap());

    let discarded = send(&app.app, request("DELETE", &result, None)).await;
    assert_eq!(discarded.status, StatusCode::NO_CONTENT);
    assert_eq!(get(&app.app, &result).await.status, StatusCode::NOT_FOUND);
    let again = send(&app.app, request("DELETE", &result, None)).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sweeping_removes_old_finished_jobs_and_keeps_recent_ones() {
    let app = edit_app().await;
    app.llm.push_ok(SUMMARY_JSON);
    app.generator
        .push_image(solid_png(WIDTH, HEIGHT, [250, 250, 250]));
    let id = analysed_image(&app.app).await;
    let (_, json, _) = run_edit(&app.app, &id, &slider_edit_body(None)).await;
    let result = format!("/api/v1/jobs/{}/result", json["job_id"].as_str().unwrap());

    let kept = sweep_jobs(&app.app.state, Duration::from_secs(3600))
        .await
        .unwrap();
    assert_eq!(kept, 0);
    assert_eq!(get(&app.app, &result).await.status, StatusCode::OK);

    let removed = sweep_jobs(&app.app.state, Duration::ZERO).await.unwrap();
    assert!(removed >= 1);
    assert_eq!(get(&app.app, &result).await.status, StatusCode::NOT_FOUND);
}
