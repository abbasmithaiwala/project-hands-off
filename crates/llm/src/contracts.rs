//! What the models are asked for, and how their answers are validated.
//!
//! Models propose; this module disposes. Every answer is parsed, checked and repaired or
//! rejected here before the rest of the application sees it.

use std::collections::HashSet;

use hands_off_core::NormalizedPoint;
use hands_off_core::model::{Axis, Property, Widget, WidgetKind};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::schema::strict_schema;

/// Schema identifier for property suggestions.
pub const SUGGESTIONS_SCHEMA: &str = "property_suggestions";
/// Schema identifier for widget plans.
pub const WIDGET_SCHEMA: &str = "widget_plan";
/// Bump when the suggester prompt or schema changes, to invalidate cached results.
pub const SUGGESTER_PROMPT_VERSION: &str = "v1";
/// Bump when the planner prompt or schema changes, to invalidate cached results.
pub const PLANNER_PROMPT_VERSION: &str = "v1";
/// System prompt for the suggester.
pub const SUGGESTER_SYSTEM: &str = include_str!("../../../prompts/suggester.v1.md");
/// System prompt for the widget planner.
pub const PLANNER_SYSTEM: &str = include_str!("../../../prompts/planner.v1.md");

const MAX_PROPERTIES: usize = 6;
const MAX_KEY_LEN: usize = 32;
const MAX_LABEL_CHARS: usize = 40;
const MIN_SLIDER_SPAN: f32 = 0.02;
/// Control points must cover at least this much of the crop in each direction.
const MIN_POINT_SPREAD: f32 = 0.12;
/// Handles must be at least this far apart (crop units).
const MIN_HANDLE_DISTANCE: f32 = 0.04;
const CONTROL_POINTS: std::ops::RangeInclusive<usize> = 3..=16;
const HANDLES: std::ops::RangeInclusive<usize> = 1..=8;

/// One suggested property, as the model writes it.
#[derive(Deserialize, JsonSchema)]
struct PropertyDraft {
    /// snake_case identifier, at most 32 characters.
    key: String,
    /// Short label shown to the user.
    label: String,
    /// The control that suits the property.
    widget: WidgetKind,
}

/// The suggester's answer.
#[derive(Deserialize, JsonSchema)]
struct SuggestionDraft {
    /// Three to six properties, most useful first.
    properties: Vec<PropertyDraft>,
}

/// A point in crop coordinates, as the model writes it.
#[derive(Deserialize, JsonSchema)]
struct PointDraft {
    /// Horizontal position within the crop, 0 left to 1 right.
    x: f32,
    /// Vertical position within the crop, 0 top to 1 bottom.
    y: f32,
}

/// Slider line, as the model writes it.
#[derive(Deserialize, JsonSchema)]
struct AxisDraft {
    /// Where the handle starts.
    start: PointDraft,
    /// Where the handle ends.
    end: PointDraft,
}

/// The planner's answer. Only the fields for `kind` are filled; the rest are null.
#[derive(Deserialize, JsonSchema)]
struct WidgetDraft {
    /// The widget kind that was requested.
    kind: WidgetKind,
    /// Slider only: the line the handle moves along.
    axis: Option<AxisDraft>,
    /// Slider only: lowest scale multiplier.
    min: Option<f32>,
    /// Slider only: highest scale multiplier.
    max: Option<f32>,
    /// Slider only: starting multiplier.
    initial: Option<f32>,
    /// Control points only: points around the boundary.
    points: Option<Vec<PointDraft>>,
    /// Multi handle only: independent handle positions.
    handles: Option<Vec<PointDraft>>,
}

/// The JSON Schema the suggester must satisfy.
#[must_use]
pub fn suggestion_schema() -> Value {
    strict_schema::<SuggestionDraft>()
}

/// The JSON Schema the widget planner must satisfy.
#[must_use]
pub fn widget_schema() -> Value {
    strict_schema::<WidgetDraft>()
}

