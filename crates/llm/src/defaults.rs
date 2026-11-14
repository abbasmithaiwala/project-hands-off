//! Deterministic fallbacks used when a model cannot provide a usable answer, so the editor
//! never dead-ends.

use hands_off_core::NormalizedPoint;
use hands_off_core::model::{Axis, DetectedObject, Property, Widget, WidgetKind};

const MAX_DEFAULT_POINTS: usize = 12;
const SLIDER_RANGE: (f32, f32, f32) = (0.5, 1.5, 1.0);

/// Generic properties that make sense for almost any object.
#[must_use]
pub fn default_properties() -> Vec<Property> {
    vec![
        Property {
            key: "size".into(),
            label: "Size".into(),
            widget: WidgetKind::Slider,
        },
        Property {
            key: "shape".into(),
            label: "Shape".into(),
            widget: WidgetKind::ControlPoints,
        },
    ]
}

fn point(x: f32, y: f32) -> NormalizedPoint {
    NormalizedPoint {
        x: x.clamp(0.0, 1.0),
        y: y.clamp(0.0, 1.0),
    }
}

fn edge_lengths(outline: &[NormalizedPoint]) -> Vec<f32> {
    (0..outline.len())
        .map(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
            (b.x - a.x).hypot(b.y - a.y)
        })
        .collect()
}

