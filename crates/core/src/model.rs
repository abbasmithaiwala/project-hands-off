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
    /// Project name shared by every version of the same photo.
    pub name: Option<String>,
    /// The version this one was edited from; absent for an original upload.
    pub parent_id: Option<ImageId>,
    /// What produced this version from its parent; absent for an original upload.
    pub edit: Option<EditResult>,
}

/// Longest accepted project name, in characters.
pub const MAX_NAME_CHARS: usize = 80;

/// Trim a user-supplied project name and check it is non-empty, short and printable.
///
/// # Errors
/// [`crate::Error::Validation`] when the name is empty, too long or has control characters.
pub fn normalize_name(raw: &str) -> crate::Result<String> {
    let name = raw.trim();
    if name.is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err(crate::Error::Validation(format!(
            "name must be 1 to {MAX_NAME_CHARS} printable characters"
        )));
    }
    Ok(name.to_owned())
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
    /// A handle on a ring that turns the object about a pivot.
    Rotate,
    /// A corner handle that resizes the whole object about a fixed anchor.
    Scale,
    /// A single handle that carries the whole object to a new place.
    Move,
}

impl WidgetKind {
    /// The snake_case name used in JSON and prompts.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Slider => "slider",
            Self::ControlPoints => "control_points",
            Self::MultiHandle => "multi_handle",
            Self::Rotate => "rotate",
            Self::Scale => "scale",
            Self::Move => "move",
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
    /// A faster, cheaper generator offered as a draft option when retrying.
    pub draft_generator: String,
    /// Tried when the stage's own model fails or is unavailable.
    pub fallback: String,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            suggester: "deepseek/deepseek-v4.1-flash".into(),
            ui_planner: "deepseek/deepseek-v4.1-flash".into(),
            summariser: "deepseek/deepseek-v4.1-flash".into(),
            generator: "google/gemini-3-pro-image".into(),
            draft_generator: "google/gemini-3-pro-image".into(),
            fallback: "anthropic/claude-haiku-5.5".into(),
        }
    }
}

/// Whether `value` looks like an OpenRouter model id: short, and made of safe characters.
#[must_use]
pub fn is_valid_model_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.:/~".contains(c))
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
            ("draft_generator", &self.draft_generator),
            ("fallback", &self.fallback),
        ] {
            let valid = is_valid_model_id(value);
            if !valid {
                return Err(crate::Error::Validation(format!(
                    "{field}: not a valid model id"
                )));
            }
        }
        Ok(())
    }
}

/// What the reference image sent with an edit shows.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    /// A rough render of the object moved to its new place, to be reproduced.
    #[default]
    Warped,
    /// The original photo with the object's new silhouette drawn over it as a line.
    Outline,
}

/// A request to apply a batch of edits to an image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct EditRequest {
    /// The edits, applied together in one generation.
    pub edits: Vec<Edit>,
    /// For each affected area of the image, the box it covers before and after the edits.
    /// Only pixels inside these boxes may change. When empty, the edited objects' boxes are used.
    #[serde(default)]
    pub regions: Vec<BBox>,
    /// Base64 PNG used as a geometry reference for the model; see `preview_kind`.
    #[serde(default)]
    pub preview_png: Option<String>,
    /// What `preview_png` shows. Defaults to a warped render of the edited objects.
    #[serde(default)]
    pub preview_kind: PreviewKind,
    /// Image model to use instead of the configured generator.
    #[serde(default)]
    pub model: Option<String>,
}

/// One line of the change log for an applied edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct ChangeSummary {
    /// Index into the request's `edits`.
    pub edit_index: u32,
    /// Plain-language description, for example "Shortened the couch to about 60% of its length".
    pub text: String,
}

