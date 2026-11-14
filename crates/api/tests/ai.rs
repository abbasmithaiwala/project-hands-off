//! Settings, property suggestions and widget planning through the HTTP API, with a scripted
//! model provider so no network or credits are needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use hands_off_api::app::router;
use hands_off_api::detection::StaticDetector;
use hands_off_core::Config;
use serde_json::{Value, json};

const SUGGESTIONS: &str = r#"{"properties":[
    {"key":"length","label":"Couch length","widget":"slider"},
    {"key":"shape","label":"Seat shape","widget":"control_points"},
    {"key":"arms","label":"Arm position","widget":"multi_handle"}]}"#;

fn slider(start_x: f32) -> String {
    json!({ "kind": "slider", "axis": { "start": {"x": start_x, "y": 0.5}, "end": {"x": 0.9, "y": 0.5} },
            "min": 0.5, "max": 1.5, "initial": 1.0, "points": null, "handles": null })
    .to_string()
}

/// Six well-spread points around the edge of the crop, all outside the object.
fn control_points_outside_the_object() -> String {
    let corners = [
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
        (0.5, 0.0),
        (0.5, 1.0),
    ];
    let points: Vec<Value> = corners
        .iter()
        .map(|(x, y)| json!({"x": x, "y": y}))
        .collect();
    json!({ "kind": "control_points", "axis": null, "min": null, "max": null, "initial": null, "points": points, "handles": null })
        .to_string()
}

fn properties_path(id: &str) -> String {
    format!("/api/v1/images/{id}/objects/0/properties")
}

fn widget_path(id: &str) -> String {
    format!("/api/v1/images/{id}/objects/0/widget")
}

