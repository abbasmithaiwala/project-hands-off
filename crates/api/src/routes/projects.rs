//! Saved projects: listing, version history, renaming and deleting.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use hands_off_core::model::{ImageRecord, normalize_name};
use hands_off_core::storage::BlobKey;
use hands_off_core::{Error, ImageId};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::error::{ApiError, ApiJson, ApiPath, ErrorBody};
use crate::state::AppState;

const DEFAULT_PROJECT_LIMIT: u32 = 24;
const MAX_PROJECT_LIMIT: u32 = 100;

/// Query parameters for listing projects.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListQuery {
    /// Maximum number of projects to return (default 24, at most 100).
    limit: Option<u32>,
}

/// Body of a rename request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct Rename {
    /// The new project name (1 to 80 printable characters).
    pub name: String,
}

/// List saved projects, most recent first. Each entry is the project's latest version.
#[utoipa::path(
    get,
    path = "/api/v1/images",
    tag = "projects",
    params(ListQuery),
    responses((status = 200, description = "Latest version of each project", body = Vec<ImageRecord>))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ImageRecord>>, ApiError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PROJECT_LIMIT)
        .clamp(1, MAX_PROJECT_LIMIT);
    Ok(Json(state.store.list_tips(limit).await?))
}

/// The versions leading to an image, original first, each with what its edit changed.
#[utoipa::path(
    get,
    path = "/api/v1/images/{id}/history",
    tag = "projects",
    params(("id" = ImageId, Path, description = "Image id")),
    responses(
        (status = 200, description = "Versions from the original to this image", body = Vec<ImageRecord>),
        (status = 404, description = "Unknown image", body = ErrorBody),
    )
)]
pub async fn history(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ImageId>,
) -> Result<Json<Vec<ImageRecord>>, ApiError> {
    let chain = state.store.lineage(id).await?;
    if chain.is_empty() {
        return Err(Error::NotFound("image").into());
    }
    Ok(Json(chain))
}

/// Rename a project: every version leading to this image takes the new name.
#[utoipa::path(
    patch,
    path = "/api/v1/images/{id}",
    tag = "projects",
    params(("id" = ImageId, Path, description = "Image id")),
    request_body = Rename,
    responses(
        (status = 200, description = "The renamed image", body = ImageRecord),
        (status = 400, description = "Invalid name", body = ErrorBody),
        (status = 404, description = "Unknown image", body = ErrorBody),
    )
)]
pub async fn rename(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ImageId>,
    ApiJson(body): ApiJson<Rename>,
) -> Result<Json<ImageRecord>, ApiError> {
    let name = normalize_name(&body.name)?;
    state.store.rename_lineage(id, &name).await?;
    let record = state
        .store
        .image_by_id(id)
        .await?
        .ok_or(Error::NotFound("image"))?;
    Ok(Json(record))
}

/// Delete an image, every version edited from it, and their stored files.
#[utoipa::path(
    delete,
    path = "/api/v1/images/{id}",
    tag = "projects",
    params(("id" = ImageId, Path, description = "Image id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Unknown image", body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ImageId>,
) -> Result<StatusCode, ApiError> {
    let removed = state.store.delete_image_tree(id).await?;
    if removed.is_empty() {
        return Err(Error::NotFound("image").into());
    }
    for image in removed {
        state.blobs.remove_tree(&BlobKey::image_dir(image)).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
