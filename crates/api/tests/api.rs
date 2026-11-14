//! End-to-end tests of the HTTP API through the real router, with in-memory storage.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use hands_off_api::app::router;
use hands_off_api::detection::{DetectorProvider, StaticDetector};
use hands_off_api::jobs::JobContext;
use hands_off_api::state::AppState;
use hands_off_api::storage::{FsBlobStore, SqliteStore};
use hands_off_core::job::JobEvent;
use hands_off_core::{BBox, Config, Error};
use hands_off_imaging::mask::Mask;
use hands_off_vision::{Detection, Detector};
use http_body_util::BodyExt;
use image::{ImageFormat, RgbImage};
use serde_json::Value;
use tower::ServiceExt;

const BOUNDARY: &str = "X-TEST-BOUNDARY";

struct TestApp {
    router: Router,
    state: AppState,
    _dir: tempfile::TempDir,
}

struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("response body is JSON")
    }
}

async fn app_with(configure: impl FnOnce(&mut Config)) -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config::from_lookup(|_| None).unwrap();
    configure(&mut config);
    let blobs = FsBlobStore::new(dir.path()).await.unwrap();
    let store = SqliteStore::in_memory().await.unwrap();
    let state = AppState::new(config, Arc::new(blobs), Arc::new(store));
    TestApp {
        router: router(state.clone()),
        state,
        _dir: dir,
    }
}

async fn app() -> TestApp {
    app_with(|_| {}).await
}

async fn send(app: &TestApp, request: Request<Body>) -> Response {
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    Response {
        status,
        headers,
        body,
    }
}

async fn get(app: &TestApp, path: &str) -> Response {
    send(app, Request::get(path).body(Body::empty()).unwrap()).await
}

fn png(width: u32, height: u32) -> Vec<u8> {
    encode(width, height, ImageFormat::Png)
}

fn encode(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 251) as u8, (y % 241) as u8, 7])
    })
    .write_to(&mut out, format)
    .unwrap();
    out.into_inner()
}

fn multipart_request(field: &str, bytes: &[u8]) -> Request<Body> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"x\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    Request::post("/api/v1/images")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

async fn upload(app: &TestApp, bytes: &[u8]) -> Response {
    send(app, multipart_request("file", bytes)).await
}

#[tokio::test]
async fn health_reports_ok_and_sets_a_request_id() {
    let app = app().await;
    let response = get(&app, "/api/v1/health").await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["status"], "ok");
    assert!(response.headers.contains_key("x-request-id"));
}

#[tokio::test]
async fn uploaded_image_can_be_fetched_back() {
    let app = app().await;
    let original = png(64, 48);

    let created = upload(&app, &original).await;
    assert_eq!(created.status, StatusCode::CREATED);
    let record = created.json();
    assert_eq!(record["width"], 64);
    assert_eq!(record["height"], 48);
    assert_eq!(record["format"], "png");
    assert_eq!(record["size_bytes"], original.len());
    let id = record["id"].as_str().unwrap();

    let fetched = get(&app, &format!("/api/v1/images/{id}")).await;
    assert_eq!(fetched.status, StatusCode::OK);
    assert_eq!(fetched.json()["hash"], record["hash"]);

    let bytes = get(&app, &format!("/api/v1/images/{id}/original")).await;
    assert_eq!(bytes.status, StatusCode::OK);
    assert_eq!(bytes.headers[header::CONTENT_TYPE], "image/png");
    assert_eq!(bytes.body, original);

    let thumbnail = get(&app, &format!("/api/v1/images/{id}/thumbnail")).await;
    assert_eq!(thumbnail.status, StatusCode::OK);
    assert_eq!(thumbnail.headers[header::CONTENT_TYPE], "image/jpeg");
    assert!(image::load_from_memory(&thumbnail.body).is_ok());
}

#[tokio::test]
async fn jpeg_uploads_report_their_format() {
    let app = app().await;
    let response = upload(&app, &encode(20, 20, ImageFormat::Jpeg)).await;
    assert_eq!(response.status, StatusCode::CREATED);
    assert_eq!(response.json()["format"], "jpeg");
}

#[tokio::test]
async fn identical_uploads_share_one_record() {
    let app = app().await;
    let bytes = png(32, 32);
    let first = upload(&app, &bytes).await;
    let second = upload(&app, &bytes).await;
    assert_eq!(first.status, StatusCode::CREATED);
    assert_eq!(second.status, StatusCode::OK);
    assert_eq!(first.json()["id"], second.json()["id"]);
}

