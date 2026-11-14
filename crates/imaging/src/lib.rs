//! Image decoding, validation, resizing and mask processing.

pub mod composite;
pub mod crop;
pub mod mask;
pub mod object_color;

use std::io::Cursor;

use hands_off_core::model::ImageFormat;
use hands_off_core::{Error, Limits, Result};
use image::codecs::jpeg::JpegEncoder;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};

const THUMBNAIL_JPEG_QUALITY: u8 = 85;

/// Format and size of a validated image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    /// Detected format.
    pub format: ImageFormat,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// Validate `bytes` as a supported image within `limits` without fully decoding it.
///
/// # Errors
/// - [`Error::UnsupportedMedia`] if the data is not a JPEG, PNG or WebP image.
/// - [`Error::PayloadTooLarge`] if the file or its dimensions exceed `limits`.
/// - [`Error::Validation`] if the image header is corrupt or empty.
pub fn inspect(bytes: &[u8], limits: &Limits) -> Result<ImageInfo> {
    if bytes.len() > limits.max_upload_bytes {
        return Err(Error::PayloadTooLarge(format!(
            "image is {} bytes; the limit is {}",
            bytes.len(),
            limits.max_upload_bytes
        )));
    }

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| unsupported())?;
    let format = match reader.format() {
        Some(image::ImageFormat::Jpeg) => ImageFormat::Jpeg,
        Some(image::ImageFormat::Png) => ImageFormat::Png,
        Some(image::ImageFormat::WebP) => ImageFormat::Webp,
        _ => return Err(unsupported()),
    };
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| Error::Validation("image is corrupt or unreadable".into()))?;
    let (raw_width, raw_height) = decoder.dimensions();
    let (width, height) = if swaps_axes(decoder.orientation().unwrap_or(Orientation::NoTransforms))
    {
        (raw_height, raw_width)
    } else {
        (raw_width, raw_height)
    };

    if width == 0 || height == 0 {
        return Err(Error::Validation("image has no pixels".into()));
    }
    if width > limits.max_dimension || height > limits.max_dimension {
        return Err(Error::PayloadTooLarge(format!(
            "image is {width}x{height}; the limit is {0}x{0}",
            limits.max_dimension
        )));
    }
    Ok(ImageInfo {
        format,
        width,
        height,
    })
}

/// Decode an image and apply its EXIF orientation, so the pixels match what a viewer shows.
///
/// # Errors
/// [`Error::UnsupportedMedia`] for unknown formats and [`Error::Validation`] for corrupt data.
pub fn decode_oriented(bytes: &[u8]) -> Result<DynamicImage> {
    let corrupt = |_| Error::Validation("image is corrupt or unreadable".into());
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| unsupported())?
        .into_decoder()
        .map_err(corrupt)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).map_err(corrupt)?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn swaps_axes(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

/// Produce a JPEG thumbnail whose longest side is at most `max_side` pixels. Images that
/// already fit are not enlarged.
///
/// # Errors
/// Returns [`Error::Validation`] if the image cannot be decoded and
/// [`Error::Internal`] if encoding fails.
pub fn thumbnail(bytes: &[u8], max_side: u32) -> Result<Vec<u8>> {
    let decoded = decode_oriented(bytes)?;
    let resized = if decoded.width() <= max_side && decoded.height() <= max_side {
        decoded.to_rgb8()
    } else {
        decoded
            .resize(max_side, max_side, FilterType::Triangle)
            .to_rgb8()
    };

    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, THUMBNAIL_JPEG_QUALITY)
        .encode_image(&resized)
        .map_err(Error::internal)?;
    Ok(out)
}

fn unsupported() -> Error {
    Error::UnsupportedMedia("only JPEG, PNG and WebP images are supported".into())
}

#[cfg(test)]
mod tests {
    use image::{ImageFormat as Fmt, RgbImage};

    use super::*;

