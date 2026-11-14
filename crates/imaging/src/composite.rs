//! Putting a generated image back into the original: match its size and colour, then blend it
//! in only where edits were made, so everything else stays byte-for-byte identical.
//!
//! Pixel coordinates are bounded by validated image sizes, so the casts cannot overflow.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::io::Cursor;

use hands_off_core::{BBox, Error, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

/// Colour correction is skipped unless at least this share of the image lies outside the edit
/// regions: with less untouched area there is nothing reliable to measure.
const MIN_REFERENCE_SHARE: f32 = 0.02;
const MIN_GAIN: f32 = 0.8;
const MAX_GAIN: f32 = 1.25;
const MIN_DEVIATION: f32 = 1e-3;
const JPEG_QUALITY: u8 = 92;
/// How far past an edited box the edit may reach, as a fraction of the box.
const REGION_PADDING: f32 = 0.06;
const FEATHER_DIVISOR: u32 = 70;
const MIN_FEATHER_PX: u32 = 6;

/// A rectangle of pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PixelRect {
    fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Convert normalised boxes to pixel rectangles, grown by `padding` (a fraction of each box's
/// size) and clamped to the image.
#[must_use]
pub fn regions_to_pixels(
    regions: &[BBox],
    width: u32,
    height: u32,
    padding: f32,
) -> Vec<PixelRect> {
    regions
        .iter()
        .map(|b| {
            let (w, h) = (width as f32, height as f32);
            let x0 = ((b.x - b.width * padding) * w).floor().clamp(0.0, w - 1.0);
            let y0 = ((b.y - b.height * padding) * h).floor().clamp(0.0, h - 1.0);
            let x1 = ((b.x + b.width * (1.0 + padding)) * w)
                .ceil()
                .clamp(x0 + 1.0, w);
            let y1 = ((b.y + b.height * (1.0 + padding)) * h)
                .ceil()
                .clamp(y0 + 1.0, h);
            PixelRect {
                x: x0 as u32,
                y: y0 as u32,
                width: (x1 - x0) as u32,
                height: (y1 - y0) as u32,
            }
        })
        .collect()
}

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

fn outside(rects: &[PixelRect], x: u32, y: u32) -> bool {
    !rects.iter().any(|r| r.contains(x, y))
}

struct Stats {
    mean: [f32; 3],
    deviation: [f32; 3],
}

fn measure(image: &RgbImage, rects: &[PixelRect]) -> Option<(Stats, usize)> {
    let mut sum = [0.0_f64; 3];
    let mut sum_sq = [0.0_f64; 3];
    let mut count = 0_usize;
    for (x, y, pixel) in image.enumerate_pixels() {
        if !outside(rects, x, y) {
            continue;
        }
        count += 1;
        for c in 0..3 {
            let v = f64::from(pixel[c]);
            sum[c] += v;
            sum_sq[c] += v * v;
        }
    }
    if count == 0 {
        return None;
    }
    let n = count as f64;
    let mut stats = Stats {
        mean: [0.0; 3],
        deviation: [0.0; 3],
    };
    for c in 0..3 {
        let mean = sum[c] / n;
        stats.mean[c] = mean as f32;
        stats.deviation[c] = (sum_sq[c] / n - mean * mean).max(0.0).sqrt() as f32;
    }
    Some((stats, count))
}

/// Nudge `generated` so the colour and contrast of the area outside `rects` match `original`.
///
/// Image models often shift the whole picture slightly; measuring only the untouched area
/// removes that shift without being fooled by the edit itself. Returns whether it was applied.
pub fn match_colors(generated: &mut RgbImage, original: &RgbImage, rects: &[PixelRect]) -> bool {
    let total = (original.width() * original.height()) as usize;
    let (Some((reference, outside_count)), Some((current, _))) =
        (measure(original, rects), measure(generated, rects))
    else {
        return false;
    };
    if (outside_count as f32) < MIN_REFERENCE_SHARE * total as f32 {
        return false;
    }

    let mut gain = [1.0_f32; 3];
    let mut offset = [0.0_f32; 3];
    for c in 0..3 {
        gain[c] = if current.deviation[c] > MIN_DEVIATION {
            (reference.deviation[c] / current.deviation[c]).clamp(MIN_GAIN, MAX_GAIN)
        } else {
            1.0
        };
        offset[c] = reference.mean[c] - gain[c] * current.mean[c];
    }
    for pixel in generated.pixels_mut() {
        for c in 0..3 {
            pixel[c] = (gain[c] * f32::from(pixel[c]) + offset[c])
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
    true
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much of the generated image to use at `(x, y)`: 0 outside every rectangle (so those
/// pixels are never touched), rising smoothly to 1 over `feather` pixels inside the edges.
fn weight(rects: &[PixelRect], x: u32, y: u32, feather: u32) -> f32 {
    let mut best = 0.0_f32;
    for rect in rects {
        if !rect.contains(x, y) {
            continue;
        }
        let feather = feather.min(rect.width.min(rect.height) / 2);
        let distance = (x - rect.x)
            .min(rect.x + rect.width - 1 - x)
            .min(y - rect.y)
            .min(rect.y + rect.height - 1 - y);
        let w = smoothstep((distance as f32 + 1.0) / (feather as f32 + 1.0));
        best = best.max(w);
    }
    best
}

/// Blend `generated` into `original` inside `rects`, feathering the edges over `feather`
/// pixels. Pixels outside every rectangle are copied from `original` unchanged.
///
/// # Panics
/// Never for same-sized images; if the sizes differ the overlapping area is blended.
#[must_use]
pub fn blend(
    original: &RgbImage,
    generated: &RgbImage,
    rects: &[PixelRect],
    feather: u32,
) -> RgbImage {
    let mut out = original.clone();
    let (width, height) = (
        original.width().min(generated.width()),
        original.height().min(generated.height()),
    );
    for rect in rects {
        let x_end = (rect.x + rect.width).min(width);
        let y_end = (rect.y + rect.height).min(height);
        for y in rect.y..y_end {
            for x in rect.x..x_end {
                let w = weight(rects, x, y, feather);
                if w <= 0.0 {
                    continue;
                }
                let (before, after) = (original.get_pixel(x, y), generated.get_pixel(x, y));
                let mixed = if w >= 1.0 {
                    *after
                } else {
                    Rgb([0, 1, 2].map(|c| {
                        (f32::from(before[c]) * (1.0 - w) + f32::from(after[c]) * w).round() as u8
                    }))
                };
                out.put_pixel(x, y, mixed);
            }
        }
    }
    out
}

/// The outcome of [`composite_edit`].
#[derive(Debug, Clone)]
pub struct Composited {
    /// The original with the edit blended in, at the original's resolution.
    pub image: RgbImage,
    /// How different the generated image's proportions were from the original's (0 = same).
    pub aspect_difference: f32,
    /// Whether the generated image's colours were adjusted to match the original.
    pub color_matched: bool,
}

/// Put `generated` into `original` inside `regions` (normalised boxes): resize it to match,
/// correct its colours using the untouched area, and feather it in. Everything outside the
/// regions stays identical to `original`.
#[must_use]
pub fn composite_edit(
    original: &DynamicImage,
    generated: &DynamicImage,
    regions: &[BBox],
) -> Composited {
    let original = original.to_rgb8();
    let (width, height) = original.dimensions();
    let aspect_difference = aspect_difference(generated.width(), generated.height(), width, height);
    let mut fitted = fit_to(generated, width, height);
    let rects = regions_to_pixels(regions, width, height, REGION_PADDING);
    let color_matched = match_colors(&mut fitted, &original, &rects);
    let feather = (width.min(height) / FEATHER_DIVISOR).max(MIN_FEATHER_PX);
    Composited {
        image: blend(&original, &fitted, &rects, feather),
        aspect_difference,
        color_matched,
    }
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

    fn flat(width: u32, height: u32, colour: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(width, height, Rgb(colour))
    }

    fn bbox(x: f32, y: f32, w: f32, h: f32) -> BBox {
        BBox::new(x, y, w, h).unwrap()
    }

    #[test]
    fn regions_become_padded_clamped_pixel_rectangles() {
        let rects = regions_to_pixels(&[bbox(0.2, 0.2, 0.4, 0.4)], 1000, 500, 0.25);
        assert_eq!(
            rects,
            [PixelRect {
                x: 100,
                y: 50,
                width: 600,
                height: 300
            }]
        );

        let edge = regions_to_pixels(&[bbox(0.0, 0.0, 1.0, 1.0)], 100, 100, 0.5);
        assert_eq!(
            edge,
            [PixelRect {
                x: 0,
                y: 0,
                width: 100,
                height: 100
            }]
        );
    }

    #[test]
    fn tiny_regions_still_cover_a_pixel() {
        let rects = regions_to_pixels(&[bbox(0.5, 0.5, 0.0001, 0.0001)], 100, 100, 0.0);
        assert!(rects[0].width >= 1 && rects[0].height >= 1);
    }

    #[test]
    fn blending_never_touches_pixels_outside_the_regions() {
        let original = RgbImage::from_fn(60, 40, |x, y| Rgb([(x * 4) as u8, (y * 6) as u8, 9]));
        let generated = flat(60, 40, [250, 0, 0]);
        let rect = PixelRect {
            x: 20,
            y: 10,
            width: 20,
            height: 20,
        };

        let out = blend(&original, &generated, &[rect], 4);

        for (x, y, pixel) in original.enumerate_pixels() {
            if !rect.contains(x, y) {
                assert_eq!(
                    out.get_pixel(x, y),
                    pixel,
                    "({x}, {y}) changed outside the region"
                );
            }
        }
    }

    #[test]
    fn deep_inside_a_region_the_generated_pixel_wins_and_edges_blend() {
        let original = flat(60, 40, [0, 0, 0]);
        let generated = flat(60, 40, [200, 200, 200]);
        let rect = PixelRect {
            x: 10,
            y: 10,
            width: 40,
            height: 20,
        };

        let out = blend(&original, &generated, &[rect], 6);

        assert_eq!(out.get_pixel(30, 20), &Rgb([200, 200, 200]));
        let edge = out.get_pixel(10, 20)[0];
        assert!(
            edge > 0 && edge < 200,
            "edge pixel should be a blend, got {edge}"
        );
        let inner = out.get_pixel(14, 20)[0];
        assert!(inner > edge, "blend should strengthen towards the middle");
    }

    #[test]
    fn several_regions_are_each_blended_in() {
        let original = flat(100, 40, [0, 0, 0]);
        let generated = flat(100, 40, [100, 100, 100]);
        let rects = [
            PixelRect {
                x: 0,
                y: 0,
                width: 30,
                height: 40,
            },
            PixelRect {
                x: 60,
                y: 0,
                width: 40,
                height: 40,
            },
        ];
        let out = blend(&original, &generated, &rects, 2);
        assert_eq!(out.get_pixel(15, 20), &Rgb([100, 100, 100]));
        assert_eq!(out.get_pixel(80, 20), &Rgb([100, 100, 100]));
        assert_eq!(out.get_pixel(45, 20), &Rgb([0, 0, 0]));
    }

    #[test]
    fn no_regions_means_no_changes() {
        let original = flat(10, 10, [5, 6, 7]);
        assert_eq!(
            blend(&original, &flat(10, 10, [200, 0, 0]), &[], 4),
            original
        );
    }

    #[test]
    fn feather_is_limited_by_the_size_of_the_region() {
        let rect = PixelRect {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        let out = blend(
            &flat(4, 4, [0, 0, 0]),
            &flat(4, 4, [255, 255, 255]),
            &[rect],
            100,
        );
        assert!(out.pixels().all(|p| p[0] > 0));
    }

    #[test]
    fn colour_matching_removes_a_global_shift_measured_outside_the_regions() {
        let original = RgbImage::from_fn(80, 80, |x, y| Rgb([(x + 60) as u8, (y + 60) as u8, 100]));
        let mut generated = RgbImage::from_fn(80, 80, |x, y| {
            Rgb([
                ((x + 60) as u8).saturating_add(25),
                ((y + 60) as u8).saturating_sub(15),
                120,
            ])
        });
        let rect = PixelRect {
            x: 30,
            y: 30,
            width: 20,
            height: 20,
        };

        assert!(match_colors(&mut generated, &original, &[rect]));

        let (after, _) = measure(&generated, &[rect]).unwrap();
        let (reference, _) = measure(&original, &[rect]).unwrap();
        for c in 0..3 {
            assert!(
                (after.mean[c] - reference.mean[c]).abs() < 2.0,
                "channel {c}"
            );
        }
    }

    #[test]
    fn colour_matching_is_skipped_when_almost_everything_is_edited() {
        let original = flat(50, 50, [100, 100, 100]);
        let mut generated = flat(50, 50, [160, 160, 160]);
        let everything = PixelRect {
            x: 0,
            y: 0,
            width: 50,
            height: 50,
        };
        assert!(!match_colors(&mut generated, &original, &[everything]));
        assert_eq!(generated.get_pixel(0, 0), &Rgb([160, 160, 160]));
    }

    #[test]
    fn colour_matching_limits_how_far_it_will_stretch_contrast() {
        let original = RgbImage::from_fn(40, 40, |x, _| Rgb([(x * 6) as u8, 0, 0]));
        let mut washed_out = RgbImage::from_fn(40, 40, |x, _| Rgb([100 + x as u8, 0, 0]));
        let rect = PixelRect {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        };
        match_colors(&mut washed_out, &original, &[rect]);
        let spread =
            i32::from(washed_out.get_pixel(39, 0)[0]) - i32::from(washed_out.get_pixel(5, 0)[0]);
        // 34 original levels, stretched by at most 1.25.
        assert!(spread <= 43, "{spread}");
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

    #[test]
    fn composite_edit_changes_only_the_regions_and_keeps_the_original_resolution() {
        let original = DynamicImage::ImageRgb8(RgbImage::from_fn(200, 120, |x, y| {
            Rgb([(x % 200) as u8, (y * 2) as u8, 60])
        }));
        // The model returns a smaller image with different content everywhere.
        let generated = DynamicImage::ImageRgb8(flat(100, 60, [250, 250, 250]));
        let region = bbox(0.4, 0.4, 0.2, 0.2);

        let result = composite_edit(&original, &generated, &[region]);

        assert_eq!(result.image.dimensions(), (200, 120));
        assert!(result.aspect_difference < 1e-6);
        let rects = regions_to_pixels(&[region], 200, 120, REGION_PADDING);
        let source = original.to_rgb8();
        let mut changed_inside = 0;
        for (x, y, pixel) in source.enumerate_pixels() {
            if rects.iter().any(|r| r.contains(x, y)) {
                changed_inside += usize::from(result.image.get_pixel(x, y) != pixel);
            } else {
                assert_eq!(
                    result.image.get_pixel(x, y),
                    pixel,
                    "({x}, {y}) changed outside the region"
                );
            }
        }
        assert!(
            changed_inside > 0,
            "the edit should be visible inside the region"
        );
    }

    #[test]
    fn composite_edit_reports_a_change_of_proportions() {
        let original = DynamicImage::ImageRgb8(flat(200, 100, [10, 10, 10]));
        let generated = DynamicImage::ImageRgb8(flat(100, 100, [10, 10, 10]));
        let result = composite_edit(&original, &generated, &[bbox(0.4, 0.4, 0.2, 0.2)]);
        // A square generated from a 2:1 original is half the expected width-to-height ratio.
        assert!(
            (result.aspect_difference - 0.5).abs() < 1e-6,
            "{}",
            result.aspect_difference
        );
    }
}
