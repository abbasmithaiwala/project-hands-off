//! AI-assisted editing controls for one detected object.

use axum::Json;
use axum::extract::{Path, State};
use hands_off_core::model::{PropertyList, Widget};
use hands_off_core::{ImageId, ObjectId};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::ai;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

/// Which property to place a control for.
#[derive(Deserialize, ToSchema)]
pub struct PlanWidget {
    /// A property key returned by the properties route.
    pub property: String,
}

/// Suggest the properties that can be edited on an object.
///
/// Results are cached, so repeated calls are free. Calls a model on first use.
#[utoipa::path(
    post,
    path = "/api/v1/images/{id}/objects/{object_id}/properties",
    tag = "objects",
    params(
        ("id" = ImageId, Path, description = "Image id"),
        ("object_id" = u32, Path, description = "Object index within the image"),
    ),
    responses(
        (status = 200, body = PropertyList),
        (status = 404, description = "Unknown image or object, or not analysed yet", body = ErrorBody),
        (status = 503, description = "No API key, no credits, or the models are unreachable", body = ErrorBody),
    )
)]
pub async fn properties(
    State(state): State<AppState>,
    Path((id, object_id)): Path<(ImageId, u32)>,
) -> Result<Json<PropertyList>, ApiError> {
    Ok(Json(ai::properties(&state, id, ObjectId(object_id)).await?))
}

/// Place the on-canvas control for one property of an object.
///
/// Results are cached. The geometry is validated and snapped onto the object's mask; if the
/// model cannot produce a usable plan, a sensible default placement is returned instead.
#[utoipa::path(
    post,
    path = "/api/v1/images/{id}/objects/{object_id}/widget",
    tag = "objects",
    params(
        ("id" = ImageId, Path, description = "Image id"),
        ("object_id" = u32, Path, description = "Object index within the image"),
    ),
    request_body = PlanWidget,
    responses(
        (status = 200, body = Widget),
        (status = 404, description = "Unknown image, object or property", body = ErrorBody),
        (status = 503, description = "No API key, no credits, or the models are unreachable", body = ErrorBody),
    )
)]
pub async fn widget(
    State(state): State<AppState>,
    Path((id, object_id)): Path<(ImageId, u32)>,
    Json(body): Json<PlanWidget>,
) -> Result<Json<Widget>, ApiError> {
    Ok(Json(
        ai::widget(&state, id, ObjectId(object_id), &body.property).await?,
    ))
}
