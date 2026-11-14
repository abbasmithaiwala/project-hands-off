//! Image decoding, validation and resizing.

use std::io::Cursor;

use hands_off_core::model::ImageFormat;
use hands_off_core::{Error, Limits, Result};
use image::codecs::jpeg::JpegEncoder;
use image::{ImageReader, imageops::FilterType};

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
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| Error::Validation("image is corrupt or unreadable".into()))?;

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

/// Produce a JPEG thumbnail whose longest side is at most `max_side` pixels.
///
/// # Errors
/// Returns [`Error::Validation`] if the image cannot be decoded and
/// [`Error::Internal`] if encoding fails.
pub fn thumbnail(bytes: &[u8], max_side: u32) -> Result<Vec<u8>> {
    let decoded = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| unsupported())?
        .decode()
        .map_err(|_| Error::Validation("image is corrupt or unreadable".into()))?;
    let resized = decoded
        .resize(max_side, max_side, FilterType::Triangle)
        .to_rgb8();

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
    fn thumbnail_never_upscales_beyond_the_bound() {
        let jpeg = thumbnail(&encode(20, 10, Fmt::Png), 100).unwrap();
        let info = inspect(&jpeg, &limits()).unwrap();
        assert!(info.width <= 100 && info.height <= 100);
    }

    #[test]
    fn thumbnail_rejects_garbage() {
        assert!(thumbnail(b"nope", 64).is_err());
    }
}
