//! The `OpenRouter` chat-completions client, for structured answers and generated images.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hands_off_core::{Error, Result};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};

use crate::provider::{
    ImageGenerator, ImageInput, ImageRequest, ImageResponse, JsonRequest, JsonResponse,
    KeyProvider, LlmProvider, Stage, Usage,
};

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";
const JSON_TIMEOUT: Duration = Duration::from_secs(90);
const IMAGE_TIMEOUT: Duration = Duration::from_secs(150);
const DEFAULT_BACKOFF: Duration = Duration::from_millis(600);
const ATTEMPTS_PER_MODEL: u32 = 2;
const MAX_TOKENS: u32 = 4000;
const SNIPPET_CHARS: usize = 160;
const REFUSAL_WORDS: [&str; 6] = [
    "moderation",
    "flagged",
    "safety",
    "content policy",
    "content_policy",
    "policy violation",
];

/// What to do after a failed call.
enum Failure {
    /// Give up entirely: the key, the account or the content is the problem.
    Fatal(Error),
    /// Try the same model again after a pause.
    Retry(String),
    /// This model cannot serve the request; move to the next one.
    NextModel(String),
}

/// Why a successful HTTP response could not be used.
enum Rejection {
    /// The answer was empty or malformed.
    Unusable(String),
    /// The model answered in words instead of producing what was asked for.
    Words(String),
}

struct Done<T> {
    value: T,
    model: String,
    usage: Usage,
    latency: Duration,
}

/// Calls hosted models through `OpenRouter`, with retries and fallback models.
pub struct OpenRouterProvider {
    client: reqwest::Client,
    base_url: String,
    keys: Arc<dyn KeyProvider>,
    backoff: Duration,
}

impl OpenRouterProvider {
    /// A provider using the production endpoint.
    #[must_use]
    pub fn new(keys: Arc<dyn KeyProvider>) -> Self {
        Self::with_base_url(keys, DEFAULT_BASE_URL)
    }

