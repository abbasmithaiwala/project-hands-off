//! A scripted provider for tests: replays canned answers and records what it was asked.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use hands_off_core::{Error, Result};

use crate::provider::{
    ImageGenerator, ImageRequest, ImageResponse, JsonRequest, JsonResponse, LlmProvider, Usage,
};

/// Replays queued answers in order and records every request.
#[derive(Default)]
pub struct MockProvider {
    answers: Mutex<VecDeque<std::result::Result<String, String>>>,
    requests: Mutex<Vec<JsonRequest>>,
}

impl MockProvider {
    /// An empty mock. Calls fail until answers are queued.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a successful answer.
    pub fn push_ok(&self, content: impl Into<String>) {
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Ok(content.into()));
    }

    /// Queue a failure with the given message.
    pub fn push_err(&self, message: impl Into<String>) {
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Err(message.into()));
    }

    /// Every request received so far.
    #[must_use]
    pub fn requests(&self) -> Vec<JsonRequest> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete_json(&self, request: JsonRequest) -> Result<JsonResponse> {
        let model = request.models.first().cloned().unwrap_or_default();
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request);
        let next = self
            .answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front();
        match next {
            Some(Ok(content)) => Ok(JsonResponse {
                content,
                model,
                usage: Usage::default(),
                latency: Duration::ZERO,
            }),
            Some(Err(message)) => Err(Error::Unavailable(message)),
            None => Err(Error::Unavailable(
                "the mock provider has no more answers".into(),
            )),
        }
    }
}

/// A scripted image generator for tests: replays canned images and records its requests.
#[derive(Default)]
pub struct MockGenerator {
    answers: Mutex<VecDeque<std::result::Result<Vec<u8>, Error>>>,
    requests: Mutex<Vec<ImageRequest>>,
    delay: Mutex<Duration>,
}

impl MockGenerator {
    /// An empty mock. Calls fail until answers are queued.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a successful image.
    pub fn push_image(&self, bytes: Vec<u8>) {
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Ok(bytes));
    }

    /// Queue a failure.
    pub fn push_error(&self, error: Error) {
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Err(error));
    }

    /// Make each call wait first, to simulate a slow model.
    pub fn set_delay(&self, delay: Duration) {
        *self.delay.lock().unwrap_or_else(PoisonError::into_inner) = delay;
    }

    /// Every request received so far.
    #[must_use]
    pub fn requests(&self) -> Vec<ImageRequest> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl ImageGenerator for MockGenerator {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResponse> {
        let model = request.models.first().cloned().unwrap_or_default();
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request);
        let delay = *self.delay.lock().unwrap_or_else(PoisonError::into_inner);
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        let next = self
            .answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front();
        match next {
            Some(Ok(bytes)) => Ok(ImageResponse {
                bytes,
                model,
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    cost_usd: Some(0.05),
                },
                latency: delay,
            }),
            Some(Err(error)) => Err(error),
            None => Err(Error::Unavailable(
                "the mock generator has no more images".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Stage;

    fn request() -> JsonRequest {
        JsonRequest {
            stage: Stage::Suggester,
            models: vec!["m".into()],
            system: String::new(),
            user_text: "hi".into(),
            images: Vec::new(),
            schema_name: "x",
            schema: serde_json::Value::Null,
            repair: None,
        }
    }

    #[tokio::test]
    async fn replays_answers_in_order_and_records_requests() {
        let mock = MockProvider::new();
        mock.push_ok("one");
        mock.push_err("boom");
        assert_eq!(mock.complete_json(request()).await.unwrap().content, "one");
        assert!(
            matches!(mock.complete_json(request()).await.unwrap_err(), Error::Unavailable(m) if m == "boom")
        );
        assert!(mock.complete_json(request()).await.is_err());
        assert_eq!(mock.requests().len(), 3);
    }

    #[tokio::test]
    async fn the_mock_generator_replays_images_and_errors_and_records_requests() {
        let mock = MockGenerator::new();
        mock.push_image(vec![1, 2, 3]);
        mock.push_error(Error::Refused("no".into()));
        let request = || ImageRequest {
            models: vec!["m".into()],
            instruction: "go".into(),
            original: crate::provider::ImageInput {
                mime: "image/png",
                bytes: vec![],
            },
            preview: None,
        };
        assert_eq!(mock.generate(request()).await.unwrap().bytes, [1, 2, 3]);
        assert!(matches!(
            mock.generate(request()).await.unwrap_err(),
            Error::Refused(_)
        ));
        assert!(mock.generate(request()).await.is_err());
        assert_eq!(mock.requests().len(), 3);
    }
}
