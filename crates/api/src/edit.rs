//! Applying a batch of edits: validate, summarise, generate with an image model, then blend
//! the result back into the original so only the edited areas change.

use std::collections::HashSet;
use std::collections::hash_map::Entry;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hands_off_core::model::{
    ChangeSummary, DetectedObject, Edit, EditChange, EditRequest, EditResult, is_valid_model_id,
};
use hands_off_core::storage::BlobKey;
use hands_off_core::{BBox, Error, ImageId, JobId, ObjectId, Result};
use hands_off_imaging::composite::{composite_edit, encode_jpeg, encode_png, fit_to, fit_within};
use hands_off_llm::contracts::{
    EditLine, SUMMARISER_SYSTEM, SUMMARY_SCHEMA, generation_instruction, parse_edit_summary,
    summariser_user_text, summary_schema, template_summary,
};
use hands_off_llm::{ImageInput, ImageRequest, JsonRequest, Stage};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::task::spawn_blocking;

use crate::ai::{ask, cached_property_labels};
use crate::analysis::cached_objects;
use crate::jobs::JobContext;
use crate::state::AppState;

const MAX_EDITS: usize = 20;
const MAX_REGIONS: usize = 40;
const MAX_POINTS: usize = 64;
const MAX_PREVIEW_BYTES: usize = 12 * 1024 * 1024;
const MODEL_IMAGE_MAX_SIDE: u32 = 1280;
const MAX_SLIDER_VALUE: f32 = 4.0;
/// Differences in proportions above this are reported in the activity log.
const ASPECT_WARNING: f32 = 0.05;

/// A validated request, ready to run.
#[derive(Debug)]
pub struct Prepared {
    image_id: ImageId,
    request: EditRequest,
    /// The object each edit refers to, in edit order.
    objects: Vec<DetectedObject>,
    preview: Option<Vec<u8>>,
}

/// What is kept for an edit job so the result can later be accepted.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredResult {
    /// The image the edits were applied to.
    pub image_id: ImageId,
    /// Summary and cost of the generation.
    pub result: EditResult,
}

fn check_change(index: usize, change: &EditChange) -> Result<()> {
    let invalid = |why: &str| Error::Validation(format!("edit {index}: {why}"));
    match change {
        EditChange::Slider { value, .. } => {
            if value.is_finite() && *value > 0.0 && *value <= MAX_SLIDER_VALUE {
                Ok(())
            } else {
                Err(invalid("the slider value must be above 0 and at most 4"))
            }
        }
        EditChange::ControlPoints { origin, target }
        | EditChange::MultiHandle { origin, target } => {
            if origin.is_empty() || origin.len() > MAX_POINTS || origin.len() != target.len() {
                Err(invalid(
                    "origin and target must have the same number of points (1 to 64)",
                ))
            } else {
                Ok(())
            }
        }
    }
}

/// Check a request against the image it targets, so mistakes fail immediately with a clear
/// message instead of partway through a paid generation.
///
/// # Errors
/// [`Error::NotFound`] for an unanalysed image, [`Error::Validation`] for a malformed request,
/// [`Error::PayloadTooLarge`] for an oversized preview.
pub async fn prepare(
    state: &AppState,
    image_id: ImageId,
    request: EditRequest,
) -> Result<Prepared> {
    if request.edits.is_empty() {
        return Err(Error::Validation("at least one edit is required".into()));
    }
    if request.edits.len() > MAX_EDITS || request.regions.len() > MAX_REGIONS {
        return Err(Error::Validation(format!(
            "at most {MAX_EDITS} edits and {MAX_REGIONS} regions are allowed"
        )));
    }
    if request
        .model
        .as_deref()
        .is_some_and(|m| !is_valid_model_id(m))
    {
        return Err(Error::Validation("model is not a valid model id".into()));
    }

    let known = cached_objects(state, image_id)
        .await?
        .ok_or(Error::NotFound("analysis"))?;
    let mut objects = Vec::with_capacity(request.edits.len());
    let mut seen = HashSet::new();
    for (index, edit) in request.edits.iter().enumerate() {
        let object = known
            .iter()
            .find(|o| o.id == edit.object_id)
            .ok_or_else(|| {
                Error::Validation(format!("edit {index} refers to an unknown object"))
            })?;
        if !seen.insert((edit.object_id, edit.property.clone())) {
            return Err(Error::Validation(format!(
                "edit {index} repeats an object and property"
            )));
        }
        check_change(index, &edit.change)?;
        objects.push(object.clone());
    }

    let preview = request
        .preview_png
        .as_deref()
        .map(|encoded| {
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| Error::Validation("preview_png is not valid base64".into()))?;
            if bytes.len() > MAX_PREVIEW_BYTES {
                return Err(Error::PayloadTooLarge(
                    "the preview image is too large".into(),
                ));
            }
            hands_off_imaging::inspect(&bytes, &state.config.limits)?;
            Ok(bytes)
        })
        .transpose()?;

    Ok(Prepared {
        image_id,
        request,
        objects,
        preview,
    })
}