#[tokio::test]
async fn non_image_uploads_are_rejected_with_415() {
    let app = app().await;
    let response = upload(&app, b"this is not an image").await;
    assert_eq!(response.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(response.json()["code"], "unsupported_media");
}

#[tokio::test]
async fn files_over_the_size_limit_are_rejected_with_413() {
    let app = app_with(|config| config.limits.max_upload_bytes = 2_000).await;
    let response = upload(&app, &png(300, 300)).await;
    assert_eq!(response.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(response.json()["code"], "payload_too_large");
}

#[tokio::test]
async fn bodies_far_over_the_limit_are_cut_off_with_413() {
    let app = app_with(|config| config.limits.max_upload_bytes = 1_000).await;
    let response = upload(&app, &vec![0_u8; 300_000]).await;
    assert_eq!(response.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn images_over_the_dimension_limit_are_rejected_with_413() {
    let app = app_with(|config| config.limits.max_dimension = 32).await;
    let response = upload(&app, &png(64, 8)).await;
    assert_eq!(response.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn missing_file_field_is_a_validation_error() {
    let app = app().await;
    let response = send(&app, multipart_request("not_file", &png(8, 8))).await;
    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["code"], "validation");
}

#[tokio::test]
async fn unknown_and_malformed_image_ids_are_client_errors() {
    let app = app().await;
    let missing = get(
        &app,
        &format!("/api/v1/images/{}", hands_off_core::ImageId::new()),
    )
    .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.json()["code"], "not_found");

    let malformed = get(&app, "/api/v1/images/not-a-uuid").await;
    assert_eq!(malformed.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn job_events_stream_as_server_sent_events_until_completion() {
    let app = app().await;
    let id = app
        .state
        .jobs
        .spawn("demo", |ctx| async move {
            ctx.progress("analysing", "looking around");
            ctx.log("found 2 objects");
            Ok(None)
        })
        .await
        .unwrap();

    let response = get(&app, &format!("/api/v1/jobs/{id}/events")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.headers[header::CONTENT_TYPE], "text/event-stream");
    let text = String::from_utf8(response.body).unwrap();
    for expected in [
        "event: progress",
        "event: log",
        "event: completed",
        "found 2 objects",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }

    let record = get(&app, &format!("/api/v1/jobs/{id}")).await.json();
    assert_eq!(record["status"], "completed");
}

#[tokio::test]
async fn failed_jobs_stream_a_failed_event() {
    let app = app().await;
    let id = app
        .state
        .jobs
        .spawn("demo", |_| async {
            Err(hands_off_core::Error::Validation("bad input".into()))
        })
        .await
        .unwrap();
    let text =
        String::from_utf8(get(&app, &format!("/api/v1/jobs/{id}/events")).await.body).unwrap();
    assert!(text.contains("event: failed"));
    assert!(text.contains("bad input"));
}

#[tokio::test]
async fn deleting_a_running_job_cancels_it() {
    let app = app().await;
    let id = app
        .state
        .jobs
        .spawn("demo", |ctx| async move {
            ctx.cancellation().cancelled().await;
            Ok(None)
        })
        .await
        .unwrap();

    let response = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/jobs/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status, StatusCode::ACCEPTED);

    let events: Vec<JobEvent> =
        futures::StreamExt::collect(app.state.jobs.events(id).await.unwrap()).await;
    assert_eq!(events.last(), Some(&JobEvent::Cancelled));
    let record = get(&app, &format!("/api/v1/jobs/{id}")).await.json();
    assert_eq!(record["status"], "cancelled");
}

#[tokio::test]
async fn unknown_jobs_return_404() {
    let app = app().await;
    let id = hands_off_core::JobId::new();
    assert_eq!(
        get(&app, &format!("/api/v1/jobs/{id}")).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&app, &format!("/api/v1/jobs/{id}/events")).await.status,
        StatusCode::NOT_FOUND
    );
    let cancel = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/jobs/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(cancel.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cors_allows_the_configured_origin_only() {
    let app = app().await;
    let preflight = |origin: &str| {
        Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/v1/images")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .body(Body::empty())
            .unwrap()
    };
    let allowed = send(&app, preflight("http://localhost:5173")).await;
    assert_eq!(
        allowed.headers[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "http://localhost:5173"
    );
    let denied = send(&app, preflight("https://evil.example")).await;
    assert!(
        !denied
            .headers
            .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
    );
}

#[tokio::test]
async fn jobs_are_not_leaked_between_apps() {
    let first = app().await;
    let second = app().await;
    let id = first
        .state
        .jobs
        .spawn("demo", |_| async { Ok(None) })
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        get(&second, &format!("/api/v1/jobs/{id}")).await.status,
        StatusCode::NOT_FOUND
    );
}

/// Reports one fixed object and counts how often it was asked to detect.
struct FakeDetector {
    calls: Arc<AtomicUsize>,
}

impl Detector for FakeDetector {
    fn detect(&self, image: &image::DynamicImage) -> hands_off_core::Result<Vec<Detection>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let (w, h) = (image.width().min(64), image.height().min(64));
        let data = (0..h)
            .flat_map(|y| {
                (0..w).map(move |x| x >= w / 4 && x < w * 3 / 4 && y >= h / 4 && y < h * 3 / 4)
            })
            .collect();
        Ok(vec![Detection {
            class_id: 63,
            label: "couch",
            score: 0.93,
            bbox: BBox::new(0.25, 0.25, 0.5, 0.5).unwrap(),
            mask: Mask::new(w, h, data).unwrap(),
        }])
    }
}

struct MissingModel;

#[async_trait]
impl DetectorProvider for MissingModel {
    async fn detector(&self, _context: &JobContext) -> hands_off_core::Result<Arc<dyn Detector>> {
        Err(Error::Unavailable(
            "model is not installed: run `just models`".into(),
        ))
    }
}

async fn app_with_detector() -> (TestApp, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut app = app().await;
    app.state = app
        .state
        .with_detectors(Arc::new(StaticDetector(Arc::new(FakeDetector {
            calls: Arc::clone(&calls),
        }))));
    app.router = router(app.state.clone());
    (app, calls)
}

async fn uploaded_id(app: &TestApp) -> String {
    upload(app, &png(120, 80)).await.json()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn analyze(app: &TestApp, id: &str) -> (StatusCode, Vec<JobEvent>) {
    let response = send(
        app,
        Request::post(format!("/api/v1/images/{id}/analyze"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    if response.status != StatusCode::ACCEPTED {
        return (response.status, Vec::new());
    }
    let job: hands_off_core::JobId = response.json()["job_id"].as_str().unwrap().parse().unwrap();
    let events = futures::StreamExt::collect(app.state.jobs.events(job).await.unwrap()).await;
    (response.status, events)
}

#[tokio::test]
async fn analysis_streams_objects_then_serves_them_and_their_masks() {
    let (app, _) = app_with_detector().await;
    let id = uploaded_id(&app).await;

    let (status, events) = analyze(&app, &id).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let found: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, JobEvent::ObjectFound { .. }))
        .collect();
    assert_eq!(found.len(), 1);
    assert!(matches!(events.last(), Some(JobEvent::Completed { .. })));

    let objects = get(&app, &format!("/api/v1/images/{id}/objects"))
        .await
        .json();
    assert_eq!(objects[0]["label"], "couch");
    assert_eq!(objects[0]["id"], 0);
    assert!(objects[0]["outline"].as_array().unwrap().len() >= 3);

    let mask = get(&app, &format!("/api/v1/images/{id}/objects/0/mask")).await;
    assert_eq!(mask.status, StatusCode::OK);
    assert_eq!(mask.headers[header::CONTENT_TYPE], "image/png");
    assert!(Mask::from_png(&mask.body).unwrap().area() > 0);
}

#[tokio::test]
async fn analysing_twice_uses_the_cache() {
    let (app, calls) = app_with_detector().await;
    let id = uploaded_id(&app).await;

    analyze(&app, &id).await;
    let (_, second) = analyze(&app, &id).await;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        second
            .iter()
            .any(|e| matches!(e, JobEvent::ObjectFound { .. }))
    );
    assert!(
        second
            .iter()
            .any(|e| matches!(e, JobEvent::Progress { stage, .. } if stage == "cached"))
    );
}

#[tokio::test]
async fn a_missing_model_fails_the_job_with_an_actionable_message() {
    let mut app = app().await;
    app.state = app.state.with_detectors(Arc::new(MissingModel));
    app.router = router(app.state.clone());
    let id = uploaded_id(&app).await;

    let (_, events) = analyze(&app, &id).await;

    match events.last() {
        Some(JobEvent::Failed { code, message }) => {
            assert_eq!(*code, hands_off_core::ErrorCode::Unavailable);
            assert!(message.contains("just models"));
        }
        other => panic!("expected a failed event, got {other:?}"),
    }
}

#[tokio::test]
async fn analysis_endpoints_reject_unknown_images_and_objects() {
    let (app, _) = app_with_detector().await;
    let missing = hands_off_core::ImageId::new();
    assert_eq!(
        analyze(&app, &missing.to_string()).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&app, &format!("/api/v1/images/{missing}/objects"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    let id = uploaded_id(&app).await;
    let before = get(&app, &format!("/api/v1/images/{id}/objects")).await;
    assert_eq!(before.status, StatusCode::NOT_FOUND);
    assert_eq!(before.json()["code"], "not_found");

    analyze(&app, &id).await;
    assert_eq!(
        get(&app, &format!("/api/v1/images/{id}/objects/9/mask"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn unavailable_errors_map_to_503() {
    let response = hands_off_api::error::ApiError(Error::Unavailable("x".into()));
    assert_eq!(
        axum::response::IntoResponse::into_response(response).status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}
