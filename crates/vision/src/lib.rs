//! Local object detection and segmentation behind swappable traits.

pub mod coco;
pub mod detector;
pub mod models;
pub mod rfdetr;

pub use detector::{Detection, Detector};
