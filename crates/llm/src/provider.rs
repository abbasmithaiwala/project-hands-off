//! Provider-independent request and response types.

use std::time::Duration;

use async_trait::async_trait;
use hands_off_core::Result;
use secrecy::SecretString;

/// The step of the editing pipeline a request belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Suggests editable properties for an object.
    Suggester,
    /// Plans control geometry on the canvas.
    UiPlanner,
    /// Summarises edits.
    Summariser,
}

impl Stage {
    /// Name used in logs.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Suggester => "suggester",
            Self::UiPlanner => "ui_planner",
            Self::Summariser => "summariser",
        }
    }
}

/// An image attached to a request.
#[derive(Debug, Clone)]
pub struct ImageInput {
    /// MIME type, for example `image/jpeg`.
    pub mime: &'static str,
    /// Encoded image bytes.
    pub bytes: Vec<u8>,
}

/// Asks the model to correct an earlier answer that failed validation.
#[derive(Debug, Clone)]
pub struct Repair {
    /// What the model answered before.
    pub previous_answer: String,
    /// Why it was rejected.
    pub problem: String,
}

/// A request for a JSON answer that conforms to a schema.
#[derive(Debug, Clone)]
pub struct JsonRequest {
    /// Pipeline stage, for logging.
    pub stage: Stage,
    /// Models to try in order.
    pub models: Vec<String>,
    /// System prompt.
    pub system: String,
    /// User message text.
    pub user_text: String,
    /// Images shown to the model.
    pub images: Vec<ImageInput>,
    /// Short identifier of the schema.
    pub schema_name: &'static str,
    /// Strict JSON Schema the answer must satisfy.
    pub schema: serde_json::Value,
    /// Set when retrying after a rejected answer.
    pub repair: Option<Repair>,
}

/// Token usage and cost of one call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    /// Prompt tokens.
    pub prompt_tokens: u64,
    /// Completion tokens.
    pub completion_tokens: u64,
    /// Cost in US dollars, when the provider reports it.
    pub cost_usd: Option<f64>,
}

/// The model's answer. The content is unvalidated JSON text.
#[derive(Debug, Clone)]
pub struct JsonResponse {
    /// Raw answer text.
    pub content: String,
    /// The model that produced it.
    pub model: String,
    /// Token usage and cost.
    pub usage: Usage,
    /// Time the successful call took.
    pub latency: Duration,
}

/// Something that can answer structured requests.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Ask for a JSON answer.
    ///
    /// # Errors
    /// [`hands_off_core::Error::Unavailable`] if no key is set, credits are exhausted, or no
    /// model responds.
    async fn complete_json(&self, request: JsonRequest) -> Result<JsonResponse>;
}

/// Supplies the API key at call time, so it can be changed without restarting.
#[async_trait]
pub trait KeyProvider: Send + Sync {
    /// The current key, if one is configured.
    async fn api_key(&self) -> Result<Option<SecretString>>;
}
