//! Image upload and retrieval.

use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use hands_off_core::model::ImageRecord;
use hands_off_core::storage::BlobKey;
use hands_off_core::{ContentHash, Error, ImageId};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::task::spawn_blocking;
use utoipa::ToSchema;

use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

const THUMBNAIL_MAX_SIDE: u32 = 512;
const CACHE_FOREVER: &str = "private, max-age=31536000, immutable";

/// Multipart form accepted by the upload endpoint.
#[derive(Serialize, ToSchema)]
pub struct UploadForm {
    /// The image file (JPEG, PNG or WebP).
    #[schema(format = Binary, value_type = String)]
    pub file: Vec<u8>,
}

async fn read_file_field(multipart: &mut Multipart) -> Result<Vec<u8>, ApiError> {
    while let Some(field) = multipart.next_field().await? {
        if field.name() == Some("file") {
            return Ok(field.bytes().await?.to_vec());
        }
    }
    Err(Error::Validation("multipart field `file` is required".into()).into())
}

fn sha256_hex(bytes: &[u8]) -> Result<ContentHash, Error> {
    ContentHash::new(hex::encode(Sha256::digest(bytes)))
}

/// Upload an image. Re-uploading identical bytes returns the existing record.
#[utoipa::path(
    post,
    path = "/api/v1/images",
    tag = "images",
    request_body(content = UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Image stored", body = ImageRecord),
        (status = 200, description = "An identical image already existed", body = ImageRecord),
        (status = 400, description = "Malformed request", body = ErrorBody),
        (status = 413, description = "File or dimensions too large", body = ErrorBody),
        (status = 415, description = "Unsupported image type", body = ErrorBody),
    )
)]
pub async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImageRecord>), ApiError> {
    let bytes = read_file_field(&mut multipart).await?;
    let limits = state.config.limits;

    let (info, hash, bytes) = spawn_blocking(move || {
        let info = hands_off_imaging::inspect(&bytes, &limits)?;
        let hash = sha256_hex(&bytes)?;
        Ok::<_, Error>((info, hash, bytes))
    })
    .await
    .map_err(Error::internal)??;

    if let Some(existing) = state.store.image_by_hash(&hash).await? {
        return Ok((StatusCode::OK, Json(existing)));
    }

    let record = ImageRecord {
        id: ImageId::new(),
        hash,
        format: info.format,
        width: info.width,
        height: info.height,
        size_bytes: bytes.len() as u64,
        created_at: Utc::now(),
    };

    let (original, thumbnail) = spawn_blocking(move || {
        let thumbnail = hands_off_imaging::thumbnail(&bytes, THUMBNAIL_MAX_SIDE)?;
        Ok::<_, Error>((bytes, thumbnail))
    })
    .await
    .map_err(Error::internal)??;

    state
        .blobs
        .put(&BlobKey::original(record.id), original)
        .await?;
    state
        .blobs
        .put(&BlobKey::thumbnail(record.id), thumbnail)
        .await?;
    if let Err(error) = state.store.insert_image(&record).await {
        // A concurrent identical upload may have won the unique-hash race.
        return match state.store.image_by_hash(&record.hash).await? {
            Some(existing) => Ok((StatusCode::OK, Json(existing))),
            None => Err(error.into()),
        };
    }
    Ok((StatusCode::CREATED, Json(record)))
}

async fn find(state: &AppState, id: ImageId) -> Result<ImageRecord, ApiError> {
    state
        .store
        .image_by_id(id)
        .await?
        .ok_or_else(|| Error::NotFound("image").into())
}

/// Fetch an image's metadata.
#[utoipa::path(
    get,
    path = "/api/v1/images/{id}",
    tag = "images",
    params(("id" = ImageId, Path, description = "Image id")),
    responses(
        (status = 200, body = ImageRecord),
        (status = 404, description = "Unknown image", body = ErrorBody),
    )
)]
pub async fn get_image(
    State(state): State<AppState>,
    Path(id): Path<ImageId>,
) -> Result<Json<ImageRecord>, ApiError> {
    Ok(Json(find(&state, id).await?))
}

/// Download the original image bytes.
#[utoipa::path(
    get,
    path = "/api/v1/images/{id}/original",
    tag = "images",
    params(("id" = ImageId, Path, description = "Image id")),
    responses(
        (status = 200, description = "The original image", content_type = "image/*", body = Vec<u8>),
        (status = 404, description = "Unknown image", body = ErrorBody),
    )
)]
pub async fn get_original(
    State(state): State<AppState>,
    Path(id): Path<ImageId>,
) -> Result<Response, ApiError> {
    let record = find(&state, id).await?;
    let bytes = state.blobs.get(&BlobKey::original(id)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, record.format.mime()),
            (header::CACHE_CONTROL, CACHE_FOREVER),
        ],
        bytes,
    )
        .into_response())
}

/// Download a JPEG thumbnail.
#[utoipa::path(
    get,
    path = "/api/v1/images/{id}/thumbnail",
    tag = "images",
    params(("id" = ImageId, Path, description = "Image id")),
    responses(
        (status = 200, description = "JPEG thumbnail", content_type = "image/jpeg", body = Vec<u8>),
        (status = 404, description = "Unknown image", body = ErrorBody),
    )
)]
pub async fn get_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<ImageId>,
) -> Result<Response, ApiError> {
    find(&state, id).await?;
    let bytes = state.blobs.get(&BlobKey::thumbnail(id)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, CACHE_FOREVER),
        ],
        bytes,
    )
        .into_response())
}
