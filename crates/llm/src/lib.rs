//! `OpenRouter` gateway and model-facing contracts.

pub mod contracts;
pub mod defaults;
pub mod demo;
pub mod mock;
pub mod openrouter;
pub mod provider;
pub mod schema;

pub use provider::{
    ImageGenerator, ImageInput, ImageRequest, ImageResponse, JsonRequest, JsonResponse,
    KeyProvider, LlmProvider, Repair, Stage, Usage,
};
