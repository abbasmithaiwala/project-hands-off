//! Resolution-independent geometry. All coordinates are normalised to `0.0..=1.0`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::Error;

fn check_unit(name: &str, value: f32) -> Result<(), Error> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "{name} must be within 0..=1, got {value}"
        )))
    }
}

/// A point in normalised image coordinates (origin top-left).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(try_from = "RawPoint")]
pub struct NormalizedPoint {
    /// Horizontal position, `0.0` left to `1.0` right.
    pub x: f32,
    /// Vertical position, `0.0` top to `1.0` bottom.
    pub y: f32,
}

#[derive(Deserialize, JsonSchema)]
struct RawPoint {
    x: f32,
    y: f32,
}

impl NormalizedPoint {
    /// Create a validated point.
    ///
    /// # Errors
    /// Returns [`Error::Validation`] if either coordinate is outside `0..=1` or not finite.
    pub fn new(x: f32, y: f32) -> Result<Self, Error> {
        check_unit("x", x)?;
        check_unit("y", y)?;
        Ok(Self { x, y })
    }
}

impl TryFrom<RawPoint> for NormalizedPoint {
    type Error = Error;

    fn try_from(raw: RawPoint) -> Result<Self, Self::Error> {
        Self::new(raw.x, raw.y)
    }
}

/// An axis-aligned box in normalised image coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(try_from = "RawBox")]
pub struct BBox {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

#[derive(Deserialize, JsonSchema)]
struct RawBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl BBox {
    /// Create a validated box that lies entirely within the image.
    ///
    /// # Errors
    /// Returns [`Error::Validation`] if any value is not finite, the size is not positive,
    /// or the box extends past the image bounds.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, Error> {
        check_unit("x", x)?;
        check_unit("y", y)?;
        check_unit("width", width)?;
        check_unit("height", height)?;
        if width <= 0.0 || height <= 0.0 {
            return Err(Error::Validation("box must have a positive size".into()));
        }
        if x + width > 1.0 + f32::EPSILON || y + height > 1.0 + f32::EPSILON {
            return Err(Error::Validation("box extends past the image".into()));
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
}

impl TryFrom<RawBox> for BBox {
    type Error = Error;

    fn try_from(raw: RawBox) -> Result<Self, Self::Error> {
        Self::new(raw.x, raw.y, raw.width, raw.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_accepts_unit_range_inclusive() {
        assert!(NormalizedPoint::new(0.0, 1.0).is_ok());
    }

    #[test]
    fn point_rejects_out_of_range_and_non_finite() {
        assert!(NormalizedPoint::new(-0.01, 0.5).is_err());
        assert!(NormalizedPoint::new(0.5, 1.01).is_err());
        assert!(NormalizedPoint::new(f32::NAN, 0.5).is_err());
        assert!(NormalizedPoint::new(0.5, f32::INFINITY).is_err());
    }

    #[test]
    fn point_is_validated_on_deserialize() {
        assert!(serde_json::from_str::<NormalizedPoint>(r#"{"x":2.0,"y":0.5}"#).is_err());
        assert!(serde_json::from_str::<NormalizedPoint>(r#"{"x":0.2,"y":0.5}"#).is_ok());
    }

    #[test]
    fn bbox_must_fit_inside_the_image() {
        assert!(BBox::new(0.1, 0.1, 0.5, 0.5).is_ok());
        assert!(BBox::new(0.6, 0.1, 0.5, 0.5).is_err());
        assert!(BBox::new(0.1, 0.6, 0.5, 0.5).is_err());
    }

    #[test]
    fn bbox_must_have_positive_size() {
        assert!(BBox::new(0.1, 0.1, 0.0, 0.5).is_err());
        assert!(BBox::new(0.1, 0.1, 0.5, 0.0).is_err());
    }
}
