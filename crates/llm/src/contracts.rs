//! What the models are asked for, and how their answers are validated.
//!
//! Models propose; this module disposes. Every answer is parsed, checked and repaired or
//! rejected here before the rest of the application sees it.

use std::collections::HashSet;
use std::fmt::Write;

use hands_off_core::NormalizedPoint;
use hands_off_core::model::{
    Axis, ChangeSummary, EditChange, PreviewKind, Property, Widget, WidgetKind,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::schema::strict_schema;

/// Schema identifier for property suggestions.
pub const SUGGESTIONS_SCHEMA: &str = "property_suggestions";
/// Schema identifier for widget plans.
pub const WIDGET_SCHEMA: &str = "widget_plan";
/// Bump when the suggester prompt or schema changes, to invalidate cached results.
pub const SUGGESTER_PROMPT_VERSION: &str = "v2";
/// Bump when the planner prompt or schema changes, to invalidate cached results.
pub const PLANNER_PROMPT_VERSION: &str = "v2";
/// Schema identifier for edit summaries.
pub const SUMMARY_SCHEMA: &str = "edit_summary";
/// Bump when the summariser prompt or schema changes.
pub const SUMMARISER_PROMPT_VERSION: &str = "v4";
/// System prompt for the summariser.
pub const SUMMARISER_SYSTEM: &str = include_str!("../../../prompts/summariser.v4.md");
const GENERATOR_WITH_OUTLINE: &str = include_str!("../../../prompts/generator-with-outline.v3.md");
const GENERATOR_WITH_PREVIEW: &str = include_str!("../../../prompts/generator-with-preview.v3.md");
const GENERATOR_WITH_ROTATION: &str =
    include_str!("../../../prompts/generator-with-rotation.v1.md");
const GENERATOR_WITHOUT_PREVIEW: &str =
    include_str!("../../../prompts/generator-without-preview.v3.md");

/// System prompt for the suggester.
pub const SUGGESTER_SYSTEM: &str = include_str!("../../../prompts/suggester.v2.md");
/// System prompt for the widget planner.
pub const PLANNER_SYSTEM: &str = include_str!("../../../prompts/planner.v2.md");

const MAX_PROPERTIES: usize = 6;
const MAX_KEY_LEN: usize = 32;
const MAX_LABEL_CHARS: usize = 40;
const MIN_SLIDER_SPAN: f32 = 0.02;
/// Control points must cover at least this much of the crop in each direction.
const MIN_POINT_SPREAD: f32 = 0.12;
/// Handles must be at least this far apart (crop units).
const MIN_HANDLE_DISTANCE: f32 = 0.04;
/// The ring of a rotate control must be at least this big (crop units) to be grabbable.
const MIN_ROTATE_RADIUS: f32 = 0.04;
/// A rotate control must offer at least this much total turn, in degrees.
const MIN_ROTATE_SPAN: f32 = 10.0;
const MAX_ROTATE_DEGREES: f32 = 180.0;
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
    /// Rotate only: the point the object turns about.
    pivot: Option<PointDraft>,
    /// Rotate and move only: where the handle rests.
    handle: Option<PointDraft>,
    /// Slider and scale: lowest multiplier. Rotate: furthest anticlockwise turn in degrees (negative).
    min: Option<f32>,
    /// Slider and scale: highest multiplier. Rotate: furthest clockwise turn in degrees (positive).
    max: Option<f32>,
    /// Slider and scale: starting multiplier. Rotate and move: null.
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

/// An edit described for the summariser: who changed what, and how.
#[derive(Debug, Clone, Copy)]
pub struct EditLine<'a> {
    /// Position in the request's edit list.
    pub index: usize,
    /// For example "couch".
    pub object_label: &'a str,
    /// For example "Couch length".
    pub property_label: &'a str,
    /// What the user changed.
    pub change: &'a EditChange,
}