/// User message for the suggester.
#[must_use]
pub fn suggester_user_text(label: &str) -> String {
    format!("Object: {label}\nSuggest the editable properties.")
}

/// User message for the widget planner. `object_box` is the object's bounding box within the
/// crop as `[left, top, right, bottom]`, so the model knows where to look.
#[must_use]
pub fn planner_user_text(object_label: &str, property: &Property, object_box: [f32; 4]) -> String {
    format!(
        "Object: {object_label}\nProperty: {} ({})\nWidget: {}\nObject box in the crop: x from {:.2} to {:.2}, y from {:.2} to {:.2}",
        property.key,
        property.label,
        property.widget.as_str(),
        object_box[0],
        object_box[2],
        object_box[1],
        object_box[3],
    )
}

/// Parse and sanitise a suggestion answer.
///
/// Invalid entries are dropped; the answer is rejected only if nothing usable remains.
///
/// # Errors
/// A human-readable description of the problem, suitable for a repair request.
pub fn parse_suggestions(content: &str) -> Result<Vec<Property>, String> {
    let draft: SuggestionDraft = serde_json::from_str(content)
        .map_err(|e| format!("the JSON did not match the schema ({e})"))?;

    let mut seen = HashSet::new();
    let properties: Vec<Property> = draft
        .properties
        .into_iter()
        .filter_map(|p| {
            let key = p.key.trim().to_ascii_lowercase().replace([' ', '-'], "_");
            let label = p.label.trim().to_owned();
            let valid_key = !key.is_empty()
                && key.len() <= MAX_KEY_LEN
                && key.starts_with(|c: char| c.is_ascii_lowercase())
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            let valid_label = !label.is_empty() && label.chars().count() <= MAX_LABEL_CHARS;
            (valid_key && valid_label && seen.insert(key.clone())).then_some(Property {
                key,
                label,
                widget: p.widget,
            })
        })
        .take(MAX_PROPERTIES)
        .collect();

    if properties.is_empty() {
        Err("no valid properties: each needs a snake_case key, a label of at most 40 characters and a widget".into())
    } else {
        Ok(properties)
    }
}

fn check_spread(points: &[PointDraft]) -> Result<(), String> {
    let range = |value: fn(&PointDraft) -> f32| {
        let (min, max) = points
            .iter()
            .map(value)
            .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
        max - min
    };
    if range(|p| p.x) < MIN_POINT_SPREAD || range(|p| p.y) < MIN_POINT_SPREAD {
        return Err(
            "the points are bunched together; spread them around the part being reshaped".into(),
        );
    }
    Ok(())
}

fn check_distinct(points: &[PointDraft]) -> Result<(), String> {
    for (i, a) in points.iter().enumerate() {
        if points[i + 1..]
            .iter()
            .any(|b| (a.x - b.x).hypot(a.y - b.y) < MIN_HANDLE_DISTANCE)
        {
            return Err(
                "two handles are almost on top of each other; each must sit on a different part"
                    .into(),
            );
        }
    }
    Ok(())
}

/// Maps a point from crop coordinates into image coordinates, clamping to the image.
pub type Place<'a> = &'a dyn Fn(f32, f32) -> NormalizedPoint;
/// Moves a point onto the object if it lies off it.
pub type Snap<'a> = &'a dyn Fn(NormalizedPoint) -> NormalizedPoint;

