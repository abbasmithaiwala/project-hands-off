//! Image upload and retrieval.

use axum::Json;
use axum::extract::{Multipart, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use hands_off_core::model::{DetectedObject, ImageRecord};
use hands_off_core::storage::BlobKey;
use hands_off_core::{Error, ImageId, JobId, ObjectId};
use serde::Serialize;
use utoipa::ToSchema;

use crate::analysis;
use crate::error::{ApiError, ApiPath, ErrorBody};
use crate::ingest::ingest;
use crate::state::AppState;

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
    let (record, created) = ingest(&state, bytes).await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(record)))
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
    ApiPath(id): ApiPath<ImageId>,
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
    ApiPath(id): ApiPath<ImageId>,
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
    ApiPath(id): ApiPath<ImageId>,
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

/// Reference to a newly started background job.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobCreated {
    /// Use with the `/jobs/{id}` routes to follow progress.
    pub job_id: JobId,
}

/// Start detecting and outlining the objects in an image.
///
/// The returned job streams one `object_found` event per object. Results are cached, so
/// analysing the same image again is instant.
#[utoipa::path(
    post,
    path = "/api/v1/images/{id}/analyze",
    tag = "images",
    params(("id" = ImageId, Path, description = "Image id")),
    responses(
        (status = 202, description = "Analysis started", body = JobCreated),
        (status = 404, description = "Unknown image", body = ErrorBody),
    )
)]
pub async fn analyze(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ImageId>,
) -> Result<(StatusCode, Json<JobCreated>), ApiError> {
    find(&state, id).await?;
    let worker = state.clone();
    let job_id = state
        .jobs
        .spawn("analyze", move |context| async move {
            analysis::run(&worker, id, &context).await
        })
        .await?;
    Ok((StatusCode::ACCEPTED, Json(JobCreated { job_id })))
}

/// List the objects found in an image.
#[utoipa::path(
    get,
    path = "/api/v1/images/{id}/objects",
    tag = "images",
    params(("id" = ImageId, Path, description = "Image id")),
    responses(
        (status = 200, description = "Detected objects, most confident first", body = Vec<DetectedObject>),
        (status = 404, description = "Unknown image, or it has not been analysed yet", body = ErrorBody),
    )
)]
pub async fn list_objects(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ImageId>,
) -> Result<Json<Vec<DetectedObject>>, ApiError> {
    find(&state, id).await?;
    let objects = analysis::cached_objects(&state, id)
        .await?
        .ok_or(Error::NotFound("analysis"))?;
    Ok(Json(objects))
}

/// Download one object's mask as a grayscale PNG (white is the object).
#[utoipa::path(
    get,
    path = "/api/v1/images/{id}/objects/{object_id}/mask",
    tag = "images",
    params(
        ("id" = ImageId, Path, description = "Image id"),
        ("object_id" = u32, Path, description = "Object index within the image"),
    ),
    responses(
        (status = 200, description = "Mask PNG", content_type = "image/png", body = Vec<u8>),
        (status = 404, description = "Unknown image or object", body = ErrorBody),
    )
)]
pub async fn get_mask(
    State(state): State<AppState>,
    ApiPath((id, object_id)): ApiPath<(ImageId, u32)>,
) -> Result<Response, ApiError> {
    find(&state, id).await?;
    let objects = analysis::cached_objects(&state, id)
        .await?
        .ok_or(Error::NotFound("analysis"))?;
    let object = ObjectId(object_id);
    if !objects.iter().any(|o| o.id == object) {
        return Err(Error::NotFound("object").into());
    }
    let bytes = state.blobs.get(&BlobKey::mask(id, object)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, CACHE_FOREVER),
        ],
        bytes,
    )
        .into_response())
}
