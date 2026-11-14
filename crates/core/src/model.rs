//! Domain model shared by the API, the model-facing contracts and the frontend.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::geometry::{BBox, NormalizedPoint};
use crate::ids::{ContentHash, ImageId, ObjectId};

/// Supported upload formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// JPEG.
    Jpeg,
    /// PNG.
    Png,
    /// WebP.
    Webp,
}

impl ImageFormat {
    /// The MIME type for this format.
    #[must_use]
    pub fn mime(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }

    /// Stable lowercase name used for storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    /// Parse the storage name produced by [`ImageFormat::as_str`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }
}

/// Metadata of an uploaded image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct ImageRecord {
    /// Unique identifier.
    pub id: ImageId,
    /// SHA-256 of the original bytes; identical uploads share one record.
    pub hash: ContentHash,
    /// Detected format.
    pub format: ImageFormat,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Size of the original file in bytes.
    pub size_bytes: u64,
    /// Upload time.
    pub created_at: DateTime<Utc>,
}

/// An object found in an image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct DetectedObject {
    /// Index within the image.
    pub id: ObjectId,
    /// Human-readable class label.
    pub label: String,
    /// Bounding box.
    pub bbox: BBox,
    /// Detector confidence in `0.0..=1.0`.
    pub score: f32,
    /// Simplified outline of the object's mask, as a closed polygon.
    pub outline: Vec<NormalizedPoint>,
}

/// The kind of on-canvas control used to edit a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    /// A single draggable value along an axis.
    Slider,
    /// A polygon of draggable control points.
    ControlPoints,
    /// Several independent handles (puppet-warp style).
    MultiHandle,
}

impl WidgetKind {
    /// The snake_case name used in JSON and prompts.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Slider => "slider",
            Self::ControlPoints => "control_points",
            Self::MultiHandle => "multi_handle",
        }
    }
}

/// An editable property of an object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct Property {
    /// Stable key, for example `length`.
    pub key: String,
    /// Label shown to the user.
    pub label: String,
    /// Control used to edit it.
    pub widget: WidgetKind,
}

/// The editable properties suggested for one object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct PropertyList {
    /// Properties, most useful first.
    pub properties: Vec<Property>,
}

/// Which model each AI stage uses. Any OpenRouter model id is allowed, including
/// `typesafe/jev-router`, which picks a model and reasoning effort per request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ModelSettings {
    /// Suggests editable properties for an object.
    pub suggester: String,
    /// Plans where a control sits on the canvas.
    pub ui_planner: String,
    /// Summarises edits into instructions and change logs.
    pub summariser: String,
    /// Generates the edited image.
    pub generator: String,
    /// Tried when the stage's own model fails or is unavailable.
    pub fallback: String,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            suggester: "google/gemini-3.1-flash-lite".into(),
            ui_planner: "google/gemini-3.1-flash-lite".into(),
            summariser: "google/gemini-3.1-flash-lite".into(),
            generator: "google/gemini-3-pro-image".into(),
            fallback: "anthropic/claude-haiku-5.5".into(),
        }
    }
}

impl ModelSettings {
    /// Check every model id has a plausible shape.
    ///
    /// # Errors
    /// [`Error::Validation`] naming the offending field.
    pub fn validate(&self) -> Result<(), crate::Error> {
        for (field, value) in [
            ("suggester", &self.suggester),
            ("ui_planner", &self.ui_planner),
            ("summariser", &self.summariser),
            ("generator", &self.generator),
            ("fallback", &self.fallback),
        ] {
            let valid = !value.is_empty()
                && value.len() <= 100
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.:/~".contains(c));
            if !valid {
                return Err(crate::Error::Validation(format!(
                    "{field}: not a valid model id"
                )));
            }
        }
        Ok(())
    }
}

/// A line segment along which a slider moves.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct Axis {
    /// Where the slider starts.
    pub start: NormalizedPoint,
    /// Where the slider ends.
    pub end: NormalizedPoint,
}