/// The outcome of a successful edit job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct EditResult {
    /// One-line summary of everything that changed.
    pub summary: String,
    /// One entry per edit.
    pub changes: Vec<ChangeSummary>,
    /// The instruction that was sent to the image model.
    pub instruction: String,
    /// The image model that produced the result.
    pub model: String,
    /// Cost of the image generation in US dollars, when reported.
    pub cost_usd: Option<f64>,
    /// How long the image generation took.
    pub generation_ms: u64,
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
    /// A handle on a ring around `pivot`; dragging it around the ring turns the object.
    Rotate {
        /// The point the object turns about, which stays fixed.
        pivot: NormalizedPoint,
        /// Where the handle rests. Its distance from `pivot` is the ring's radius and its
        /// direction is the zero angle.
        handle: NormalizedPoint,
        /// Furthest anticlockwise turn, in degrees (negative).
        min: f32,
        /// Furthest clockwise turn, in degrees (positive).
        max: f32,
    },
    /// A handle that resizes the whole object evenly about a fixed anchor.
    Scale {
        /// `start` is the anchor that stays fixed; `end` is where the handle rests.
        axis: Axis,
        /// Smallest allowed multiplier.
        min: f32,
        /// Largest allowed multiplier.
        max: f32,
        /// Starting multiplier, normally `1.0`.
        initial: f32,
    },
    /// A handle that moves the whole object without changing its shape.
    Move {
        /// Where the handle rests, on the object.
        handle: NormalizedPoint,
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
            Self::Rotate { .. } => WidgetKind::Rotate,
            Self::Scale { .. } => WidgetKind::Scale,
            Self::Move { .. } => WidgetKind::Move,
        }
    }
}

/// The change a user made with a widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EditChange {
    /// A slider moved to `value`.
    Slider {
        /// New value, as a multiplier (`1.0` is unchanged).
        value: f32,
        /// The line the handle moved along. Its start stays fixed, which lets the edit be
        /// described in words ("keeping the left end in place").
        #[serde(default)]
        axis: Option<Axis>,
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
    /// The object turned about `pivot`.
    Rotate {
        /// Clockwise turn in degrees as seen in the photo; negative is anticlockwise.
        degrees: f32,
        /// The point the object turned about, so the edit can be put in words.
        pivot: NormalizedPoint,
    },
    /// The whole object resized evenly about `pivot`.
    Scale {
        /// New size as a multiplier (`1.0` is unchanged).
        value: f32,
        /// The anchor that stayed fixed, so the edit can be put in words.
        pivot: NormalizedPoint,
    },
    /// The whole object moved, as a share of the image's width and height.
    Move {
        /// Rightward shift as a fraction of the image width; negative is leftward.
        dx: f32,
        /// Downward shift as a fraction of the image height; negative is upward.
        dy: f32,
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
    fn transform_widgets_and_edits_round_trip_with_their_kind_names() {
        let widgets = [
            (
                Widget::Rotate {
                    pivot: point(0.5, 0.9),
                    handle: point(0.5, 0.2),
                    min: -45.0,
                    max: 45.0,
                },
                WidgetKind::Rotate,
                "rotate",
            ),
            (
                Widget::Scale {
                    axis: Axis {
                        start: point(0.5, 0.5),
                        end: point(0.8, 0.2),
                    },
                    min: 0.5,
                    max: 2.0,
                    initial: 1.0,
                },
                WidgetKind::Scale,
                "scale",
            ),
            (
                Widget::Move {
                    handle: point(0.5, 0.5),
                },
                WidgetKind::Move,
                "move",
            ),
        ];
        for (widget, kind, name) in widgets {
            let json = serde_json::to_value(&widget).unwrap();
            assert_eq!(json["kind"], name);
            assert_eq!(widget.kind(), kind);
            assert_eq!(kind.as_str(), name);
            assert_eq!(serde_json::from_value::<Widget>(json).unwrap(), widget);
        }

        let changes = [
            EditChange::Rotate {
                degrees: -12.5,
                pivot: point(0.5, 0.9),
            },
            EditChange::Scale {
                value: 1.3,
                pivot: point(0.5, 0.5),
            },
            EditChange::Move { dx: 0.1, dy: -0.05 },
        ];
        for change in changes {
            let back: EditChange =
                serde_json::from_str(&serde_json::to_string(&change).unwrap()).unwrap();
            assert_eq!(back, change);
        }
    }

    #[test]
    fn a_rotation_needs_its_pivot() {
        let json = r#"{"kind":"rotate","degrees":10.0}"#;
        assert!(serde_json::from_str::<EditChange>(json).is_err());
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
            change: EditChange::Slider {
                value: 0.55,
                axis: None,
            },
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
    fn edit_requests_default_their_optional_fields() {
        let request: EditRequest = serde_json::from_str(r#"{"edits":[]}"#).unwrap();
        assert!(
            request.regions.is_empty()
                && request.preview_png.is_none()
                && request.preview_kind == PreviewKind::Warped
                && request.model.is_none()
        );
    }

    #[test]
    fn image_format_names_round_trip() {
        for format in [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Webp] {
            assert_eq!(ImageFormat::from_name(format.as_str()), Some(format));
        }
        assert_eq!(ImageFormat::from_name("gif"), None);
    }
}