    /// A provider using a custom endpoint (for tests or self-hosted gateways).
    #[must_use]
    pub fn with_base_url(keys: Arc<dyn KeyProvider>, base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            keys,
            backoff: DEFAULT_BACKOFF,
        }
    }

    /// Pause between retries of the same model (doubles on each retry).
    #[must_use]
    pub fn with_backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    fn json_body(request: &JsonRequest, model: &str) -> Value {
        let mut content = vec![json!({ "type": "text", "text": request.user_text })];
        content.extend(request.images.iter().map(image_part));
        let mut messages = vec![
            json!({ "role": "system", "content": request.system }),
            json!({ "role": "user", "content": content }),
        ];
        if let Some(repair) = &request.repair {
            messages.push(json!({ "role": "assistant", "content": repair.previous_answer }));
            messages.push(json!({
                "role": "user",
                "content": format!(
                    "That answer was rejected: {}. Reply again with corrected JSON only.",
                    repair.problem
                ),
            }));
        }
        json!({
            "model": model,
            "messages": messages,
            "response_format": {
                "type": "json_schema",
                "json_schema": { "name": request.schema_name, "strict": true, "schema": request.schema },
            },
            "provider": { "require_parameters": true },
            "usage": { "include": true },
            "max_tokens": MAX_TOKENS,
        })
    }

    fn image_body(request: &ImageRequest, model: &str) -> Value {
        let mut content = vec![json!({ "type": "text", "text": request.instruction })];
        content.push(image_part(&request.original));
        content.extend(request.preview.iter().map(image_part));
        json!({
            "model": model,
            "messages": [{ "role": "user", "content": content }],
            "modalities": ["image", "text"],
            "usage": { "include": true },
        })
    }

    async fn call<T>(
        &self,
        key: &SecretString,
        model: &str,
        body: &Value,
        timeout: Duration,
        extract: &impl Fn(&Value) -> std::result::Result<T, Rejection>,
    ) -> std::result::Result<Done<T>, Failure> {
        let started = Instant::now();
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(key.expose_secret())
            .timeout(timeout)
            .json(body)
            .send()
            .await
            .map_err(|e| {
                let how = if e.is_timeout() {
                    "timed out"
                } else {
                    "unreachable"
                };
                Failure::Retry(format!("{model}: network error ({how})"))
            })?;

        let status = response.status();
        let payload: Value = response.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(classify(status.as_u16(), model, &payload));
        }
        match extract(&payload) {
            Ok(value) => Ok(Done {
                value,
                model: payload["model"].as_str().unwrap_or(model).to_owned(),
                usage: parse_usage(&payload["usage"]),
                latency: started.elapsed(),
            }),
            Err(Rejection::Unusable(problem)) => {
                Err(Failure::NextModel(format!("{model}: {problem}")))
            }
            Err(Rejection::Words(text)) => Err(Failure::NextModel(format!(
                "{model}: replied with text: {text}"
            ))),
        }
    }

    /// Try each model in turn, retrying transient failures, until one answers usably.
    async fn run<T>(
        &self,
        stage: Stage,
        models: &[String],
        timeout: Duration,
        build: impl Fn(&str) -> Value,
        extract: impl Fn(&Value) -> std::result::Result<T, Rejection>,
    ) -> Result<Done<T>> {
        let key = self.keys.api_key().await?.ok_or_else(|| {
            Error::Unavailable("An OpenRouter API key is required. Add one in Settings.".into())
        })?;

        let mut tried: Vec<&str> = Vec::new();
        let mut last_problem = String::from("no models configured");
        let mut spoke_instead: Option<String> = None;
        for model in models {
            if tried.contains(&model.as_str()) {
                continue;
            }
            tried.push(model);
            let body = build(model);
            for attempt in 0..ATTEMPTS_PER_MODEL {
                match self.call(&key, model, &body, timeout, &extract).await {
                    Ok(done) => {
                        tracing::info!(
                            stage = stage.name(),
                            model = %done.model,
                            latency_ms = done.latency.as_millis(),
                            cost_usd = done.usage.cost_usd,
                            "model call succeeded"
                        );
                        return Ok(done);
                    }
                    Err(Failure::Fatal(error)) => return Err(error),
                    Err(Failure::NextModel(problem)) => {
                        tracing::warn!(stage = stage.name(), %problem, "model skipped");
                        if let Some((_, text)) = problem.split_once("replied with text: ") {
                            spoke_instead = Some(text.to_owned());
                        }
                        last_problem = problem;
                        break;
                    }
                    Err(Failure::Retry(problem)) => {
                        tracing::warn!(stage = stage.name(), %problem, attempt, "model call failed");
                        last_problem = problem;
                        if attempt + 1 < ATTEMPTS_PER_MODEL {
                            tokio::time::sleep(self.backoff * 2_u32.pow(attempt)).await;
                        }
                    }
                }
            }
        }
        if let Some(text) = spoke_instead {
            return Err(Error::Refused(format!(
                "The image model did not produce an image. It said: \u{201c}{text}\u{201d} Try a different edit."
            )));
        }
        Err(Error::Unavailable(format!(
            "The AI models are not responding right now ({last_problem}). Try again, or choose another model in Settings."
        )))
    }
}

fn image_part(image: &ImageInput) -> Value {
    let encoded = STANDARD.encode(&image.bytes);
    json!({
        "type": "image_url",
        "image_url": { "url": format!("data:{};base64,{encoded}", image.mime) },
    })
}

fn snippet(text: &str) -> String {
    text.chars().take(SNIPPET_CHARS).collect()
}

fn classify(status: u16, model: &str, payload: &Value) -> Failure {
    let detail = payload["error"]["message"].as_str().unwrap_or("no details");
    let lowered = detail.to_lowercase();
    let flagged = payload["error"]["metadata"]["reasons"].is_array()
        || REFUSAL_WORDS.iter().any(|word| lowered.contains(word));
    if matches!(status, 400 | 403 | 422) && flagged {
        return Failure::Fatal(Error::Refused(
            "The image model declined this edit on content-policy grounds. Try a different edit."
                .into(),
        ));
    }
    match status {
        401 | 403 => Failure::Fatal(Error::Unavailable(
            "OpenRouter rejected the API key. Check it in Settings.".into(),
        )),
        402 => Failure::Fatal(Error::Unavailable(
            "Your OpenRouter account is out of credits. Add credits at openrouter.ai.".into(),
        )),
        408 | 429 | 500..=599 => {
            Failure::Retry(format!("{model}: HTTP {status} ({})", snippet(detail)))
        }
        _ => Failure::NextModel(format!("{model}: HTTP {status} ({})", snippet(detail))),
    }
}