/// Parse and validate a widget plan for the `expected` kind.
///
/// `place` converts the model's crop-relative coordinates to image coordinates and `snap`
/// pulls control points and handles onto the object's mask.
///
/// # Errors
/// A human-readable description of the problem, suitable for a repair request.
pub fn parse_widget(
    content: &str,
    expected: WidgetKind,
    place: Place<'_>,
    snap: Snap<'_>,
) -> Result<Widget, String> {
    let draft: WidgetDraft = serde_json::from_str(content)
        .map_err(|e| format!("the JSON did not match the schema ({e})"))?;
    if draft.kind != expected {
        return Err(format!(
            "kind must be `{}`, not `{}`",
            expected.as_str(),
            draft.kind.as_str()
        ));
    }
    let finite = |p: &PointDraft| p.x.is_finite() && p.y.is_finite();
    let convert = |points: Vec<PointDraft>, field: &str| -> Result<Vec<NormalizedPoint>, String> {
        if points.iter().any(|p| !finite(p)) {
            return Err(format!(
                "`{field}` contains a coordinate that is not a number"
            ));
        }
        Ok(points.iter().map(|p| snap(place(p.x, p.y))).collect())
    };

    match expected {
        WidgetKind::Slider => {
            let axis = draft.axis.ok_or("`axis` is required for a slider")?;
            let (min, max, initial) = (
                draft.min.ok_or("`min` is required for a slider")?,
                draft.max.ok_or("`max` is required for a slider")?,
                draft.initial.ok_or("`initial` is required for a slider")?,
            );
            if ![min, max, initial].iter().all(|v| v.is_finite()) || min <= 0.0 || min >= max {
                return Err("`min` must be positive and below `max`".into());
            }
            if initial < min || initial > max {
                return Err("`initial` must lie between `min` and `max`".into());
            }
            if !finite(&axis.start) || !finite(&axis.end) {
                return Err("`axis` contains a coordinate that is not a number".into());
            }
            let (start, end) = (
                place(axis.start.x, axis.start.y),
                place(axis.end.x, axis.end.y),
            );
            if (start.x - end.x).hypot(start.y - end.y) < MIN_SLIDER_SPAN {
                return Err("`axis` is too short to drag".into());
            }
            Ok(Widget::Slider {
                axis: Axis { start, end },
                min,
                max,
                initial,
            })
        }
        WidgetKind::ControlPoints => {
            let raw = draft
                .points
                .ok_or("`points` is required for control_points")?;
            check_spread(&raw)?;
            let points = convert(raw, "points")?;
            if CONTROL_POINTS.contains(&points.len()) {
                Ok(Widget::ControlPoints { points })
            } else {
                Err(format!(
                    "`points` needs {} to {} points",
                    CONTROL_POINTS.start(),
                    CONTROL_POINTS.end()
                ))
            }
        }
        WidgetKind::MultiHandle => {
            let raw = draft
                .handles
                .ok_or("`handles` is required for multi_handle")?;
            check_distinct(&raw)?;
            let handles = convert(raw, "handles")?;
            if HANDLES.contains(&handles.len()) {
                Ok(Widget::MultiHandle { handles })
            } else {
                Err(format!(
                    "`handles` needs {} to {} handles",
                    HANDLES.start(),
                    HANDLES.end()
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn identity(x: f32, y: f32) -> NormalizedPoint {
        NormalizedPoint::new(x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)).unwrap()
    }

    fn keep(p: NormalizedPoint) -> NormalizedPoint {
        p
    }

    fn suggestions(value: &Value) -> Result<Vec<Property>, String> {
        parse_suggestions(&value.to_string())
    }

    fn prop(key: &str, label: &str, widget: &str) -> Value {
        json!({ "key": key, "label": label, "widget": widget })
    }

    #[test]
    fn accepts_valid_suggestions_in_order() {
        let found = suggestions(&json!({ "properties": [
            prop("length", "Couch length", "slider"),
            prop("shape", "Seat shape", "control_points"),
        ]}))
        .unwrap();
        assert_eq!(
            found.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(),
            ["length", "shape"]
        );
        assert_eq!(found[1].widget, WidgetKind::ControlPoints);
    }

    #[test]
    fn normalises_keys_and_trims_labels() {
        let found = suggestions(
            &json!({ "properties": [prop(" Arm Position ", "  Arms  ", "multi_handle")] }),
        )
        .unwrap();
        assert_eq!(
            (found[0].key.as_str(), found[0].label.as_str()),
            ("arm_position", "Arms")
        );
    }

    #[test]
    fn drops_invalid_duplicate_and_excess_entries_but_keeps_the_rest() {
        let mut entries = vec![
            prop("ok", "Fine", "slider"),
            prop("ok", "Duplicate", "slider"),
            prop("9starts_with_digit", "Bad key", "slider"),
            prop("", "Empty key", "slider"),
            prop("no_label", "", "slider"),
            prop("long_label", &"x".repeat(41), "slider"),
            prop(&"k".repeat(33), "Long key", "slider"),
        ];
        entries.extend((0..8).map(|i| prop(&format!("p{i}"), "Extra", "slider")));
        let found = suggestions(&json!({ "properties": entries })).unwrap();
        assert_eq!(found.len(), MAX_PROPERTIES);
        assert_eq!(found[0].key, "ok");
        assert_eq!(found.iter().filter(|p| p.key == "ok").count(), 1);
    }

    #[test]
    fn rejects_answers_with_nothing_usable_or_the_wrong_shape() {
        assert!(suggestions(&json!({ "properties": [] })).is_err());
        assert!(suggestions(&json!({ "properties": [prop("", "x", "slider")] })).is_err());
        assert!(parse_suggestions("not json").is_err());
        assert!(suggestions(&json!({ "properties": [prop("a", "A", "dial")] })).is_err());
    }

    fn widget(value: &Value, kind: WidgetKind) -> Result<Widget, String> {
        parse_widget(&value.to_string(), kind, &identity, &keep)
    }

    fn slider_json() -> Value {
        json!({ "kind": "slider", "axis": { "start": {"x": 0.1, "y": 0.5}, "end": {"x": 0.9, "y": 0.5} },
                "min": 0.5, "max": 1.5, "initial": 1.0, "points": null, "handles": null })
    }

    #[test]
    fn parses_each_widget_kind() {
        let slider = widget(&slider_json(), WidgetKind::Slider).unwrap();
        assert!(
            matches!(slider, Widget::Slider { min, max, initial, .. } if (min, max, initial) == (0.5, 1.5, 1.0))
        );

        let points: Vec<Value> = (0..6)
            .map(|i| json!({"x": 0.1 * f64::from(i), "y": 0.2 + 0.3 * f64::from(i % 2)}))
            .collect();
        let control = widget(&json!({ "kind": "control_points", "axis": null, "min": null, "max": null, "initial": null, "points": points, "handles": null }), WidgetKind::ControlPoints).unwrap();
        assert!(matches!(control, Widget::ControlPoints { points } if points.len() == 6));

        let multi = widget(&json!({ "kind": "multi_handle", "axis": null, "min": null, "max": null, "initial": null, "points": null, "handles": [{"x": 0.2, "y": 0.2}, {"x": 0.8, "y": 0.2}] }), WidgetKind::MultiHandle).unwrap();
        assert!(matches!(multi, Widget::MultiHandle { handles } if handles.len() == 2));
    }

    #[test]
    fn rejects_a_kind_different_from_the_one_requested() {
        let error = widget(&slider_json(), WidgetKind::ControlPoints).unwrap_err();
        assert!(error.contains("control_points"), "{error}");
    }

    #[test]
    fn rejects_bad_slider_values() {
        for (field, value) in [
            ("min", json!(1.5)),
            ("min", json!(0.0)),
            ("max", json!(0.4)),
            ("initial", json!(2.0)),
            ("initial", json!(0.1)),
        ] {
            let mut json = slider_json();
            json[field] = value;
            assert!(widget(&json, WidgetKind::Slider).is_err(), "{field}");
        }
        let mut missing = slider_json();
        missing["axis"] = Value::Null;
        assert!(
            widget(&missing, WidgetKind::Slider)
                .unwrap_err()
                .contains("axis")
        );
        let mut short = slider_json();
        short["axis"] = json!({ "start": {"x": 0.5, "y": 0.5}, "end": {"x": 0.505, "y": 0.5} });
        assert!(
            widget(&short, WidgetKind::Slider)
                .unwrap_err()
                .contains("short")
        );
    }

    #[test]
    fn rejects_too_few_or_too_many_points() {
        let few = json!({ "kind": "control_points", "axis": null, "min": null, "max": null, "initial": null, "points": [{"x": 0.1, "y": 0.1}], "handles": null });
        assert!(widget(&few, WidgetKind::ControlPoints).is_err());
        let many: Vec<Value> = (0..9).map(|_| json!({"x": 0.5, "y": 0.5})).collect();
        let too_many = json!({ "kind": "multi_handle", "axis": null, "min": null, "max": null, "initial": null, "points": null, "handles": many });
        assert!(widget(&too_many, WidgetKind::MultiHandle).is_err());
    }

    #[test]
    fn maps_and_snaps_points_but_not_the_slider_axis() {
        let place = |x: f32, y: f32| NormalizedPoint::new(0.25 + x * 0.5, 0.25 + y * 0.5).unwrap();
        let snap = |_: NormalizedPoint| NormalizedPoint::new(0.5, 0.5).unwrap();
        let handles = json!({ "kind": "multi_handle", "axis": null, "min": null, "max": null, "initial": null, "points": null, "handles": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}] });
        let result =
            parse_widget(&handles.to_string(), WidgetKind::MultiHandle, &place, &snap).unwrap();
        assert!(
            matches!(&result, Widget::MultiHandle { handles } if handles.iter().all(|p| (p.x, p.y) == (0.5, 0.5)))
        );

        let slider = parse_widget(
            &slider_json().to_string(),
            WidgetKind::Slider,
            &place,
            &snap,
        )
        .unwrap();
        let Widget::Slider { axis, .. } = slider else {
            panic!("not a slider")
        };
        assert!(
            (axis.start.x - 0.30).abs() < 1e-6,
            "axis is mapped but not snapped"
        );
    }

    #[test]
    fn rejects_bunched_control_points_and_overlapping_handles() {
        let clustered: Vec<Value> = (0..6)
            .map(|i| json!({"x": 0.8 + 0.005 * f64::from(i), "y": 0.9}))
            .collect();
        let bunched = json!({ "kind": "control_points", "axis": null, "min": null, "max": null, "initial": null, "points": clustered, "handles": null });
        assert!(
            widget(&bunched, WidgetKind::ControlPoints)
                .unwrap_err()
                .contains("bunched")
        );

        let overlapping = json!({ "kind": "multi_handle", "axis": null, "min": null, "max": null, "initial": null, "points": null, "handles": [{"x": 0.5, "y": 0.5}, {"x": 0.51, "y": 0.5}] });
        assert!(
            widget(&overlapping, WidgetKind::MultiHandle)
                .unwrap_err()
                .contains("on top of each other")
        );
    }

    #[test]
    fn rejects_non_finite_coordinates_in_json_that_slipped_through() {
        let mut json = slider_json();
        json["axis"]["start"]["x"] = Value::Null;
        assert!(widget(&json, WidgetKind::Slider).is_err());
    }

    #[test]
    fn schemas_are_strict_and_name_every_field() {
        for schema in [suggestion_schema(), widget_schema()] {
            assert_eq!(schema["additionalProperties"], false);
            assert!(!schema.to_string().contains("$ref"));
        }
        let required = widget_schema()["required"].clone();
        for field in ["kind", "axis", "min", "max", "initial", "points", "handles"] {
            assert!(
                required.as_array().unwrap().contains(&json!(field)),
                "{field}"
            );
        }
    }

    #[test]
    fn user_texts_carry_the_label_and_property() {
        assert!(suggester_user_text("couch").contains("couch"));
        let p = Property {
            key: "length".into(),
            label: "Couch length".into(),
            widget: WidgetKind::Slider,
        };
        let text = planner_user_text("couch", &p, [0.1, 0.2, 0.9, 0.8]);
        assert!(
            text.contains("Property: length (Couch length)") && text.contains("Widget: slider")
        );
    }

    #[test]
    fn prompts_are_embedded() {
        assert!(
            SUGGESTER_SYSTEM.contains("control_points") && PLANNER_SYSTEM.contains("normalised")
        );
    }
}