    fn encode(width: u32, height: u32, format: Fmt) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        RgbImage::new(width, height)
            .write_to(&mut out, format)
            .unwrap();
        out.into_inner()
    }

    fn limits() -> Limits {
        Limits {
            max_upload_bytes: 1024 * 1024,
            max_dimension: 512,
        }
    }

    #[test]
    fn inspects_png_and_jpeg() {
        let png = inspect(&encode(40, 30, Fmt::Png), &limits()).unwrap();
        assert_eq!(
            png,
            ImageInfo {
                format: ImageFormat::Png,
                width: 40,
                height: 30
            }
        );
        let jpeg = inspect(&encode(10, 20, Fmt::Jpeg), &limits()).unwrap();
        assert_eq!(jpeg.format, ImageFormat::Jpeg);
    }

    #[test]
    fn rejects_non_images() {
        let error = inspect(b"definitely not an image", &limits()).unwrap_err();
        assert!(matches!(error, Error::UnsupportedMedia(_)));
    }

    #[test]
    fn rejects_unsupported_formats() {
        let gif_header = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";
        let error = inspect(gif_header, &limits()).unwrap_err();
        assert!(matches!(error, Error::UnsupportedMedia(_)));
    }

    #[test]
    fn rejects_oversized_files_and_dimensions() {
        let small_limit = Limits {
            max_upload_bytes: 16,
            max_dimension: 512,
        };
        assert!(matches!(
            inspect(&encode(40, 30, Fmt::Png), &small_limit).unwrap_err(),
            Error::PayloadTooLarge(_)
        ));
        assert!(matches!(
            inspect(&encode(600, 10, Fmt::Png), &limits()).unwrap_err(),
            Error::PayloadTooLarge(_)
        ));
    }

    /// Insert an EXIF segment declaring `orientation` right after a JPEG's SOI marker.
    fn with_orientation(jpeg: &[u8], orientation: u8) -> Vec<u8> {
        let mut exif =
            b"Exif\0\0MM\x00\x2a\x00\x00\x00\x08\x00\x01\x01\x12\x00\x03\x00\x00\x00\x01\x00"
                .to_vec();
        exif.extend([orientation, 0, 0, 0, 0]);
        let mut out = jpeg[..2].to_vec();
        out.extend([0xFF, 0xE1]);
        out.extend(u16::try_from(exif.len() + 2).unwrap().to_be_bytes());
        out.extend(exif);
        out.extend(&jpeg[2..]);
        out
    }

    #[test]
    fn exif_rotation_swaps_reported_dimensions() {
        let rotated = with_orientation(&encode(40, 20, Fmt::Jpeg), 6);
        let info = inspect(&rotated, &limits()).unwrap();
        assert_eq!((info.width, info.height), (20, 40));
        let upright = with_orientation(&encode(40, 20, Fmt::Jpeg), 1);
        assert_eq!(inspect(&upright, &limits()).unwrap().width, 40);
    }

    #[test]
    fn decode_oriented_applies_exif_rotation() {
        let rotated = with_orientation(&encode(40, 20, Fmt::Jpeg), 6);
        let image = decode_oriented(&rotated).unwrap();
        assert_eq!((image.width(), image.height()), (20, 40));
    }

    #[test]
    fn thumbnails_respect_exif_rotation() {
        let rotated = with_orientation(&encode(40, 20, Fmt::Jpeg), 6);
        let info = inspect(&thumbnail(&rotated, 100).unwrap(), &limits()).unwrap();
        assert_eq!((info.width, info.height), (20, 40));
    }

    #[test]
    fn rejects_truncated_images() {
        let mut bytes = encode(40, 30, Fmt::Png);
        bytes.truncate(12);
        assert!(inspect(&bytes, &limits()).is_err());
    }

    #[test]
    fn thumbnail_preserves_aspect_ratio_within_bounds() {
        let jpeg = thumbnail(&encode(400, 200, Fmt::Png), 100).unwrap();
        let info = inspect(&jpeg, &limits()).unwrap();
        assert_eq!(info.format, ImageFormat::Jpeg);
        assert_eq!((info.width, info.height), (100, 50));
    }

    #[test]
    fn thumbnail_does_not_enlarge_small_images() {
        let jpeg = thumbnail(&encode(20, 10, Fmt::Png), 100).unwrap();
        let info = inspect(&jpeg, &limits()).unwrap();
        assert_eq!((info.width, info.height), (20, 10));
    }

    #[test]
    fn thumbnail_rejects_garbage() {
        assert!(thumbnail(b"nope", 64).is_err());
    }
}