fn parse_usage(usage: &Value) -> Usage {
    Usage {
        prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
        completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
        cost_usd: usage["cost"].as_f64(),
    }
}

fn text_of(payload: &Value) -> Option<&str> {
    payload["choices"][0]["message"]["content"]
        .as_str()
        .filter(|c| !c.trim().is_empty())
}

/// Decode the first `data:` URL image in a response.
fn first_image(payload: &Value) -> std::result::Result<Vec<u8>, Rejection> {
    let url = payload["choices"][0]["message"]["images"][0]["image_url"]["url"].as_str();
    let Some(url) = url else {
        return Err(match text_of(payload) {
            Some(text) => Rejection::Words(snippet(text)),
            None => Rejection::Unusable("returned no image".into()),
        });
    };
    let encoded = url
        .split_once(";base64,")
        .map(|(_, data)| data)
        .ok_or_else(|| Rejection::Unusable("returned an image that is not base64 data".into()))?;
    STANDARD
        .decode(encoded)
        .map_err(|_| Rejection::Unusable("returned an image that could not be decoded".into()))
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn complete_json(&self, request: JsonRequest) -> Result<JsonResponse> {
        let done = self
            .run(
                request.stage,
                &request.models,
                JSON_TIMEOUT,
                |model| Self::json_body(&request, model),
                |payload| {
                    text_of(payload)
                        .map(str::to_owned)
                        .ok_or_else(|| Rejection::Unusable("returned no content".into()))
                },
            )
            .await?;
        Ok(JsonResponse {
            content: done.value,
            model: done.model,
            usage: done.usage,
            latency: done.latency,
        })
    }
}

