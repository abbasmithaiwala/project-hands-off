//! Resizing and encoding images for models and for the stored result.
//!
//! Pixel coordinates are bounded by validated image sizes, so the casts cannot overflow.
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use std::io::Cursor;

use hands_off_core::{Error, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{DynamicImage, ImageFormat, RgbImage};

const JPEG_QUALITY: u8 = 92;

/// Resize `image` to exactly `width` x `height`.
#[must_use]
pub fn fit_to(image: &DynamicImage, width: u32, height: u32) -> RgbImage {
    let rgb = image.to_rgb8();
    if rgb.dimensions() == (width, height) {
        return rgb;
    }
    imageops::resize(&rgb, width, height, FilterType::Lanczos3)
}

/// Shrink `image` so its longest side is at most `max_side`, never enlarging it.
#[must_use]
pub fn fit_within(image: &DynamicImage, max_side: u32) -> RgbImage {
    if image.width().max(image.height()) <= max_side {
        return image.to_rgb8();
    }
    image
        .resize(max_side, max_side, FilterType::Lanczos3)
        .to_rgb8()
}

/// How much the shape of `(width, height)` differs from `(other_width, other_height)`, as a
/// fraction (0 means the same aspect ratio).
#[must_use]
pub fn aspect_difference(width: u32, height: u32, other_width: u32, other_height: u32) -> f32 {
    let a = width as f32 / height.max(1) as f32;
    let b = other_width as f32 / other_height.max(1) as f32;
    (a / b - 1.0).abs()
}

/// Encode as PNG.
///
/// # Errors
/// [`Error::Internal`] if encoding fails.
pub fn encode_png(image: &RgbImage) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, ImageFormat::Png)
        .map_err(Error::internal)?;
    Ok(out.into_inner())
}

/// Encode as high-quality JPEG, for sending to models.
///
/// # Errors
/// [`Error::Internal`] if encoding fails.
pub fn encode_jpeg(image: &RgbImage) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(image)
        .map_err(Error::internal)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    fn flat(width: u32, height: u32, colour: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(width, height, Rgb(colour))
    }

    #[test]
    fn resizing_and_aspect_helpers() {
        let image = DynamicImage::ImageRgb8(flat(200, 100, [1, 2, 3]));
        assert_eq!(fit_to(&image, 100, 50).dimensions(), (100, 50));
        assert_eq!(fit_to(&image, 200, 100).dimensions(), (200, 100));
        assert_eq!(fit_within(&image, 100).dimensions(), (100, 50));
        assert_eq!(fit_within(&image, 1000).dimensions(), (200, 100));
        assert!(aspect_difference(200, 100, 100, 50) < 1e-6);
        assert!((aspect_difference(200, 100, 100, 100) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn encoders_produce_decodable_images() {
        let image = flat(8, 6, [10, 20, 30]);
        assert_eq!(
            image::load_from_memory(&encode_png(&image).unwrap())
                .unwrap()
                .width(),
            8
        );
        assert_eq!(
            image::load_from_memory(&encode_jpeg(&image).unwrap())
                .unwrap()
                .height(),
            6
        );
    }
}
