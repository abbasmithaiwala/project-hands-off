//! The image analysis job: detect objects, outline them and cache the result.

use hands_off_core::job::JobEvent;
use hands_off_core::model::DetectedObject;
use hands_off_core::storage::BlobKey;
use hands_off_core::{Error, ImageId, ObjectId, Result};
use hands_off_vision::Detection;
use serde_json::json;
use tokio::task::spawn_blocking;

use crate::jobs::JobContext;
use crate::state::AppState;

/// Maximum deviation of a simplified outline from the traced mask boundary, in mask pixels.
const OUTLINE_TOLERANCE_PX: f32 = 1.5;
const MIN_OUTLINE_POINTS: usize = 3;

/// An object together with its encoded mask.
struct Analysed {
    object: DetectedObject,
    mask_png: Vec<u8>,
}

/// Load the objects found earlier for `id`, if the image has been analysed.
///
/// # Errors
/// [`Error::Internal`] if storage fails or the cached analysis is unreadable.
pub async fn cached_objects(state: &AppState, id: ImageId) -> Result<Option<Vec<DetectedObject>>> {
    match state.blobs.get(&BlobKey::analysis(id)).await {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(Error::internal),
        Err(Error::NotFound(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Analyse an image, streaming each object to the job's subscribers as it is found.
///
/// # Errors
/// [`Error::NotFound`] for unknown images, [`Error::Unavailable`] if the model is missing,
/// and other errors if decoding or inference fails.
pub async fn run(
    state: &AppState,
    id: ImageId,
    context: &JobContext,
) -> Result<Option<serde_json::Value>> {
    state
        .store
        .image_by_id(id)
        .await?
        .ok_or(Error::NotFound("image"))?;

    if let Some(objects) = cached_objects(state, id).await? {
        context.progress("cached", "Using the earlier analysis");
        announce(context, &objects);
        return Ok(Some(json!({ "objects": objects.len() })));
    }

    context.progress("loading_model", "Preparing the detection model");
    let detector = state.detectors.detector(context).await?;

    context.progress("detecting", "Finding objects");
    let bytes = state.blobs.get(&BlobKey::original(id)).await?;
    let analysed = spawn_blocking(move || -> Result<Vec<Analysed>> {
        let image = hands_off_imaging::decode_oriented(&bytes)?;
        analyse(detector.detect(&image)?)
    })
    .await
    .map_err(Error::internal)??;

    for item in &analysed {
        state
            .blobs
            .put(&BlobKey::mask(id, item.object.id), item.mask_png.clone())
            .await?;
    }
    let objects: Vec<DetectedObject> = analysed.into_iter().map(|a| a.object).collect();
    let encoded = serde_json::to_vec(&objects).map_err(Error::internal)?;
    state.blobs.put(&BlobKey::analysis(id), encoded).await?;

    announce(context, &objects);
    context.log(&format!("Found {} objects", objects.len()));
    Ok(Some(json!({ "objects": objects.len() })))
}

fn announce(context: &JobContext, objects: &[DetectedObject]) {
    for object in objects {
        context.event(JobEvent::ObjectFound {
            object: object.clone(),
        });
    }
}

fn analyse(detections: Vec<Detection>) -> Result<Vec<Analysed>> {
    let mut analysed = Vec::new();
    for detection in detections {
        let outline = detection.mask.outline(OUTLINE_TOLERANCE_PX);
        if outline.len() < MIN_OUTLINE_POINTS {
            continue;
        }
        let id = ObjectId(u32::try_from(analysed.len()).map_err(Error::internal)?);
        analysed.push(Analysed {
            object: DetectedObject {
                id,
                label: detection.label.to_owned(),
                bbox: detection.bbox,
                score: detection.score,
                outline,
            },
            mask_png: detection.mask.to_png()?,
        });
    }
    Ok(analysed)
}