#[async_trait]
impl ImageGenerator for OpenRouterProvider {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResponse> {
        let done = self
            .run(
                Stage::Generator,
                &request.models,
                IMAGE_TIMEOUT,
                |model| Self::image_body(&request, model),
                first_image,
            )
            .await?;
        Ok(ImageResponse {
            bytes: done.value,
            model: done.model,
            usage: done.usage,
            latency: done.latency,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use axum::{Json, Router};
    use tokio::net::TcpListener;

    use super::*;
    use crate::provider::{ImageInput, Repair, Stage};

    struct FixedKey(Option<&'static str>);

    #[async_trait]
    impl KeyProvider for FixedKey {
        async fn api_key(&self) -> Result<Option<SecretString>> {
            Ok(self.0.map(|k| SecretString::from(k.to_owned())))
        }
    }

    type Seen = Arc<Mutex<Vec<(Option<String>, Value)>>>;

    /// Scripted upstream: returns the next `(status, body)` per call and records requests.
    #[derive(Clone, Default)]
    struct Upstream {
        script: Arc<Mutex<Vec<(u16, Value)>>>,
        seen: Seen,
        calls: Arc<AtomicUsize>,
    }

    async fn handler(
        State(up): State<Upstream>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> (StatusCode, Json<Value>) {
        up.calls.fetch_add(1, Ordering::SeqCst);
        let auth = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        up.seen.lock().unwrap().push((auth, body));
        let (status, payload) = {
            let mut script = up.script.lock().unwrap();
            if script.is_empty() {
                (500, json!({}))
            } else {
                script.remove(0)
            }
        };
        (StatusCode::from_u16(status).unwrap(), Json(payload))
    }

    async fn upstream(script: Vec<(u16, Value)>) -> (Upstream, String) {
        let up = Upstream {
            script: Arc::new(Mutex::new(script)),
            ..Upstream::default()
        };
        let app = Router::new()
            .route("/chat/completions", post(handler))
            .with_state(up.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (up, url)
    }

    fn ok(content: &str) -> (u16, Value) {
        (
            200,
            json!({
                "model": "served/model",
                "choices": [{ "message": { "content": content } }],
                "usage": { "prompt_tokens": 12, "completion_tokens": 7, "cost": 0.0021 },
            }),
        )
    }

    fn provider(url: &str, key: Option<&'static str>) -> OpenRouterProvider {
        OpenRouterProvider::with_base_url(Arc::new(FixedKey(key)), url)
            .with_backoff(Duration::from_millis(1))
    }

    fn request(models: &[&str]) -> JsonRequest {
        JsonRequest {
            stage: Stage::Suggester,
            models: models.iter().map(|m| (*m).to_owned()).collect(),
            system: "system prompt".into(),
            user_text: "describe".into(),
            images: vec![ImageInput {
                mime: "image/jpeg",
                bytes: vec![1, 2, 3],
            }],
            schema_name: "thing",
            schema: json!({ "type": "object" }),
            repair: None,
        }
    }

    #[tokio::test]
    async fn returns_content_usage_and_sends_a_structured_request() {
        let (up, url) = upstream(vec![ok("{\"a\":1}")]).await;
        let response = provider(&url, Some("sk-secret"))
            .complete_json(request(&["m1"]))
            .await
            .unwrap();

        assert_eq!(response.content, "{\"a\":1}");
        assert_eq!(response.model, "served/model");
        assert_eq!(
            response.usage,
            Usage {
                prompt_tokens: 12,
                completion_tokens: 7,
                cost_usd: Some(0.0021)
            }
        );

        let (auth, body) = up.seen.lock().unwrap()[0].clone();
        assert_eq!(auth.as_deref(), Some("Bearer sk-secret"));
        assert_eq!(body["model"], "m1");
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert_eq!(body["response_format"]["json_schema"]["name"], "thing");
        assert_eq!(body["provider"]["require_parameters"], true);
        let user = &body["messages"][1]["content"];
        assert_eq!(user[0]["text"], "describe");
        assert!(
            user[1]["image_url"]["url"]
                .as_str()
                .unwrap()
                .starts_with("data:image/jpeg;base64,")
        );
    }

    #[tokio::test]
    async fn repair_requests_append_the_rejected_answer_and_the_problem() {
        let (up, url) = upstream(vec![ok("{}")]).await;
        let mut req = request(&["m1"]);
        req.repair = Some(Repair {
            previous_answer: "{\"bad\":true}".into(),
            problem: "missing field x".into(),
        });
        provider(&url, Some("k")).complete_json(req).await.unwrap();

        let messages = up.seen.lock().unwrap()[0].1["messages"].clone();
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[2]["content"], "{\"bad\":true}");
        assert!(
            messages[3]["content"]
                .as_str()
                .unwrap()
                .contains("missing field x")
        );
    }

    #[tokio::test]
    async fn retries_transient_errors_on_the_same_model() {
        let (up, url) = upstream(vec![
            (503, json!({"error": {"message": "overloaded"}})),
            ok("{}"),
        ])
        .await;
        assert!(
            provider(&url, Some("k"))
                .complete_json(request(&["m1"]))
                .await
                .is_ok()
        );
        assert_eq!(up.calls.load(Ordering::SeqCst), 2);
        let seen = up.seen.lock().unwrap();
        assert_eq!(seen[0].1["model"], "m1");
        assert_eq!(seen[1].1["model"], "m1");
    }

    #[tokio::test]
    async fn falls_back_to_the_next_model_when_one_cannot_serve_the_request() {
        let (up, url) = upstream(vec![
            (400, json!({"error": {"message": "no structured outputs"}})),
            ok("{}"),
        ])
        .await;
        provider(&url, Some("k"))
            .complete_json(request(&["bad", "good"]))
            .await
            .unwrap();
        let seen = up.seen.lock().unwrap();
        assert_eq!(
            (seen[0].1["model"].as_str(), seen[1].1["model"].as_str()),
            (Some("bad"), Some("good"))
        );
    }

    #[tokio::test]
    async fn skips_duplicate_models() {
        let (up, url) = upstream(vec![(400, json!({})), (400, json!({}))]).await;
        let _ = provider(&url, Some("k"))
            .complete_json(request(&["same", "same"]))
            .await;
        assert_eq!(up.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn rejected_keys_and_empty_accounts_fail_immediately_with_clear_messages() {
        for (status, expected) in [
            (401, "rejected the API key"),
            (403, "rejected the API key"),
            (402, "out of credits"),
        ] {
            let (up, url) = upstream(vec![(status, json!({}))]).await;
            let error = provider(&url, Some("k"))
                .complete_json(request(&["a", "b"]))
                .await
                .unwrap_err();
            assert!(
                matches!(&error, Error::Unavailable(m) if m.contains(expected)),
                "{error}"
            );
            assert_eq!(
                up.calls.load(Ordering::SeqCst),
                1,
                "no fallback after {status}"
            );
        }
    }

    #[tokio::test]
    async fn a_missing_key_is_reported_without_calling_the_network() {
        let (up, url) = upstream(vec![ok("{}")]).await;
        let error = provider(&url, None)
            .complete_json(request(&["m1"]))
            .await
            .unwrap_err();
        assert!(matches!(&error, Error::Unavailable(m) if m.contains("API key")));
        assert_eq!(up.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn exhausting_every_model_reports_the_last_problem() {
        let (_, url) = upstream(vec![
            (500, json!({"error": {"message": "boom"}})),
            (500, json!({"error": {"message": "boom"}})),
        ])
        .await;
        let error = provider(&url, Some("k"))
            .complete_json(request(&["only"]))
            .await
            .unwrap_err();
        assert!(
            matches!(&error, Error::Unavailable(m) if m.contains("not responding") && m.contains("boom")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn empty_content_moves_on_to_the_next_model() {
        let (_, url) = upstream(vec![
            (200, json!({"choices": [{"message": {"content": ""}}]})),
            ok("{\"ok\":true}"),
        ])
        .await;
        let response = provider(&url, Some("k"))
            .complete_json(request(&["blank", "good"]))
            .await
            .unwrap();
        assert_eq!(response.content, "{\"ok\":true}");
    }

    #[tokio::test]
    async fn an_unreachable_endpoint_is_reported_as_unavailable_not_a_panic() {
        let error = provider("http://127.0.0.1:1", Some("k"))
            .complete_json(request(&["m"]))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Unavailable(_)));
    }

    #[tokio::test]
    async fn the_api_key_never_appears_in_errors() {
        let (_, url) = upstream(vec![(500, json!({})), (500, json!({}))]).await;
        let error = provider(&url, Some("sk-very-secret"))
            .complete_json(request(&["m"]))
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("sk-very-secret"));
    }

    fn image_ok(bytes: &[u8]) -> (u16, Value) {
        (
            200,
            json!({
                "model": "google/served-image",
                "choices": [{ "message": {
                    "content": "Here you go",
                    "images": [{ "image_url": { "url": format!("data:image/png;base64,{}", STANDARD.encode(bytes)) } }],
                } }],
                "usage": { "prompt_tokens": 900, "completion_tokens": 1300, "cost": 0.139 },
            }),
        )
    }

    fn image_request(models: &[&str], with_preview: bool) -> ImageRequest {
        ImageRequest {
            models: models.iter().map(|m| (*m).to_owned()).collect(),
            instruction: "Shorten the couch".into(),
            original: ImageInput {
                mime: "image/jpeg",
                bytes: vec![1, 2, 3],
            },
            preview: with_preview.then(|| ImageInput {
                mime: "image/png",
                bytes: vec![4, 5, 6],
            }),
        }
    }

    #[tokio::test]
    async fn generates_an_image_and_sends_text_original_and_preview_in_that_order() {
        let (up, url) = upstream(vec![image_ok(b"PNGDATA")]).await;
        let response = provider(&url, Some("k"))
            .generate(image_request(&["gen"], true))
            .await
            .unwrap();

        assert_eq!(response.bytes, b"PNGDATA");
        assert_eq!(response.model, "google/served-image");
        assert_eq!(response.usage.cost_usd, Some(0.139));

        let body = up.seen.lock().unwrap()[0].1.clone();
        assert_eq!(body["modalities"], json!(["image", "text"]));
        assert!(
            body.get("response_format").is_none(),
            "image requests are not schema-constrained"
        );
        let parts = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0]["text"], "Shorten the couch");
        let urls: Vec<&str> = parts[1..]
            .iter()
            .map(|p| p["image_url"]["url"].as_str().unwrap())
            .collect();
        assert!(urls[0].starts_with("data:image/jpeg;base64,"));
        assert!(urls[1].starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn the_preview_is_optional() {
        let (up, url) = upstream(vec![image_ok(b"x")]).await;
        provider(&url, Some("k"))
            .generate(image_request(&["gen"], false))
            .await
            .unwrap();
        let body = up.seen.lock().unwrap()[0].1.clone();
        assert_eq!(body["messages"][0]["content"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_model_that_answers_in_words_is_reported_as_a_refusal_with_what_it_said() {
        let reply = (
            200,
            json!({ "choices": [{ "message": { "content": "I can't edit photos of people." } }] }),
        );
        let (_, url) = upstream(vec![reply]).await;
        let error = provider(&url, Some("k"))
            .generate(image_request(&["gen"], true))
            .await
            .unwrap_err();
        assert!(
            matches!(&error, Error::Refused(m) if m.contains("can't edit photos of people")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn moderation_blocks_are_refusals_not_key_problems_and_do_not_fall_back() {
        let blocked = (
            403,
            json!({ "error": { "message": "Input flagged", "metadata": { "reasons": ["violence"] } } }),
        );
        let (up, url) = upstream(vec![blocked]).await;
        let error = provider(&url, Some("k"))
            .generate(image_request(&["a", "b"], true))
            .await
            .unwrap_err();
        assert!(
            matches!(&error, Error::Refused(m) if m.contains("content-policy")),
            "{error}"
        );
        assert_eq!(up.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn safety_system_rejections_are_refusals() {
        let blocked = (
            400,
            json!({ "error": { "message": "Your request was rejected by the safety system." } }),
        );
        let (_, url) = upstream(vec![blocked]).await;
        let error = provider(&url, Some("k"))
            .generate(image_request(&["gen"], true))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Refused(_)));
    }

    #[tokio::test]
    async fn an_empty_answer_moves_on_to_the_next_model() {
        let empty = (
            200,
            json!({ "choices": [{ "message": { "content": null } }] }),
        );
        let (up, url) = upstream(vec![empty, image_ok(b"second")]).await;
        let response = provider(&url, Some("k"))
            .generate(image_request(&["first", "second"], true))
            .await
            .unwrap();
        assert_eq!(response.bytes, b"second");
        assert_eq!(up.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn undecodable_images_are_skipped_and_exhaustion_is_reported_as_unavailable() {
        let garbage = || {
            (
                200,
                json!({ "choices": [{ "message": { "images": [{ "image_url": { "url": "data:image/png;base64,@@@" } }] } }] }),
            )
        };
        let (_, url) = upstream(vec![garbage(), garbage()]).await;
        let error = provider(&url, Some("k"))
            .generate(image_request(&["a", "b"], true))
            .await
            .unwrap_err();
        assert!(
            matches!(&error, Error::Unavailable(m) if m.contains("not responding")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn image_calls_retry_transient_errors() {
        let (up, url) = upstream(vec![(503, json!({})), image_ok(b"ok")]).await;
        provider(&url, Some("k"))
            .generate(image_request(&["gen"], true))
            .await
            .unwrap();
        assert_eq!(up.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn dropping_the_future_abandons_the_request_so_jobs_can_be_cancelled() {
        // The upstream never answers; cancelling must return promptly instead of waiting for it.
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                StatusCode::OK
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let generator = provider(&url, Some("k"));
        let started = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_millis(200),
            generator.generate(image_request(&["gen"], true)),
        )
        .await;
        assert!(
            outcome.is_err(),
            "the call should still be waiting when the timeout drops it"
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
