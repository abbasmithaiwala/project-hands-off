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

fn distance_to_segment(p: NormalizedPoint, a: NormalizedPoint, b: NormalizedPoint) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f32::EPSILON {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / length_sq).clamp(0.0, 1.0);
    (p.x - (a.x + t * dx)).hypot(p.y - (a.y + t * dy))
}

/// Ramer-Douglas-Peucker on an open chain, keeping both ends.
fn simplify_chain(chain: &[NormalizedPoint], tolerance: f32) -> Vec<NormalizedPoint> {
    let (Some(&first), Some(&last)) = (chain.first(), chain.last()) else {
        return Vec::new();
    };
    let worst = chain
        .iter()
        .enumerate()
        .skip(1)
        .take(chain.len().saturating_sub(2))
        .map(|(i, &p)| (i, distance_to_segment(p, first, last)))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    match worst {
        Some((index, distance)) if distance > tolerance => {
            let mut left = simplify_chain(&chain[..=index], tolerance);
            left.pop();
            left.extend(simplify_chain(&chain[index..], tolerance));
            left
        }
        _ => vec![first, last],
    }
}

/// The corners of a closed `outline`: as few straight-edged vertices as keep the shape, at most
/// `max_points`. Empty when the outline is too small to be a shape.
#[must_use]
pub fn corner_points(outline: &[NormalizedPoint], max_points: usize) -> Vec<NormalizedPoint> {
    const START_TOLERANCE: f32 = 0.002;
    const MIN_POINTS: usize = 4;
    if outline.len() < MIN_POINTS {
        return Vec::new();
    }
    // Split the loop at its two most distant vertices so each half is an open chain.
    let far = (1..outline.len())
        .max_by(|&i, &j| {
            let d = |k: usize| (outline[k].x - outline[0].x).hypot(outline[k].y - outline[0].y);
            d(i).total_cmp(&d(j))
        })
        .unwrap_or(1);
    let mut closing = outline[far..].to_vec();
    closing.push(outline[0]);
    let mut tolerance = START_TOLERANCE;
    loop {
        let mut points = simplify_chain(&outline[..=far], tolerance);
        points.pop();
        let mut back = simplify_chain(&closing, tolerance);
        back.pop();
        points.extend(back);
        if points.len() <= max_points || tolerance > 1.0 {
            return if points.len() >= MIN_POINTS {
                points
            } else {
                Vec::new()
            };
        }
        tolerance *= 1.4;
    }
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
            let mut points = corner_points(&object.outline, MAX_DEFAULT_POINTS);
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

    fn l_shape() -> Vec<NormalizedPoint> {
        // An L made of many collinear points, as a traced mask outline would be.
        let mut points = Vec::new();
        let corners = [
            (0.2, 0.2),
            (0.5, 0.2),
            (0.5, 0.5),
            (0.8, 0.5),
            (0.8, 0.8),
            (0.2, 0.8),
        ];
        for i in 0..corners.len() {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            for step in 0..10_u8 {
                let t = f32::from(step) / 10.0;
                points.push(point(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        points
    }

    #[test]
    fn corners_of_a_traced_shape_are_its_real_corners() {
        let corners = corner_points(&l_shape(), 12);
        assert_eq!(corners.len(), 6, "{corners:?}");
        for expected in [(0.2, 0.2), (0.5, 0.5), (0.8, 0.8)] {
            assert!(
                corners
                    .iter()
                    .any(|p| (p.x - expected.0).abs() < 1e-3 && (p.y - expected.1).abs() < 1e-3),
                "{expected:?} missing from {corners:?}"
            );
        }
    }

    #[test]
    fn corners_are_limited_by_loosening_the_tolerance() {
        let circle: Vec<_> = (0..90_u8)
            .map(|i| {
                let angle = f32::from(i) / 90.0 * std::f32::consts::TAU;
                point(0.5 + 0.3 * angle.cos(), 0.5 + 0.3 * angle.sin())
            })
            .collect();
        let corners = corner_points(&circle, 10);
        assert!((4..=10).contains(&corners.len()), "{}", corners.len());
    }

    #[test]
    fn tiny_outlines_have_no_corners() {
        assert!(corner_points(&[point(0.1, 0.1), point(0.2, 0.2)], 12).is_empty());
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