/// Placement and geometry of a control on the canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Widget {
    /// A slider scaling a property between `min` and `max`.
    Slider {
        /// Line the handle moves along.
        axis: Axis,
        /// Lowest allowed value.
        min: f32,
        /// Highest allowed value.
        max: f32,
        /// Starting value.
        initial: f32,
    },
    /// Control points forming a shape.
    ControlPoints {
        /// Initial point positions.
        points: Vec<NormalizedPoint>,
    },
    /// Independent handles.
    MultiHandle {
        /// Initial handle positions.
        handles: Vec<NormalizedPoint>,
    },
}

impl Widget {
    /// The [`WidgetKind`] of this widget.
    #[must_use]
    pub fn kind(&self) -> WidgetKind {
        match self {
            Self::Slider { .. } => WidgetKind::Slider,
            Self::ControlPoints { .. } => WidgetKind::ControlPoints,
            Self::MultiHandle { .. } => WidgetKind::MultiHandle,
        }
    }
}

/// The change a user made with a widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EditChange {
    /// A slider moved to `value`.
    Slider {
        /// New value.
        value: f32,
    },
    /// Points moved from `origin` to `target`.
    ControlPoints {
        /// Positions before the edit.
        origin: Vec<NormalizedPoint>,
        /// Positions after the edit.
        target: Vec<NormalizedPoint>,
    },
    /// Handles moved from `origin` to `target`.
    MultiHandle {
        /// Positions before the edit.
        origin: Vec<NormalizedPoint>,
        /// Positions after the edit.
        target: Vec<NormalizedPoint>,
    },
}

/// One user edit on one object property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct Edit {
    /// Object being edited.
    pub object_id: ObjectId,
    /// Property key being edited.
    pub property: String,
    /// What the user changed.
    pub change: EditChange,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> NormalizedPoint {
        NormalizedPoint::new(x, y).unwrap()
    }

    #[test]
    fn widget_serialises_with_a_kind_tag() {
        let widget = Widget::ControlPoints {
            points: vec![point(0.1, 0.2)],
        };
        let json = serde_json::to_value(&widget).unwrap();
        assert_eq!(json["kind"], "control_points");
    }

    #[test]
    fn widget_round_trips_and_reports_its_kind() {
        let widget = Widget::Slider {
            axis: Axis {
                start: point(0.0, 0.5),
                end: point(1.0, 0.5),
            },
            min: 0.5,
            max: 1.5,
            initial: 1.0,
        };
        let back: Widget = serde_json::from_str(&serde_json::to_string(&widget).unwrap()).unwrap();
        assert_eq!(back, widget);
        assert_eq!(back.kind(), WidgetKind::Slider);
    }

    #[test]
    fn edit_rejects_out_of_range_points() {
        let json = r#"{"object_id":1,"property":"shape","change":{"kind":"control_points",
            "origin":[{"x":0.1,"y":0.1}],"target":[{"x":3.0,"y":0.1}]}}"#;
        assert!(serde_json::from_str::<Edit>(json).is_err());
    }

    #[test]
    fn edit_round_trips() {
        let edit = Edit {
            object_id: ObjectId(3),
            property: "length".into(),
            change: EditChange::Slider { value: 0.55 },
        };
        let back: Edit = serde_json::from_str(&serde_json::to_string(&edit).unwrap()).unwrap();
        assert_eq!(back, edit);
    }

    #[test]
    fn default_model_settings_are_valid() {
        assert!(ModelSettings::default().validate().is_ok());
    }

    #[test]
    fn model_settings_reject_bad_ids() {
        for bad in ["", "has space", "semi;colon", &"x".repeat(101)] {
            let settings = ModelSettings {
                suggester: bad.into(),
                ..ModelSettings::default()
            };
            assert!(settings.validate().is_err(), "{bad:?}");
        }
        let jev = ModelSettings {
            suggester: "typesafe/jev-router".into(),
            ..ModelSettings::default()
        };
        assert!(jev.validate().is_ok());
    }

    #[test]
    fn image_format_names_round_trip() {
        for format in [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Webp] {
            assert_eq!(ImageFormat::from_name(format.as_str()), Some(format));
        }
        assert_eq!(ImageFormat::from_name("gif"), None);
    }
}
