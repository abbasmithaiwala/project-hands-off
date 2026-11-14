//! Helpers shared by the HTTP integration tests.
#![allow(dead_code, unreachable_pub, clippy::unwrap_used, clippy::expect_used)]

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use hands_off_api::app::router;
use hands_off_api::detection::{DetectorProvider, StaticDetector};
use hands_off_api::jobs::JobContext;
use hands_off_api::state::AppState;
use hands_off_api::storage::{FsBlobStore, SqliteStore};
use hands_off_core::job::JobEvent;
use hands_off_core::{BBox, Config, Error};
use hands_off_imaging::mask::Mask;
use hands_off_llm::mock::MockProvider;
use hands_off_vision::{Detection, Detector};
use http_body_util::BodyExt;
use image::{ImageFormat, RgbImage};
use serde_json::Value;
use tower::ServiceExt;

const BOUNDARY: &str = "X-TEST-BOUNDARY";

pub struct TestApp {
    pub router: Router,
    pub state: AppState,
    pub _dir: tempfile::TempDir,
}

pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("response body is JSON")
    }
}

pub async fn app_with(configure: impl FnOnce(&mut Config)) -> TestApp {
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

pub async fn app() -> TestApp {
    app_with(|_| {}).await
}

pub async fn send(app: &TestApp, request: Request<Body>) -> Response {
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

pub async fn get(app: &TestApp, path: &str) -> Response {
    send(app, Request::get(path).body(Body::empty()).unwrap()).await
}

pub fn png(width: u32, height: u32) -> Vec<u8> {
    encode(width, height, ImageFormat::Png)
}

pub fn encode(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 251) as u8, (y % 241) as u8, 7])
    })
    .write_to(&mut out, format)
    .unwrap();
    out.into_inner()
}

pub fn multipart_request(field: &str, bytes: &[u8]) -> Request<Body> {
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

pub async fn upload(app: &TestApp, bytes: &[u8]) -> Response {
    send(app, multipart_request("file", bytes)).await
}

/// Reports one fixed object and counts how often it was asked to detect.
pub struct FakeDetector {
    pub calls: Arc<AtomicUsize>,
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

pub struct MissingModel;

#[async_trait]
impl DetectorProvider for MissingModel {
    async fn detector(&self, _context: &JobContext) -> hands_off_core::Result<Arc<dyn Detector>> {
        Err(Error::Unavailable(
            "model is not installed: run `just models`".into(),
        ))
    }
}

pub async fn app_with_detector() -> (TestApp, Arc<AtomicUsize>) {
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

pub async fn uploaded_id(app: &TestApp) -> String {
    upload(app, &png(120, 80)).await.json()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

pub async fn analyze(app: &TestApp, id: &str) -> (StatusCode, Vec<JobEvent>) {
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

/// An app whose model provider is a scripted mock, with a fake detector.
pub async fn app_with_ai() -> (TestApp, Arc<MockProvider>) {
    let (mut app, _) = app_with_detector().await;
    let mock = Arc::new(MockProvider::new());
    app.state = app
        .state
        .with_llm(Arc::clone(&mock) as Arc<dyn hands_off_llm::LlmProvider>);
    app.router = router(app.state.clone());
    (app, mock)
}

/// Upload an image and analyse it, returning its id.
pub async fn analysed_image(app: &TestApp) -> String {
    let id = uploaded_id(app).await;
    let (status, events) = analyze(app, &id).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(matches!(events.last(), Some(JobEvent::Completed { .. })));
    id
}

pub async fn post_json(app: &TestApp, path: &str, body: &Value) -> Response {
    send(
        app,
        Request::post(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
}

pub async fn put_json(app: &TestApp, path: &str, body: &Value) -> Response {
    send(
        app,
        Request::put(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
}
