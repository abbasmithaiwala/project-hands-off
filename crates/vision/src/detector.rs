//! The detector abstraction used by the rest of the application.

use hands_off_core::{BBox, Result};
use hands_off_imaging::mask::Mask;
use image::DynamicImage;

/// One detected object with its mask.
#[derive(Debug, Clone)]
pub struct Detection {
    /// COCO category id.
    pub class_id: u16,
    /// Human-readable class label.
    pub label: &'static str,
    /// Confidence in `0.0..=1.0`.
    pub score: f32,
    /// Bounding box in normalised image coordinates.
    pub bbox: BBox,
    /// Cleaned-up mask of the object, covering the whole image at reduced resolution.
    pub mask: Mask,
}

/// Finds and segments objects in an image. Implementations are blocking and CPU-bound;
/// call them from `spawn_blocking`.
pub trait Detector: Send + Sync {
    /// Detect objects, most confident first.
    ///
    /// # Errors
    /// Returns an error if inference fails.
    fn detect(&self, image: &DynamicImage) -> Result<Vec<Detection>>;
}
