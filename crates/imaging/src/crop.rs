//! Cropping objects out of images for models, and mapping crop coordinates back.
//!
//! Coordinates are bounded by validated image sizes, so the numeric casts cannot overflow.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::io::Cursor;

use hands_off_core::{BBox, Error, NormalizedPoint, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage};

use crate::mask::Mask;

const JPEG_QUALITY: u8 = 88;

/// A rectangle in normalised image coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropRegion {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl CropRegion {
    /// `bbox` grown by `padding` (a fraction of its size) on every side, clamped to the image.
    #[must_use]
    pub fn around(bbox: &BBox, padding: f32) -> Self {
        let (pad_x, pad_y) = (bbox.width * padding, bbox.height * padding);
        let (x0, y0) = ((bbox.x - pad_x).max(0.0), (bbox.y - pad_y).max(0.0));
        let (x1, y1) = (
            (bbox.x + bbox.width + pad_x).min(1.0),
            (bbox.y + bbox.height + pad_y).min(1.0),
        );
        Self {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }

    /// Convert a point given relative to this crop (`0..1` inside it) to image coordinates.
    /// Points outside the crop are clamped to the image.
    #[must_use]
    pub fn to_image_point(&self, x: f32, y: f32) -> NormalizedPoint {
        NormalizedPoint {
            x: (self.x + x * self.width).clamp(0.0, 1.0),
            y: (self.y + y * self.height).clamp(0.0, 1.0),
        }
    }

    fn pixels(&self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let x = ((self.x * width as f32).floor() as u32).min(width.saturating_sub(1));
        let y = ((self.y * height as f32).floor() as u32).min(height.saturating_sub(1));
        let w = ((self.width * width as f32).ceil() as u32).clamp(1, width - x);
        let h = ((self.height * height as f32).ceil() as u32).clamp(1, height - y);
        (x, y, w, h)
    }
}

/// Crop `region` out of `image`, shrink it to at most `max_side` pixels and encode as JPEG.
///
/// # Errors
/// [`Error::Internal`] if encoding fails.
pub fn crop_to_jpeg(image: &DynamicImage, region: &CropRegion, max_side: u32) -> Result<Vec<u8>> {
    let (x, y, w, h) = region.pixels(image.width(), image.height());
    let mut crop = image.crop_imm(x, y, w, h);
    if crop.width().max(crop.height()) > max_side {
        crop = crop.resize(max_side, max_side, FilterType::Triangle);
    }
    encode(&crop)
}

/// Like [`crop_to_jpeg`] for a mask: white is the object, black is everything else.
///
/// # Errors
/// [`Error::Internal`] if encoding fails.
pub fn crop_mask_to_jpeg(mask: &Mask, region: &CropRegion, max_side: u32) -> Result<Vec<u8>> {
    let pixels = (0..mask.height())
        .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
        .map(|(x, y)| if mask.get(x as i32, y as i32) { 255 } else { 0 })
        .collect();
    let gray = GrayImage::from_raw(mask.width(), mask.height(), pixels)
        .ok_or_else(|| Error::Validation("mask dimensions do not match its data".into()))?;
    let image = DynamicImage::ImageLuma8(gray);
    let (x, y, w, h) = region.pixels(image.width(), image.height());
    let mut crop = image.crop_imm(x, y, w, h);
    if crop.width().max(crop.height()) > max_side {
        crop = crop.resize(max_side, max_side, FilterType::Nearest);
    }
    encode(&crop)
}

fn encode(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&image.to_rgb8())
        .map_err(Error::internal)?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    fn bbox(x: f32, y: f32, w: f32, h: f32) -> BBox {
        BBox::new(x, y, w, h).unwrap()
    }

    #[test]
    fn padding_grows_the_box_and_clamps_to_the_image() {
        let region = CropRegion::around(&bbox(0.2, 0.2, 0.4, 0.4), 0.25);
        assert!((region.x - 0.1).abs() < 1e-6 && (region.width - 0.6).abs() < 1e-6);

        let edge = CropRegion::around(&bbox(0.0, 0.0, 0.5, 0.5), 0.5);
        assert_eq!((edge.x, edge.y), (0.0, 0.0));
        let far = CropRegion::around(&bbox(0.6, 0.6, 0.4, 0.4), 0.5);
        assert!((far.x + far.width - 1.0).abs() < 1e-6);
    }

    #[test]
    fn crop_points_map_back_to_image_coordinates_and_clamp() {
        let region = CropRegion {
            x: 0.2,
            y: 0.4,
            width: 0.5,
            height: 0.2,
        };
        let centre = region.to_image_point(0.5, 0.5);
        assert!((centre.x - 0.45).abs() < 1e-6 && (centre.y - 0.5).abs() < 1e-6);
        let outside = region.to_image_point(3.0, -3.0);
        assert_eq!((outside.x, outside.y), (1.0, 0.0));
    }

    #[test]
    fn cropped_jpeg_has_the_region_size_and_respects_the_size_limit() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(400, 200, Rgb([10, 200, 30])));
        let region = CropRegion {
            x: 0.25,
            y: 0.0,
            width: 0.5,
            height: 1.0,
        };

        let whole = image::load_from_memory(&crop_to_jpeg(&image, &region, 1000).unwrap()).unwrap();
        assert_eq!((whole.width(), whole.height()), (200, 200));

        let small = image::load_from_memory(&crop_to_jpeg(&image, &region, 50).unwrap()).unwrap();
        assert_eq!((small.width(), small.height()), (50, 50));
    }

    #[test]
    fn mask_crop_is_white_on_the_object_and_black_elsewhere() {
        let data = (0..20_u32)
            .flat_map(|y| (0..20_u32).map(move |x| (5..15).contains(&x) && (5..15).contains(&y)))
            .collect();
        let mask = Mask::new(20, 20, data).unwrap();
        let jpeg = crop_mask_to_jpeg(
            &mask,
            &CropRegion {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            100,
        )
        .unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap().to_luma8();
        assert!(decoded.get_pixel(10, 10).0[0] > 200);
        assert!(decoded.get_pixel(1, 1).0[0] < 50);
    }

    #[test]
    fn tiny_regions_still_produce_at_least_one_pixel() {
        let image = DynamicImage::ImageRgb8(RgbImage::new(10, 10));
        let region = CropRegion {
            x: 0.99,
            y: 0.99,
            width: 0.0001,
            height: 0.0001,
        };
        assert!(crop_to_jpeg(&image, &region, 100).is_ok());
    }
}