#[tokio::test]
async fn settings_start_empty_and_never_reveal_the_key() {
    let app = app().await;
    let initial = get(&app, "/api/v1/settings").await.json();
    assert_eq!(
        initial["openrouter"],
        json!({ "configured": false, "origin": "none" })
    );
    assert_eq!(
        initial["models"]["generator"],
        "google/gemini-3.1-flash-lite-image"
    );

    let saved = put_json(
        &app,
        "/api/v1/settings/openrouter-key",
        &json!({ "key": "sk-or-v1-supersecret" }),
    )
    .await;
    assert_eq!(saved.status, StatusCode::NO_CONTENT);

    let after = get(&app, "/api/v1/settings").await;
    assert_eq!(
        after.json()["openrouter"],
        json!({ "configured": true, "origin": "stored" })
    );
    assert!(!String::from_utf8_lossy(&after.body).contains("supersecret"));

    let removed = send(
        &app,
        Request::delete("/api/v1/settings/openrouter-key")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    assert_eq!(
        get(&app, "/api/v1/settings").await.json()["openrouter"]["configured"],
        false
    );
}

#[tokio::test]
async fn implausible_keys_and_model_ids_are_rejected() {
    let app = app().await;
    let key = put_json(
        &app,
        "/api/v1/settings/openrouter-key",
        &json!({ "key": "nope" }),
    )
    .await;
    assert_eq!(key.status, StatusCode::BAD_REQUEST);
    assert_eq!(key.json()["code"], "validation");

    let mut models = get(&app, "/api/v1/settings").await.json()["models"].clone();
    models["suggester"] = json!("not a valid id!");
    let rejected = put_json(&app, "/api/v1/settings/models", &models).await;
    assert_eq!(rejected.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chosen_models_are_saved_and_used_for_requests() {
    let (app, mock) = app_with_ai().await;
    let mut models = get(&app, "/api/v1/settings").await.json()["models"].clone();
    models["suggester"] = json!("typesafe/jev-router");
    let saved = put_json(&app, "/api/v1/settings/models", &models).await;
    assert_eq!(saved.json()["models"]["suggester"], "typesafe/jev-router");

    mock.push_ok(SUGGESTIONS);
    let id = analysed_image(&app).await;
    assert_eq!(
        post_json(&app, &properties_path(&id), &json!({}))
            .await
            .status,
        StatusCode::OK
    );

    let request = &mock.requests()[0];
    assert_eq!(
        request.models,
        ["typesafe/jev-router", "anthropic/claude-haiku-5.5"]
    );
}

#[tokio::test]
async fn properties_come_from_the_model_with_the_crop_attached_and_are_cached() {
    let (app, mock) = app_with_ai().await;
    mock.push_ok(SUGGESTIONS);
    let id = analysed_image(&app).await;

    let first = post_json(&app, &properties_path(&id), &json!({})).await;
    assert_eq!(first.status, StatusCode::OK);
    let keys: Vec<_> = first.json()["properties"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["key"].clone())
        .collect();
    assert_eq!(keys, [json!("length"), json!("shape"), json!("arms")]);

    let request = &mock.requests()[0];
    assert_eq!(request.schema_name, "property_suggestions");
    assert!(request.user_text.contains("Object: couch"));
    assert_eq!(request.images.len(), 1);
    assert!(image::load_from_memory(&request.images[0].bytes).is_ok());

    let second = post_json(&app, &properties_path(&id), &json!({})).await;
    assert_eq!(second.json(), first.json());
    assert_eq!(
        mock.requests().len(),
        1,
        "the second call is served from the cache"
    );
}

#[tokio::test]
async fn an_unusable_suggestion_is_repaired_once() {
    let (app, mock) = app_with_ai().await;
    mock.push_ok("this is not json");
    mock.push_ok(SUGGESTIONS);
    let id = analysed_image(&app).await;

    let response = post_json(&app, &properties_path(&id), &json!({})).await;
    assert_eq!(response.json()["properties"].as_array().unwrap().len(), 3);

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);
    let repair = requests[1].repair.as_ref().unwrap();
    assert_eq!(repair.previous_answer, "this is not json");
    assert!(repair.problem.contains("schema"));
}

#[tokio::test]
async fn two_unusable_suggestions_fall_back_to_generic_properties() {
    let (app, mock) = app_with_ai().await;
    mock.push_ok("garbage");
    mock.push_ok(r#"{"properties":[]}"#);
    let id = analysed_image(&app).await;

    let response = post_json(&app, &properties_path(&id), &json!({})).await;
    let keys: Vec<_> = response.json()["properties"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["key"].clone())
        .collect();
    assert_eq!(keys, [json!("size"), json!("shape")]);
}

#[tokio::test]
async fn widgets_are_mapped_from_crop_to_image_coordinates_and_cached() {
    let (app, mock) = app_with_ai().await;
    mock.push_ok(SUGGESTIONS);
    mock.push_ok(slider(0.1));
    let id = analysed_image(&app).await;

    let response = post_json(&app, &widget_path(&id), &json!({ "property": "length" })).await;
    assert_eq!(response.status, StatusCode::OK);
    let widget = response.json();
    assert_eq!(widget["kind"], "slider");
    // Object box is 0.25..0.75; with 10% padding the crop spans 0.2..0.8, so 0.1 maps to 0.26.
    let start_x = widget["axis"]["start"]["x"].as_f64().unwrap();
    assert!((start_x - 0.26).abs() < 1e-3, "{start_x}");

    let request = &mock.requests()[1];
    assert_eq!(request.schema_name, "widget_plan");
    assert_eq!(request.images.len(), 2, "photo crop and mask crop");
    assert!(request.user_text.contains("Widget: slider"));

    let again = post_json(&app, &widget_path(&id), &json!({ "property": "length" })).await;
    assert_eq!(again.json(), widget);
    assert_eq!(mock.requests().len(), 2);
}

#[tokio::test]
async fn control_points_placed_off_the_object_are_snapped_onto_it() {
    let (app, mock) = app_with_ai().await;
    mock.push_ok(SUGGESTIONS);
    mock.push_ok(control_points_outside_the_object());
    let id = analysed_image(&app).await;

    let widget = post_json(&app, &widget_path(&id), &json!({ "property": "shape" }))
        .await
        .json();
    for point in widget["points"].as_array().unwrap() {
        let (x, y) = (point["x"].as_f64().unwrap(), point["y"].as_f64().unwrap());
        assert!(
            (0.25..0.75).contains(&x) && (0.25..0.75).contains(&y),
            "({x}, {y}) is not on the object"
        );
    }
}

#[tokio::test]
async fn an_invalid_widget_is_repaired_once_then_replaced_by_a_default() {
    let (app, mock) = app_with_ai().await;
    mock.push_ok(SUGGESTIONS);
    mock.push_ok("not json");
    mock.push_ok(slider(0.1).replace("\"min\":0.5", "\"min\":2.0"));
    let id = analysed_image(&app).await;

    let widget = post_json(&app, &widget_path(&id), &json!({ "property": "length" }))
        .await
        .json();

    assert_eq!(widget["kind"], "slider");
    let start_x = widget["axis"]["start"]["x"].as_f64().unwrap();
    assert!(
        (start_x - 0.25).abs() < 1e-3,
        "default axis starts at the box edge, got {start_x}"
    );
    assert!(mock.requests()[2].repair.is_some());
}

#[tokio::test]
async fn unknown_properties_objects_and_unanalysed_images_are_404() {
    let (app, mock) = app_with_ai().await;
    mock.push_ok(SUGGESTIONS);
    let id = analysed_image(&app).await;

    let property = post_json(
        &app,
        &widget_path(&id),
        &json!({ "property": "nonexistent" }),
    )
    .await;
    assert_eq!(
        (property.status, property.json()["code"].clone()),
        (StatusCode::NOT_FOUND, json!("not_found"))
    );

    let object = post_json(
        &app,
        &format!("/api/v1/images/{id}/objects/7/properties"),
        &json!({}),
    )
    .await;
    assert_eq!(object.status, StatusCode::NOT_FOUND);

    let fresh = upload(&app, &png(90, 70)).await.json()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        post_json(&app, &properties_path(&fresh), &json!({}))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_failed_model_call_is_reported_and_not_cached() {
    let (app, mock) = app_with_ai().await;
    mock.push_err("Your OpenRouter account is out of credits.");
    mock.push_ok(SUGGESTIONS);
    let id = analysed_image(&app).await;

    let failed = post_json(&app, &properties_path(&id), &json!({})).await;
    assert_eq!(failed.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(failed.json()["code"], "unavailable");
    assert!(
        failed.json()["message"]
            .as_str()
            .unwrap()
            .contains("credits")
    );

    let retried = post_json(&app, &properties_path(&id), &json!({})).await;
    assert_eq!(retried.status, StatusCode::OK);
}

#[tokio::test]
async fn without_a_key_the_real_provider_asks_for_one() {
    let (mut app, _) = app_with_detector().await;
    app.router = router(app.state.clone());
    let id = analysed_image(&app).await;

    let response = post_json(&app, &properties_path(&id), &json!({})).await;

    assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        response.json()["message"]
            .as_str()
            .unwrap()
            .contains("API key")
    );
}

#[tokio::test]
async fn demo_mode_works_end_to_end_without_a_key() {
    let mut app = app_with(|config: &mut Config| config.demo_ai = true).await;
    let calls = Arc::new(AtomicUsize::new(0));
    app.state = app
        .state
        .with_detectors(Arc::new(StaticDetector(Arc::new(FakeDetector { calls }))));
    app.router = router(app.state.clone());
    let id = analysed_image(&app).await;

    assert_eq!(
        get(&app, "/api/v1/settings").await.json()["openrouter"]["origin"],
        "demo"
    );
    let properties = post_json(&app, &properties_path(&id), &json!({}))
        .await
        .json();
    let first = properties["properties"][0]["key"]
        .as_str()
        .unwrap()
        .to_owned();

    let widget = post_json(&app, &widget_path(&id), &json!({ "property": first })).await;
    assert_eq!(widget.status, StatusCode::OK);
    assert!(widget.json()["kind"].is_string());
}