/// `count` points spread evenly (by distance) around the closed `outline`.
#[must_use]
pub fn evenly_along(outline: &[NormalizedPoint], count: usize) -> Vec<NormalizedPoint> {
    let lengths = edge_lengths(outline);
    let total: f32 = lengths.iter().sum();
    if outline.len() < 3 || count == 0 || total <= f32::EPSILON {
        return Vec::new();
    }
    let mut points = Vec::with_capacity(count);
    let (mut edge, mut walked) = (0, 0.0_f32);
    let spacing = total / f32::from(u16::try_from(count).unwrap_or(u16::MAX));
    for n in 0..count {
        let target = spacing * f32::from(u16::try_from(n).unwrap_or(u16::MAX));
        while edge + 1 < lengths.len() && walked + lengths[edge] < target {
            walked += lengths[edge];
            edge += 1;
        }
        let t = if lengths[edge] > 0.0 {
            ((target - walked) / lengths[edge]).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (a, b) = (outline[edge], outline[(edge + 1) % outline.len()]);
        points.push(point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
    }
    points
}

/// Where along the outline (0 to the perimeter) the closest point to `target` lies, and that
/// point.
fn nearest_on_outline(
    outline: &[NormalizedPoint],
    target: NormalizedPoint,
) -> (f32, NormalizedPoint) {
    let lengths = edge_lengths(outline);
    let mut best = (f32::INFINITY, 0.0, target);
    let mut walked = 0.0;
    for (i, length) in lengths.iter().enumerate() {
        let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
        let t = if *length > 0.0 {
            (((target.x - a.x) * (b.x - a.x) + (target.y - a.y) * (b.y - a.y)) / (length * length))
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        let candidate = point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
        let distance = (candidate.x - target.x).hypot(candidate.y - target.y);
        if distance < best.0 {
            best = (distance, walked + length * t, candidate);
        }
        walked += length;
    }
    (best.1, best.2)
}

/// Move model-chosen control points onto the object's outline so they sit on its edge.
///
/// Points that land close together are merged, and the result follows the outline's order.
/// Falls back to evenly spaced points when too few distinct ones remain.
#[must_use]
pub fn anchor_to_outline(
    points: &[NormalizedPoint],
    outline: &[NormalizedPoint],
) -> Vec<NormalizedPoint> {
    const MERGE_DISTANCE: f32 = 0.03;
    if outline.len() < 3 {
        return points.to_vec();
    }
    let mut anchored: Vec<(f32, NormalizedPoint)> = Vec::new();
    for &target in points {
        let (along, snapped) = nearest_on_outline(outline, target);
        let crowded = anchored
            .iter()
            .any(|(_, p)| (p.x - snapped.x).hypot(p.y - snapped.y) < MERGE_DISTANCE);
        if !crowded {
            anchored.push((along, snapped));
        }
    }
    if anchored.len() < 4 {
        return evenly_along(outline, MAX_DEFAULT_POINTS);
    }
    anchored.sort_by(|a, b| a.0.total_cmp(&b.0));
    anchored.into_iter().map(|(_, p)| p).collect()
}

/// A sensible widget of `kind` placed from the object's box and outline.
#[must_use]
pub fn default_widget(kind: WidgetKind, property_key: &str, object: &DetectedObject) -> Widget {
    let b = object.bbox;
    let (cx, cy) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    match kind {
        WidgetKind::Slider => {
            let vertical = ["height", "tall", "stack"]
                .iter()
                .any(|word| property_key.contains(word));
            let axis = if vertical {
                Axis {
                    start: point(cx, b.y + b.height),
                    end: point(cx, b.y),
                }
            } else {
                Axis {
                    start: point(b.x, cy),
                    end: point(b.x + b.width, cy),
                }
            };
            Widget::Slider {
                axis,
                min: SLIDER_RANGE.0,
                max: SLIDER_RANGE.1,
                initial: SLIDER_RANGE.2,
            }
        }
        WidgetKind::ControlPoints => {
            let mut points = evenly_along(&object.outline, MAX_DEFAULT_POINTS);
            if points.len() < 3 {
                points = vec![
                    point(b.x, b.y),
                    point(b.x + b.width, b.y),
                    point(b.x + b.width, b.y + b.height),
                    point(b.x, b.y + b.height),
                ];
            }
            Widget::ControlPoints { points }
        }
        WidgetKind::MultiHandle => Widget::MultiHandle {
            handles: vec![
                point(cx, b.y),
                point(b.x + b.width, cy),
                point(cx, b.y + b.height),
                point(b.x, cy),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use hands_off_core::{BBox, ObjectId};

    use super::*;

    fn object(outline_len: usize) -> DetectedObject {
        let outline = (0..outline_len)
            .map(|i| {
                point(
                    0.2 + 0.5
                        * (f32::from(u8::try_from(i).unwrap())
                            / f32::from(u8::try_from(outline_len.max(1)).unwrap())),
                    0.3,
                )
            })
            .collect();
        DetectedObject {
            id: ObjectId(0),
            label: "couch".into(),
            bbox: BBox::new(0.2, 0.3, 0.5, 0.4).unwrap(),
            score: 0.9,
            outline,
        }
    }

    #[test]
    fn default_properties_cover_a_slider_and_a_shape() {
        let kinds: Vec<_> = default_properties().iter().map(|p| p.widget).collect();
        assert_eq!(kinds, [WidgetKind::Slider, WidgetKind::ControlPoints]);
    }

    #[test]
    fn sliders_run_across_the_box_and_vertically_for_height_like_properties() {
        let horizontal = default_widget(WidgetKind::Slider, "length", &object(8));
        let Widget::Slider { axis, initial, .. } = horizontal else {
            panic!()
        };
        assert!((axis.start.y - axis.end.y).abs() < 1e-6 && axis.end.x > axis.start.x);
        assert!((initial - 1.0).abs() < f32::EPSILON);

        let vertical = default_widget(WidgetKind::Slider, "stack_height", &object(8));
        let Widget::Slider { axis, .. } = vertical else {
            panic!()
        };
        assert!((axis.start.x - axis.end.x).abs() < 1e-6 && axis.end.y < axis.start.y);
    }

    #[test]
    fn control_points_are_thinned_from_the_outline() {
        let Widget::ControlPoints { points } =
            default_widget(WidgetKind::ControlPoints, "shape", &object(100))
        else {
            panic!()
        };
        assert!(
            (3..=MAX_DEFAULT_POINTS).contains(&points.len()),
            "{}",
            points.len()
        );
    }

    fn square_outline() -> Vec<NormalizedPoint> {
        [(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8)]
            .iter()
            .map(|&(x, y)| point(x, y))
            .collect()
    }

    fn distance_to_square(p: NormalizedPoint) -> f32 {
        let outside = p.x < 0.2 - 1e-4 || p.x > 0.8 + 1e-4 || p.y < 0.2 - 1e-4 || p.y > 0.8 + 1e-4;
        if outside {
            return f32::INFINITY;
        }
        (p.x - 0.2)
            .abs()
            .min((p.x - 0.8).abs())
            .min((p.y - 0.2).abs())
            .min((p.y - 0.8).abs())
    }

    #[test]
    fn evenly_spaced_points_lie_on_the_outline() {
        let points = evenly_along(&square_outline(), 8);
        assert_eq!(points.len(), 8);
        for p in points {
            assert!(distance_to_square(p) < 1e-4, "{p:?}");
        }
    }

    #[test]
    fn model_points_inside_the_object_are_moved_to_its_edge() {
        let inside: Vec<_> = [(0.5, 0.3), (0.7, 0.5), (0.5, 0.7), (0.3, 0.5), (0.4, 0.4)]
            .iter()
            .map(|&(x, y)| point(x, y))
            .collect();
        let anchored = anchor_to_outline(&inside, &square_outline());
        assert!(anchored.len() >= 4);
        for p in anchored {
            assert!(distance_to_square(p) < 1e-4, "{p:?}");
        }
    }

    #[test]
    fn crowded_model_points_fall_back_to_even_spacing() {
        let crowded: Vec<_> = (0..6_u8)
            .map(|i| point(0.5 + f32::from(i) * 0.001, 0.5))
            .collect();
        let anchored = anchor_to_outline(&crowded, &square_outline());
        assert_eq!(anchored.len(), MAX_DEFAULT_POINTS);
    }

    #[test]
    fn control_points_fall_back_to_the_box_for_degenerate_outlines() {
        let Widget::ControlPoints { points } =
            default_widget(WidgetKind::ControlPoints, "shape", &object(1))
        else {
            panic!()
        };
        assert_eq!(points.len(), 4);
    }

    #[test]
    fn handles_sit_on_the_edge_midpoints() {
        let Widget::MultiHandle { handles } =
            default_widget(WidgetKind::MultiHandle, "arms", &object(8))
        else {
            panic!()
        };
        assert_eq!(handles.len(), 4);
        assert!(
            handles
                .iter()
                .all(|p| (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y))
        );
    }
}
