//! Reading and changing user settings. The API key is write-only: it is never returned.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use hands_off_core::model::ModelSettings;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ApiError, ErrorBody};
use crate::settings::KeySource;
use crate::state::AppState;

/// Where the OpenRouter key comes from.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum KeyOrigin {
    /// Set through the server's environment.
    Environment,
    /// Saved through this API.
    Stored,
    /// Demo mode: no key is needed.
    Demo,
    /// No key yet.
    None,
}

/// Whether AI features can run, without revealing any secret.
#[derive(Debug, Serialize, ToSchema)]
pub struct KeyStatus {
    /// True when AI requests can be made.
    pub configured: bool,
    /// Where the key comes from.
    pub origin: KeyOrigin,
}

/// Current settings.
#[derive(Debug, Serialize, ToSchema)]
pub struct SettingsView {
    /// State of the OpenRouter key.
    pub openrouter: KeyStatus,
    /// The model used by each AI stage.
    pub models: ModelSettings,
}

/// Body for saving an API key.
#[derive(Deserialize, ToSchema)]
pub struct SetKey {
    /// The OpenRouter API key.
    pub key: String,
}

async fn view(state: &AppState) -> Result<SettingsView, ApiError> {
    let origin = if state.config.demo_ai {
        KeyOrigin::Demo
    } else {
        match state.settings.key_source().await? {
            KeySource::Environment => KeyOrigin::Environment,
            KeySource::Stored => KeyOrigin::Stored,
            KeySource::None => KeyOrigin::None,
        }
    };
    Ok(SettingsView {
        openrouter: KeyStatus {
            configured: !matches!(origin, KeyOrigin::None),
            origin,
        },
        models: state.settings.models().await?,
    })
}

/// Read the current settings.
#[utoipa::path(
    get,
    path = "/api/v1/settings",
    tag = "settings",
    responses((status = 200, body = SettingsView))
)]
pub async fn get_settings(State(state): State<AppState>) -> Result<Json<SettingsView>, ApiError> {
    Ok(Json(view(&state).await?))
}

/// Save the OpenRouter API key. It is stored on the server and never returned.
#[utoipa::path(
    put,
    path = "/api/v1/settings/openrouter-key",
    tag = "settings",
    request_body = SetKey,
    responses(
        (status = 204, description = "Key saved"),
        (status = 400, description = "That does not look like an API key", body = ErrorBody),
    )
)]
pub async fn set_key(
    State(state): State<AppState>,
    Json(body): Json<SetKey>,
) -> Result<StatusCode, ApiError> {
    state.settings.set_key(&body.key).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Remove the stored API key. A key set through the environment is unaffected.
#[utoipa::path(
    delete,
    path = "/api/v1/settings/openrouter-key",
    tag = "settings",
    responses((status = 204, description = "Key removed"))
)]
pub async fn clear_key(State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    state.settings.clear_key().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Choose the model for each AI stage. Any OpenRouter model id is accepted.
#[utoipa::path(
    put,
    path = "/api/v1/settings/models",
    tag = "settings",
    request_body = ModelSettings,
    responses(
        (status = 200, body = SettingsView),
        (status = 400, description = "A model id is malformed", body = ErrorBody),
    )
)]
pub async fn set_models(
    State(state): State<AppState>,
    Json(models): Json<ModelSettings>,
) -> Result<Json<SettingsView>, ApiError> {
    state.settings.set_models(&models).await?;
    Ok(Json(view(&state).await?))
}
