//! Property suggestions and widget planning: ask a model, validate, repair once, then fall
//! back to deterministic defaults. Results are cached per image, object and prompt version.

use hands_off_core::model::{DetectedObject, Property, PropertyList, Widget, WidgetKind};
use hands_off_core::storage::BlobKey;
use hands_off_core::{Error, ImageId, ObjectId, Result};
use hands_off_imaging::crop::{CropRegion, crop_mask_to_jpeg, crop_to_jpeg};
use hands_off_imaging::mask::Mask;
use hands_off_llm::contracts::{
    PLANNER_PROMPT_VERSION, PLANNER_SYSTEM, SUGGESTER_PROMPT_VERSION, SUGGESTER_SYSTEM,
    SUGGESTIONS_SCHEMA, WIDGET_SCHEMA, parse_suggestions, parse_widget, planner_user_text,
    suggester_user_text, suggestion_schema, widget_schema,
};
use hands_off_llm::defaults::{default_properties, default_widget};
use hands_off_llm::{ImageInput, JsonRequest, Repair, Stage};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::task::spawn_blocking;

use crate::analysis::find_object;
use crate::state::AppState;

const CROP_PADDING: f32 = 0.1;
const CROP_MAX_SIDE: u32 = 768;

async fn cached<T: DeserializeOwned>(state: &AppState, key: &BlobKey) -> Result<Option<T>> {
    match state.blobs.get(key).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).ok()),
        Err(Error::NotFound(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

async fn remember<T: Serialize>(state: &AppState, key: &BlobKey, value: &T) -> Result<()> {
    state
        .blobs
        .put(key, serde_json::to_vec(value).map_err(Error::internal)?)
        .await
}

/// Ask a model, validate its answer, and ask once more with the problem described if the
/// first answer was unusable. `Ok(None)` means neither answer passed validation.
pub(crate) async fn ask<T>(
    state: &AppState,
    mut request: JsonRequest,
    validate: impl Fn(&str) -> std::result::Result<T, String>,
) -> Result<Option<T>> {
    let first = state.llm.complete_json(request.clone()).await?;
    let problem = match validate(&first.content) {
        Ok(value) => return Ok(Some(value)),
        Err(problem) => problem,
    };
    tracing::warn!(stage = request.stage.name(), %problem, "model answer rejected, retrying once");
    request.repair = Some(Repair {
        previous_answer: first.content,
        problem,
    });
    let second = state.llm.complete_json(request.clone()).await?;
    match validate(&second.content) {
        Ok(value) => Ok(Some(value)),
        Err(problem) => {
            tracing::warn!(stage = request.stage.name(), %problem, "model answer rejected again, using defaults");
            Ok(None)
        }
    }
}

async fn original(state: &AppState, id: ImageId) -> Result<image::DynamicImage> {
    let bytes = state.blobs.get(&BlobKey::original(id)).await?;
    spawn_blocking(move || hands_off_imaging::decode_oriented(&bytes))
        .await
        .map_err(Error::internal)?
}

/// The labels of an object's suggested properties, keyed by property key. Reads the cache only
/// and never calls a model, so it is empty for objects whose properties were never requested.
///
/// # Errors
/// [`Error::Internal`] if storage fails.
pub async fn cached_property_labels(
    state: &AppState,
    image: ImageId,
    object_id: ObjectId,
) -> Result<std::collections::HashMap<String, String>> {
    let key = BlobKey::ai_cache(
        image,
        object_id,
        &format!("properties.{SUGGESTER_PROMPT_VERSION}"),
    )?;
    let list = cached::<PropertyList>(state, &key).await?;
    Ok(list
        .map(|l| l.properties.into_iter().map(|p| (p.key, p.label)).collect())
        .unwrap_or_default())
}

/// The editable properties for an object, from cache or a model.
///
/// # Errors
/// [`Error::NotFound`] for unknown images or objects; [`Error::Unavailable`] if no model can
/// be reached (for example, no API key).
pub async fn properties(
    state: &AppState,
    image: ImageId,
    object_id: ObjectId,
) -> Result<PropertyList> {
    let object = find_object(state, image, object_id).await?;
    let key = BlobKey::ai_cache(
        image,
        object_id,
        &format!("properties.{SUGGESTER_PROMPT_VERSION}"),
    )?;
    if let Some(list) = cached::<PropertyList>(state, &key).await? {
        return Ok(list);
    }

    let region = CropRegion::around(&object.bbox, CROP_PADDING);
    let decoded = original(state, image).await?;
    let crop = spawn_blocking(move || crop_to_jpeg(&decoded, &region, CROP_MAX_SIDE))
        .await
        .map_err(Error::internal)??;

    let models = state.settings.models().await?;
    let request = JsonRequest {
        stage: Stage::Suggester,
        models: vec![models.suggester, models.fallback],
        system: SUGGESTER_SYSTEM.into(),
        user_text: suggester_user_text(&object.label),
        images: vec![ImageInput {
            mime: "image/jpeg",
            bytes: crop,
        }],
        schema_name: SUGGESTIONS_SCHEMA,
        schema: suggestion_schema(),
        repair: None,
    };
    let properties = ask(state, request, parse_suggestions)
        .await?
        .unwrap_or_else(default_properties);

    let list = PropertyList { properties };
    remember(state, &key, &list).await?;
    Ok(list)
}

/// The control geometry for one property of an object, from cache or a model.
///
/// # Errors
/// [`Error::NotFound`] for unknown images, objects or properties; [`Error::Unavailable`] if
/// no model can be reached.
pub async fn widget(
    state: &AppState,
    image: ImageId,
    object_id: ObjectId,
    property_key: &str,
) -> Result<Widget> {
    let object = find_object(state, image, object_id).await?;
    let property = properties(state, image, object_id)
        .await?
        .properties
        .into_iter()
        .find(|p| p.key == property_key)
        .ok_or(Error::NotFound("property"))?;

    let key = BlobKey::ai_cache(
        image,
        object_id,
        &format!("widget.{}.{PLANNER_PROMPT_VERSION}", property.key),
    )?;
    if let Some(widget) = cached::<Widget>(state, &key).await? {
        return Ok(widget);
    }

    let widget = plan(state, image, &object, &property).await?;
    remember(state, &key, &widget).await?;
    Ok(widget)
}

async fn plan(
    state: &AppState,
    image: ImageId,
    object: &DetectedObject,
    property: &Property,
) -> Result<Widget> {
    // These controls are fully determined by the object's own geometry (its outline reduced to
    // corners, or its box), so no model call is needed.
    if matches!(
        property.widget,
        WidgetKind::ControlPoints | WidgetKind::Scale | WidgetKind::Move
    ) {
        return Ok(default_widget(property.widget, &property.key, object));
    }
    let region = CropRegion::around(&object.bbox, CROP_PADDING);
    let decoded = original(state, image).await?;
    let mask = Mask::from_png(&state.blobs.get(&BlobKey::mask(image, object.id)).await?)?;

    let (photo, mask_image) = {
        let mask = mask.clone();
        spawn_blocking(move || -> Result<_> {
            Ok((
                crop_to_jpeg(&decoded, &region, CROP_MAX_SIDE)?,
                crop_mask_to_jpeg(&mask, &region, CROP_MAX_SIDE)?,
            ))
        })
        .await
        .map_err(Error::internal)??
    };

    let object_box = box_in_crop(&object.bbox, &region);
    let models = state.settings.models().await?;
    let request = JsonRequest {
        stage: Stage::UiPlanner,
        models: vec![models.ui_planner, models.fallback],
        system: PLANNER_SYSTEM.into(),
        user_text: planner_user_text(&object.label, property, object_box),
        images: vec![
            ImageInput {
                mime: "image/jpeg",
                bytes: photo,
            },
            ImageInput {
                mime: "image/jpeg",
                bytes: mask_image,
            },
        ],
        schema_name: WIDGET_SCHEMA,
        schema: widget_schema(),
        repair: None,
    };

    let kind = property.widget;
    let place = |x: f32, y: f32| region.to_image_point(x, y);
    let snap = |point| mask.snap(point);
    let planned = ask(state, request, |content| {
        parse_widget(content, kind, &place, &snap)
    })
    .await?;
    Ok(planned.unwrap_or_else(|| default_widget(kind, &property.key, object)))
}

/// The object's box in crop coordinates as `[left, top, right, bottom]`.
fn box_in_crop(bbox: &hands_off_core::BBox, region: &CropRegion) -> [f32; 4] {
    let to_crop =
        |value: f32, origin: f32, extent: f32| ((value - origin) / extent).clamp(0.0, 1.0);
    [
        to_crop(bbox.x, region.x, region.width),
        to_crop(bbox.y, region.y, region.height),
        to_crop(bbox.x + bbox.width, region.x, region.width),
        to_crop(bbox.y + bbox.height, region.y, region.height),
    ]
}

#[cfg(test)]
mod tests {
    use hands_off_core::BBox;

    use super::*;

    #[test]
    fn the_object_box_is_expressed_in_crop_coordinates() {
        let bbox = BBox::new(0.25, 0.25, 0.5, 0.5).unwrap();
        let region = CropRegion::around(&bbox, 0.1);
        let [left, top, right, bottom] = box_in_crop(&bbox, &region);
        assert!((left - 0.0833).abs() < 1e-3 && (right - 0.9167).abs() < 1e-3);
        assert!((top - left).abs() < 1e-6 && (bottom - right).abs() < 1e-6);
    }

    #[test]
    fn a_box_filling_the_image_maps_to_the_whole_crop() {
        let bbox = BBox::new(0.0, 0.0, 1.0, 1.0).unwrap();
        let region = CropRegion::around(&bbox, 0.1);
        let crop_box = box_in_crop(&bbox, &region);
        assert!(
            crop_box
                .iter()
                .zip([0.0, 0.0, 1.0, 1.0])
                .all(|(a, b)| (a - b).abs() < 1e-6)
        );
    }
}