fn humanise(key: &str) -> String {
    let spaced = key.replace('_', " ");
    let mut chars = spaced.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

/// The boxes that may change: those the client reported plus every edited object's own box.
fn changed_regions(request: &EditRequest, objects: &[DetectedObject]) -> Vec<BBox> {
    let mut regions = request.regions.clone();
    for object in objects {
        if !regions.contains(&object.bbox) {
            regions.push(object.bbox);
        }
    }
    regions
}

fn seconds(ms: u64) -> String {
    format!("{:.1}s", std::time::Duration::from_millis(ms).as_secs_f64())
}

/// The original and the preview, shrunk and encoded for the image model.
struct ModelInputs {
    original: Vec<u8>,
    preview: Option<Vec<u8>>,
}

async fn model_inputs(original: Vec<u8>, preview: Option<Vec<u8>>) -> Result<ModelInputs> {
    spawn_blocking(move || -> Result<_> {
        let decoded = hands_off_imaging::decode_oriented(&original)?;
        let scaled = fit_within(&decoded, MODEL_IMAGE_MAX_SIDE);
        let (width, height) = scaled.dimensions();
        let preview = preview
            .map(|bytes| -> Result<Vec<u8>> {
                let decoded = hands_off_imaging::decode_oriented(&bytes)?;
                encode_jpeg(&fit_to(&decoded, width, height))
            })
            .transpose()?;
        Ok(ModelInputs {
            original: encode_jpeg(&scaled)?,
            preview,
        })
    })
    .await
    .map_err(Error::internal)?
}

/// Blend the generated image into the original and encode the result as PNG.
async fn blend_result(
    original: Vec<u8>,
    generated: Vec<u8>,
    regions: Vec<BBox>,
) -> Result<(Vec<u8>, f32)> {
    spawn_blocking(move || -> Result<_> {
        let original = hands_off_imaging::decode_oriented(&original)?;
        let generated = hands_off_imaging::decode_oriented(&generated)?;
        let composited = composite_edit(&original, &generated, &regions);
        Ok((encode_png(&composited.image)?, composited.aspect_difference))
    })
    .await
    .map_err(Error::internal)?
}

/// Run the edit: summarise, generate, composite and store. Reports progress as it goes.
///
/// # Errors
/// [`Error::Unavailable`] when no model can be reached, [`Error::Refused`] when the image model
/// declines, and other errors for storage or decoding failures.
pub async fn execute(
    state: &AppState,
    prepared: Prepared,
    context: &JobContext,
) -> Result<Option<serde_json::Value>> {
    let Prepared {
        image_id,
        request,
        objects,
        preview,
    } = prepared;

    context.progress("analysing", "Preparing your edits");
    let original_bytes = state.blobs.get(&BlobKey::original(image_id)).await?;
    let inputs = model_inputs(original_bytes.clone(), preview).await?;

    context.progress("planning", "Working out how to describe the changes");
    let labels = property_labels(state, image_id, &request.edits).await?;
    let lines: Vec<EditLine<'_>> = request
        .edits
        .iter()
        .zip(&objects)
        .enumerate()
        .map(|(index, (edit, object))| EditLine {
            index,
            object_label: &object.label,
            property_label: labels
                .get(index)
                .map_or(edit.property.as_str(), String::as_str),
            change: &edit.change,
        })
        .collect();
    let summary = summarise(state, &lines, context).await;
    context.log(&format!("Instruction: {}", summary.instruction));

    context.progress("generating", "Generating the edited image");
    let models = state.settings.models().await?;
    let primary = request.model.clone().unwrap_or(models.generator);
    let generated = state
        .generator
        .generate(ImageRequest {
            models: vec![primary, models.fallback],
            instruction: generation_instruction(&summary.instruction, inputs.preview.is_some()),
            original: ImageInput {
                mime: "image/jpeg",
                bytes: inputs.original,
            },
            preview: inputs.preview.map(|bytes| ImageInput {
                mime: "image/jpeg",
                bytes,
            }),
        })
        .await?;
    let generation_ms = u64::try_from(generated.latency.as_millis()).unwrap_or(u64::MAX);
    let cost = generated
        .usage
        .cost_usd
        .map(|c| format!(" · ${c:.3}"))
        .unwrap_or_default();
    context.log(&format!(
        "Generated by {} in {}{cost}",
        generated.model,
        seconds(generation_ms)
    ));

    context.progress("compositing", "Blending the result into your photo");
    let regions = changed_regions(&request, &objects);
    let (png, aspect_difference) = blend_result(original_bytes, generated.bytes, regions).await?;
    if aspect_difference > ASPECT_WARNING {
        context.log(
            "The model changed the image proportions, so its result was stretched back to fit.",
        );
    }

    let result = EditResult {
        summary: summary.summary,
        changes: summary.changes,
        instruction: summary.instruction,
        model: generated.model,
        cost_usd: generated.usage.cost_usd,
        generation_ms,
    };
    let job = context.id();
    state.blobs.put(&BlobKey::job_result(job), png).await?;
    let stored = serde_json::to_vec(&StoredResult {
        image_id,
        result: result.clone(),
    })
    .map_err(Error::internal)?;
    state.blobs.put(&BlobKey::job_meta(job), stored).await?;

    context.progress("done", "Your edit is ready to review");
    Ok(Some(json!(result)))
}

async fn property_labels(state: &AppState, image: ImageId, edits: &[Edit]) -> Result<Vec<String>> {
    let mut by_object: std::collections::HashMap<
        ObjectId,
        std::collections::HashMap<String, String>,
    > = std::collections::HashMap::new();
    let mut labels = Vec::with_capacity(edits.len());
    for edit in edits {
        if let Entry::Vacant(slot) = by_object.entry(edit.object_id) {
            slot.insert(cached_property_labels(state, image, edit.object_id).await?);
        }
        let label = by_object
            .get(&edit.object_id)
            .and_then(|known| known.get(&edit.property))
            .cloned()
            .unwrap_or_else(|| humanise(&edit.property));
        labels.push(label);
    }
    Ok(labels)
}

/// Ask a model to describe the edits; fall back to a template if it cannot.
async fn summarise(
    state: &AppState,
    lines: &[EditLine<'_>],
    context: &JobContext,
) -> hands_off_llm::contracts::EditSummary {
    let models = match state.settings.models().await {
        Ok(models) => models,
        Err(error) => {
            context.log(&format!(
                "Using a plain description of the changes ({error})"
            ));
            return template_summary(lines);
        }
    };
    let request = JsonRequest {
        stage: Stage::Summariser,
        models: vec![models.summariser, models.fallback],
        system: SUMMARISER_SYSTEM.into(),
        user_text: summariser_user_text(lines),
        images: Vec::new(),
        schema_name: SUMMARY_SCHEMA,
        schema: summary_schema(),
        repair: None,
    };
    match ask(state, request, |content| parse_edit_summary(content, lines)).await {
        Ok(Some(summary)) => summary,
        Ok(None) => {
            context
                .log("Using a plain description of the changes (the model's answer was unusable)");
            template_summary(lines)
        }
        Err(error) => {
            context.log(&format!(
                "Using a plain description of the changes ({error})"
            ));
            template_summary(lines)
        }
    }
}

/// Load what was stored for a finished edit job.
///
/// # Errors
/// [`Error::NotFound`] if the job has no stored result.
pub async fn stored_result(state: &AppState, job: JobId) -> Result<StoredResult> {
    let bytes = state
        .blobs
        .get(&BlobKey::job_meta(job))
        .await
        .map_err(|e| match e {
            Error::NotFound(_) => Error::NotFound("edit result"),
            other => other,
        })?;
    serde_json::from_slice(&bytes).map_err(Error::internal)
}

/// The change log lines of a result, for display in order.
#[must_use]
pub fn change_lines(changes: &[ChangeSummary]) -> Vec<&str> {
    changes.iter().map(|c| c.text.as_str()).collect()
}