/// Past this a turn is more than a lean, so "the top swings to one side" stops being true.
const MAX_SIDE_HINT_DEGREES: f32 = 90.0;
const MOVED_EPSILON: f32 = 1e-4;
const MAX_LISTED_MOVES: usize = 4;
const MAX_INSTRUCTION_CHARS: usize = 1500;
const MAX_SUMMARY_CHARS: usize = 200;
const MAX_CHANGE_CHARS: usize = 240;

fn percent(value: f32) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    {
        (value * 100.0).round() as i32
    }
}

fn turn_text(degrees: f32) -> String {
    #[allow(clippy::cast_possible_truncation)]
    let whole = degrees.abs().round() as i32;
    let direction = if degrees < 0.0 {
        "anticlockwise"
    } else {
        "clockwise"
    };
    format!("{whole} degrees {direction}")
}

fn shift_text(dx: f32, dy: f32) -> String {
    let part = |value: f32, positive: &str, negative: &str, of: &str| {
        let amount = percent(value.abs());
        (amount > 0).then(|| {
            let way = if value < 0.0 { negative } else { positive };
            format!("{way} by {amount}% of the image {of}")
        })
    };
    let parts: Vec<String> = [
        part(dx, "right", "left", "width"),
        part(dy, "down", "up", "height"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if parts.is_empty() {
        "left where it is".to_owned()
    } else {
        format!(
            "moved {}, with its shape and size unchanged",
            parts.join(" and ")
        )
    }
}

/// A precise, model-readable description of one edit, with numbers.
#[must_use]
pub fn describe_edit(line: &EditLine<'_>) -> String {
    let head = format!("{} of the {}", line.property_label, line.object_label);
    match line.change {
        EditChange::Slider { value, axis } => {
            let along = axis.map_or_else(String::new, |a| {
                format!(
                    " along the line from ({}% across, {}% down) to ({}% across, {}% down), keeping the start of that line fixed",
                    percent(a.start.x),
                    percent(a.start.y),
                    percent(a.end.x),
                    percent(a.end.y)
                )
            });
            format!(
                "{head}: scaled to {}% of its original size{along} (100% is unchanged)",
                percent(*value)
            )
        }
        EditChange::Scale { value, pivot } => format!(
            "{head}: resized evenly to {}% of its original size, keeping the point ({}% across, {}% down) where it is (100% is unchanged)",
            percent(*value),
            percent(pivot.x),
            percent(pivot.y)
        ),
        EditChange::Rotate { degrees, pivot } => format!(
            "{head}: turned {} as seen in the photo, about the point ({}% across, {}% down), which stays where it is",
            turn_text(*degrees),
            percent(pivot.x),
            percent(pivot.y)
        ),
        EditChange::Move { dx, dy } => format!("{head}: {}", shift_text(*dx, *dy)),
        EditChange::ControlPoints { origin, target }
        | EditChange::MultiHandle { origin, target } => {
            let moves: Vec<String> = origin
                .iter()
                .zip(target)
                .enumerate()
                .filter(|(_, (a, b))| {
                    (a.x - b.x).abs() > MOVED_EPSILON || (a.y - b.y).abs() > MOVED_EPSILON
                })
                .map(|(i, (a, b))| {
                    format!(
                        "point {} moved from ({}% across, {}% down) to ({}% across, {}% down)",
                        i + 1,
                        percent(a.x),
                        percent(a.y),
                        percent(b.x),
                        percent(b.y)
                    )
                })
                .collect();
            let extra = moves.len().saturating_sub(MAX_LISTED_MOVES);
            let mut listed = moves
                .into_iter()
                .take(MAX_LISTED_MOVES)
                .collect::<Vec<_>>()
                .join("; ");
            if extra > 0 {
                listed = format!("{listed}; and {extra} more");
            }
            format!("{head}: {listed}")
        }
    }
}

/// User message for the summariser.
#[must_use]
pub fn summariser_user_text(lines: &[EditLine<'_>]) -> String {
    let list: Vec<String> = lines
        .iter()
        .map(|l| format!("{}. {}: {}", l.index, l.object_label, describe_edit(l)))
        .collect();
    format!("Edits:\n{}", list.join("\n"))
}

/// The summariser's answer, as the model writes it.
#[derive(Deserialize, JsonSchema)]
struct SummaryDraft {
    /// One paragraph telling the image model what to change.
    instruction: String,
    /// At most 15 words for a history list.
    summary: String,
    /// One entry per numbered edit.
    changes: Vec<ChangeDraft>,
}

/// One change-log line, as the model writes it.
#[derive(Deserialize, JsonSchema)]
struct ChangeDraft {
    /// The number of the edit this describes.
    edit_index: u32,
    /// A short past-tense sentence.
    text: String,
}

/// The JSON Schema the summariser must satisfy.
#[must_use]
pub fn summary_schema() -> Value {
    strict_schema::<SummaryDraft>()
}

/// A validated summary of a batch of edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSummary {
    /// What the image model is asked to do.
    pub instruction: String,
    /// One line for a history list.
    pub summary: String,
    /// One entry per edit.
    pub changes: Vec<ChangeSummary>,
}

fn short_description(line: &EditLine<'_>) -> String {
    match line.change {
        EditChange::Slider { value, .. } | EditChange::Scale { value, .. } => {
            format!("{}%", percent(*value))
        }
        EditChange::Rotate { degrees, .. } => turn_text(*degrees),
        EditChange::Move { dx, dy } => {
            format!("moved {}% across, {}% down", percent(*dx), percent(*dy))
        }
        EditChange::ControlPoints { origin, target }
        | EditChange::MultiHandle { origin, target } => {
            let moved = origin
                .iter()
                .zip(target)
                .filter(|(a, b)| {
                    (a.x - b.x).abs() > MOVED_EPSILON || (a.y - b.y).abs() > MOVED_EPSILON
                })
                .count();
            if moved == 1 {
                "1 point moved".to_owned()
            } else {
                format!("{moved} points moved")
            }
        }
    }
}

fn template_change(line: &EditLine<'_>) -> ChangeSummary {
    ChangeSummary {
        edit_index: u32::try_from(line.index).unwrap_or(u32::MAX),
        text: format!(
            "Changed the {} of the {} ({})",
            line.property_label.to_lowercase(),
            line.object_label,
            short_description(line)
        ),
    }
}

/// A summary built from templates, used when no model can write one.
#[must_use]
pub fn template_summary(lines: &[EditLine<'_>]) -> EditSummary {
    let described: Vec<String> = lines.iter().map(describe_edit).collect();
    let changes: Vec<ChangeSummary> = lines.iter().map(template_change).collect();
    let summary = match lines {
        [only] => format!(
            "Changed the {} of the {}",
            only.property_label.to_lowercase(),
            only.object_label
        ),
        _ => format!("Made {} changes", lines.len()),
    };
    EditSummary {
        instruction: format!("Make these changes: {}.", described.join("; ")),
        summary,
        changes,
    }
}

/// Parse and validate a summary answer. Missing change-log lines are filled from templates;
/// everything else must be sound or the answer is rejected.
///
/// # Errors
/// A human-readable description of the problem, suitable for a repair request.
pub fn parse_edit_summary(content: &str, lines: &[EditLine<'_>]) -> Result<EditSummary, String> {
    let draft: SummaryDraft = serde_json::from_str(content)
        .map_err(|e| format!("the JSON did not match the schema ({e})"))?;
    let instruction = draft.instruction.trim().to_owned();
    if instruction.chars().count() < 10 || instruction.chars().count() > MAX_INSTRUCTION_CHARS {
        return Err(format!(
            "`instruction` must be between 10 and {MAX_INSTRUCTION_CHARS} characters"
        ));
    }
    let summary = draft.summary.trim().to_owned();
    if summary.is_empty() || summary.chars().count() > MAX_SUMMARY_CHARS {
        return Err(format!(
            "`summary` must be between 1 and {MAX_SUMMARY_CHARS} characters"
        ));
    }

    let mut changes: Vec<ChangeSummary> = Vec::new();
    for change in draft.changes {
        let known = lines
            .iter()
            .any(|l| u32::try_from(l.index) == Ok(change.edit_index));
        let text = change.text.trim().to_owned();
        let valid = known
            && !text.is_empty()
            && text.chars().count() <= MAX_CHANGE_CHARS
            && !changes.iter().any(|c| c.edit_index == change.edit_index);
        if valid {
            changes.push(ChangeSummary {
                edit_index: change.edit_index,
                text,
            });
        }
    }
    for line in lines {
        if !changes
            .iter()
            .any(|c| u32::try_from(line.index) == Ok(c.edit_index))
        {
            changes.push(template_change(line));
        }
    }
    changes.sort_by_key(|c| c.edit_index);
    Ok(EditSummary {
        instruction,
        summary,
        changes,
    })
}

/// The full text sent to the image model.
///
/// `reference` is what the second image shows, if one is sent. With an outline, the model is
/// told which `objects` to reshape and `instruction` is not used: the drawn line carries the
/// geometry, and numbers in words only pull it away from the picture.
#[must_use]
pub fn generation_instruction(
    instruction: &str,
    objects: &[&str],
    reference: Option<PreviewKind>,
) -> String {
    match reference {
        Some(PreviewKind::Outline) => {
            let names = match objects {
                [] => "object".to_owned(),
                [only] => (*only).to_owned(),
                [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
            };
            GENERATOR_WITH_OUTLINE.trim().replace("{objects}", &names)
        }
        Some(PreviewKind::Warped) => GENERATOR_WITH_PREVIEW
            .trim()
            .replace("{instruction}", instruction.trim()),
        None => GENERATOR_WITHOUT_PREVIEW
            .trim()
            .replace("{instruction}", instruction.trim()),
    }
}

/// The text sent to the image model when the only edit turns one object and the second image
/// shows it turned.
///
/// The direction is worked out from the angle here, not taken from the summariser's wording:
/// image models confuse clockwise with anticlockwise, so the side the top swings to is stated.
#[must_use]
pub fn rotation_instruction(object: &str, degrees: f32) -> String {
    let side = if degrees < 0.0 { "left" } else { "right" };
    let mut turn = turn_text(degrees);
    if degrees.abs() < MAX_SIDE_HINT_DEGREES {
        write!(
            turn,
            ", so the top of the {object} swings toward the {side} of the picture"
        )
        .ok();
    }
    GENERATOR_WITH_ROTATION
        .trim()
        .replace("{object}", object)
        .replace("{turn}", &turn)
}

/// Maps a point from crop coordinates into image coordinates, clamping to the image.
pub type Place<'a> = &'a dyn Fn(f32, f32) -> NormalizedPoint;
/// Moves a point onto the object if it lies off it.
pub type Snap<'a> = &'a dyn Fn(NormalizedPoint) -> NormalizedPoint;

/// The axis and multiplier range shared by sliders and scale handles.
fn scalar_range(
    draft: &WidgetDraft,
    name: &str,
    place: Place<'_>,
) -> Result<(Axis, f32, f32, f32), String> {
    let axis = draft
        .axis
        .as_ref()
        .ok_or_else(|| format!("`axis` is required for {name}"))?;
    let (min, max, initial) = (
        draft
            .min
            .ok_or_else(|| format!("`min` is required for {name}"))?,
        draft
            .max
            .ok_or_else(|| format!("`max` is required for {name}"))?,
        draft
            .initial
            .ok_or_else(|| format!("`initial` is required for {name}"))?,
    );
    if ![min, max, initial].iter().all(|v| v.is_finite()) || min <= 0.0 || min >= max {
        return Err("`min` must be positive and below `max`".into());
    }
    if initial < min || initial > max {
        return Err("`initial` must lie between `min` and `max`".into());
    }
    let finite = |p: &PointDraft| p.x.is_finite() && p.y.is_finite();
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
    Ok((Axis { start, end }, min, max, initial))
}

/// A rotate plan: a pivot, a handle on the ring around it, and the allowed turn.
fn parse_rotate(draft: &WidgetDraft, place: Place<'_>) -> Result<Widget, String> {
    let pivot = draft
        .pivot
        .as_ref()
        .ok_or("`pivot` is required for rotate")?;
    let handle = draft
        .handle
        .as_ref()
        .ok_or("`handle` is required for rotate")?;
    let (min, max) = (
        draft.min.ok_or("`min` is required for rotate")?,
        draft.max.ok_or("`max` is required for rotate")?,
    );
    if ![pivot.x, pivot.y, handle.x, handle.y]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("`pivot` or `handle` contains a coordinate that is not a number".into());
    }
    let valid_range = min.is_finite()
        && max.is_finite()
        && min <= 0.0
        && max >= 0.0
        && min >= -MAX_ROTATE_DEGREES
        && max <= MAX_ROTATE_DEGREES
        && max - min >= MIN_ROTATE_SPAN;
    if !valid_range {
        return Err(format!(
            "`min` must be at most 0 and `max` at least 0, within {MAX_ROTATE_DEGREES} degrees, spanning at least {MIN_ROTATE_SPAN} degrees"
        ));
    }
    let (pivot, handle) = (place(pivot.x, pivot.y), place(handle.x, handle.y));
    if (pivot.x - handle.x).hypot(pivot.y - handle.y) < MIN_ROTATE_RADIUS {
        return Err(
            "`handle` is too close to `pivot`; put it at the far end of the part that turns".into(),
        );
    }
    Ok(Widget::Rotate {
        pivot,
        handle,
        min,
        max,
    })
}

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
            let (axis, min, max, initial) = scalar_range(&draft, "slider", place)?;
            Ok(Widget::Slider {
                axis,
                min,
                max,
                initial,
            })
        }
        WidgetKind::Scale => {
            let (axis, min, max, initial) = scalar_range(&draft, "scale", place)?;
            Ok(Widget::Scale {
                axis,
                min,
                max,
                initial,
            })
        }
        WidgetKind::Rotate => parse_rotate(&draft, place),
        WidgetKind::Move => {
            let handle = draft.handle.ok_or("`handle` is required for move")?;
            let moved = convert(vec![handle], "handle")?;
            let handle = moved
                .first()
                .copied()
                .ok_or("`handle` is required for move")?;
            Ok(Widget::Move { handle })
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
    fn a_turn_names_the_side_the_top_swings_to() {
        let left = rotation_instruction("mug", -22.0);
        assert!(
            left.contains("22 degrees anticlockwise, so the top of the mug swings toward the left")
        );
        let right = rotation_instruction("mug", 15.0);
        assert!(
            right.contains("15 degrees clockwise, so the top of the mug swings toward the right")
        );
        let half = rotation_instruction("mug", 120.0);
        assert!(half.contains("120 degrees clockwise") && !half.contains("swings"));
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
        for field in [
            "kind", "axis", "pivot", "handle", "min", "max", "initial", "points", "handles",
        ] {
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

    fn slider_line(change: &EditChange, index: usize) -> EditLine<'_> {
        EditLine {
            index,
            object_label: "couch",
            property_label: "Couch length",
            change,
        }
    }

    fn moved_points(count: usize, shifted: usize) -> EditChange {
        let origin: Vec<NormalizedPoint> = (0..count)
            .map(|i| {
                NormalizedPoint::new(0.1 + 0.05 * f32::from(u8::try_from(i).unwrap()), 0.5).unwrap()
            })
            .collect();
        let target = origin
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i < shifted {
                    NormalizedPoint::new(p.x, 0.7).unwrap()
                } else {
                    *p
                }
            })
            .collect();
        EditChange::ControlPoints { origin, target }
    }

    #[test]
    fn describes_slider_edits_with_a_percentage() {
        let change = EditChange::Slider {
            axis: None,
            value: 0.604,
        };
        assert_eq!(
            describe_edit(&slider_line(&change, 0)),
            "Couch length of the couch: scaled to 60% of its original size (100% is unchanged)"
        );
    }

    #[test]
    fn describes_which_end_of_a_slider_line_stays_fixed() {
        let axis = Axis {
            start: NormalizedPoint::new(0.2, 0.7).unwrap(),
            end: NormalizedPoint::new(0.7, 0.7).unwrap(),
        };
        let change = EditChange::Slider {
            value: 0.55,
            axis: Some(axis),
        };
        let text = describe_edit(&slider_line(&change, 0));
        assert!(
            text.contains("along the line from (20% across, 70% down) to (70% across, 70% down), keeping the start of that line fixed"),
            "{text}"
        );
        assert!(text.ends_with("(100% is unchanged)"), "{text}");
    }

    fn rotate_json() -> Value {
        json!({ "kind": "rotate", "axis": null, "pivot": {"x": 0.5, "y": 0.9}, "handle": {"x": 0.5, "y": 0.1},
                "min": -45.0, "max": 45.0, "initial": null, "points": null, "handles": null })
    }

    #[test]
    fn parses_rotate_scale_and_move_widgets() {
        let rotate = widget(&rotate_json(), WidgetKind::Rotate).unwrap();
        assert!(matches!(
            rotate,
            Widget::Rotate { pivot, handle, min, max }
                if (pivot.y, handle.y, min, max) == (0.9, 0.1, -45.0, 45.0)
        ));

        let mut scale = slider_json();
        scale["kind"] = json!("scale");
        scale["max"] = json!(2.0);
        let scale = widget(&scale, WidgetKind::Scale).unwrap();
        assert!(matches!(scale, Widget::Scale { max, .. } if (max - 2.0).abs() < f32::EPSILON));

        let moved = widget(
            &json!({ "kind": "move", "axis": null, "pivot": null, "handle": {"x": 0.4, "y": 0.6},
                     "min": null, "max": null, "initial": null, "points": null, "handles": null }),
            WidgetKind::Move,
        )
        .unwrap();
        assert!(matches!(moved, Widget::Move { handle } if (handle.x, handle.y) == (0.4, 0.6)));
    }

    #[test]
    fn rejects_bad_rotate_plans() {
        for (field, value, expected) in [
            ("min", json!(5.0), "`min` must be at most 0"),
            ("max", json!(-5.0), "`max` at least 0"),
            ("min", json!(-200.0), "within 180"),
            ("max", json!(3.0), "spanning at least"),
        ] {
            let mut json = rotate_json();
            json[field] = value;
            if field == "max" && expected == "spanning at least" {
                json["min"] = json!(-3.0);
            }
            let error = widget(&json, WidgetKind::Rotate).unwrap_err();
            assert!(error.contains(expected), "{field}: {error}");
        }
        let mut no_pivot = rotate_json();
        no_pivot["pivot"] = Value::Null;
        assert!(
            widget(&no_pivot, WidgetKind::Rotate)
                .unwrap_err()
                .contains("pivot")
        );
        let mut cramped = rotate_json();
        cramped["handle"] = json!({"x": 0.51, "y": 0.9});
        assert!(
            widget(&cramped, WidgetKind::Rotate)
                .unwrap_err()
                .contains("too close")
        );
    }

    #[test]
    fn a_move_handle_is_snapped_onto_the_object() {
        let snap = |_: NormalizedPoint| NormalizedPoint::new(0.5, 0.5).unwrap();
        let moved = parse_widget(
            &json!({ "kind": "move", "handle": {"x": 0.0, "y": 0.0} }).to_string(),
            WidgetKind::Move,
            &identity,
            &snap,
        )
        .unwrap();
        assert!(matches!(moved, Widget::Move { handle } if (handle.x, handle.y) == (0.5, 0.5)));
        assert!(
            widget(&json!({ "kind": "move" }), WidgetKind::Move)
                .unwrap_err()
                .contains("handle")
        );
    }

    #[test]
    fn describes_rotation_scale_and_movement_in_words() {
        let pivot = NormalizedPoint::new(0.5, 0.9).unwrap();
        let turn = EditChange::Rotate {
            degrees: -14.6,
            pivot,
        };
        assert_eq!(
            describe_edit(&slider_line(&turn, 0)),
            "Couch length of the couch: turned 15 degrees anticlockwise as seen in the photo, about the point (50% across, 90% down), which stays where it is"
        );
        let clockwise = EditChange::Rotate {
            degrees: 30.0,
            pivot,
        };
        assert!(describe_edit(&slider_line(&clockwise, 0)).contains("30 degrees clockwise"));

        let scale = EditChange::Scale { value: 1.25, pivot };
        let text = describe_edit(&slider_line(&scale, 0));
        assert!(
            text.contains("resized evenly to 125%") && text.contains("(50% across, 90% down)"),
            "{text}"
        );

        let moved = EditChange::Move {
            dx: -0.12,
            dy: 0.05,
        };
        assert_eq!(
            describe_edit(&slider_line(&moved, 0)),
            "Couch length of the couch: moved left by 12% of the image width and down by 5% of the image height, with its shape and size unchanged"
        );
        let still = EditChange::Move { dx: 0.0, dy: 0.0 };
        assert!(describe_edit(&slider_line(&still, 0)).ends_with("left where it is"));
    }

    #[test]
    fn template_change_logs_name_the_turn_and_the_move() {
        let pivot = NormalizedPoint::new(0.5, 0.5).unwrap();
        let turn = EditChange::Rotate {
            degrees: 10.0,
            pivot,
        };
        let moved = EditChange::Move { dx: 0.1, dy: 0.0 };
        let lines = [slider_line(&turn, 0), slider_line(&moved, 1)];
        let summary = template_summary(&lines);
        assert!(summary.changes[0].text.contains("10 degrees clockwise"));
        assert!(
            summary.changes[1]
                .text
                .contains("moved 10% across, 0% down")
        );
    }

    #[test]
    fn describes_moved_points_and_caps_the_list() {
        let change = moved_points(8, 6);
        let text = describe_edit(&slider_line(&change, 0));
        assert!(
            text.contains("point 1 moved from (10% across, 50% down) to (10% across, 70% down)"),
            "{text}"
        );
        assert!(text.contains("point 4"), "{text}");
        assert!(!text.contains("point 5"), "{text}");
        assert!(text.ends_with("and 2 more"), "{text}");
    }

    #[test]
    fn the_summariser_message_numbers_each_edit() {
        let slider = EditChange::Slider {
            axis: None,
            value: 0.8,
        };
        let points = moved_points(3, 1);
        let text = summariser_user_text(&[slider_line(&slider, 0), slider_line(&points, 1)]);
        assert!(
            text.starts_with("Edits:\n0. couch: Couch length of the couch: scaled to 80%"),
            "{text}"
        );
        assert!(text.contains("\n1. couch:"), "{text}");
    }

    fn summary_json(instruction: &str, changes: &Value) -> String {
        json!({ "instruction": instruction, "summary": "Shortened the couch", "changes": changes })
            .to_string()
    }

    #[test]
    fn accepts_a_valid_summary() {
        let change = EditChange::Slider {
            axis: None,
            value: 0.6,
        };
        let lines = [slider_line(&change, 0)];
        let answer = summary_json(
            "Make the sofa about 40% shorter.",
            &json!([{ "edit_index": 0, "text": "Shortened the couch to about 60% of its length" }]),
        );
        let parsed = parse_edit_summary(&answer, &lines).unwrap();
        assert_eq!(parsed.instruction, "Make the sofa about 40% shorter.");
        assert_eq!(parsed.summary, "Shortened the couch");
        assert_eq!(
            parsed.changes[0].text,
            "Shortened the couch to about 60% of its length"
        );
    }

    #[test]
    fn fills_missing_drops_unknown_and_ignores_duplicate_change_lines() {
        let a = EditChange::Slider {
            axis: None,
            value: 0.6,
        };
        let b = EditChange::Slider {
            axis: None,
            value: 0.9,
        };
        let lines = [slider_line(&a, 0), slider_line(&b, 1)];
        let answer = summary_json(
            "Shorten the sofa slightly more than before.",
            &json!([
                { "edit_index": 1, "text": "Trimmed it a little" },
                { "edit_index": 1, "text": "Duplicate line" },
                { "edit_index": 9, "text": "About something that does not exist" },
            ]),
        );
        let parsed = parse_edit_summary(&answer, &lines).unwrap();
        let indexes: Vec<u32> = parsed.changes.iter().map(|c| c.edit_index).collect();
        assert_eq!(indexes, [0, 1]);
        assert_eq!(parsed.changes[1].text, "Trimmed it a little");
        assert!(
            parsed.changes[0]
                .text
                .starts_with("Changed the couch length of the couch"),
            "{parsed:?}"
        );
    }

    #[test]
    fn rejects_unusable_summaries() {
        let change = EditChange::Slider {
            axis: None,
            value: 0.6,
        };
        let lines = [slider_line(&change, 0)];
        assert!(parse_edit_summary("nope", &lines).is_err());
        assert!(
            parse_edit_summary(&summary_json("short", &json!([])), &lines)
                .unwrap_err()
                .contains("instruction")
        );
        assert!(
            parse_edit_summary(&summary_json(&"x".repeat(1600), &json!([])), &lines).is_err(),
            "overlong instructions are rejected"
        );
        let blank_summary = json!({ "instruction": "Make the sofa shorter please.", "summary": " ", "changes": [] }).to_string();
        assert!(
            parse_edit_summary(&blank_summary, &lines)
                .unwrap_err()
                .contains("summary")
        );
    }

    #[test]
    fn template_summaries_cover_every_edit() {
        let a = EditChange::Slider {
            axis: None,
            value: 0.6,
        };
        let b = moved_points(3, 2);
        let single = template_summary(&[slider_line(&a, 0)]);
        assert_eq!(single.summary, "Changed the couch length of the couch");
        assert!(single.instruction.contains("scaled to 60%"));

        let both = template_summary(&[slider_line(&a, 0), slider_line(&b, 1)]);
        assert_eq!(both.summary, "Made 2 changes");
        assert_eq!(both.changes.len(), 2);
        assert!(
            both.changes[1].text.ends_with("(2 points moved)"),
            "{:?}",
            both.changes
        );
    }

    #[test]
    fn the_generation_text_depends_on_whether_there_is_a_preview() {
        let with = generation_instruction("Shorten the sofa.", &[], Some(PreviewKind::Warped));
        assert!(with.contains("Shorten the sofa.") && with.contains("second image"));
        assert!(with.contains("identical to the original"));
        let without = generation_instruction("Shorten the sofa.", &[], None);
        assert!(without.contains("Shorten the sofa.") && !without.contains("second image"));
        assert!(!with.contains("{instruction}") && !without.contains("{instruction}"));
    }

    #[test]
    fn an_outline_reference_names_the_objects_and_ignores_the_wording() {
        let text = generation_instruction(
            "point 3 moved from 10% to 20%",
            &["bottle"],
            Some(PreviewKind::Outline),
        );
        assert!(text.contains("magenta outline drawn around the bottle"));
        assert!(text.contains("do not draw them in the result"));
        assert!(!text.contains("point 3") && !text.contains('{'));

        let several =
            generation_instruction("", &["bottle", "cup", "vase"], Some(PreviewKind::Outline));
        assert!(several.contains("bottle, cup and vase"), "{several}");
    }

    #[test]
    fn the_summary_schema_is_strict() {
        let schema = summary_schema();
        assert_eq!(schema["additionalProperties"], false);
        assert!(!schema.to_string().contains("$ref"));
        for field in ["instruction", "summary", "changes"] {
            assert!(
                schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(field)),
                "{field}"
            );
        }
    }
}
