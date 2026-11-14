//! Restoring an edited object's own colours.
//!
//! Image models often return a slightly lighter, paler object even when the rest of the photo
//! is faithful, and measuring the untouched area cannot reveal that: on a plain backdrop it is
//! all one colour. Edits here only change shape and size, never colour, so the object's colour
//! statistics in the original are the right target.
//!
//! Pixel coordinates are bounded by validated image sizes, so the casts cannot overflow.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use image::RgbImage;

use crate::composite::PixelRect;

/// A pixel counts as part of the object when it is at least this far (Euclidean, 0-255 scale)
/// from the backdrop colour of its region.
const OBJECT_DISTANCE: f32 = 48.0;
/// Fewer object pixels than this share of a region are too few to measure reliably.
const MIN_OBJECT_SHARE: f32 = 0.01;
const MIN_GAIN: f32 = 0.8;
const MAX_GAIN: f32 = 1.25;
const MIN_DEVIATION: f32 = 1.0;

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn colour(image: &RgbImage, x: u32, y: u32) -> [f32; 3] {
    let pixel = image.get_pixel(x, y);
    [
        f32::from(pixel[0]),
        f32::from(pixel[1]),
        f32::from(pixel[2]),
    ]
}

/// The typical colour of the one-pixel ring around `rect`, taken as the backdrop.
fn backdrop(image: &RgbImage, rect: PixelRect) -> [f32; 3] {
    let (x1, y1) = (rect.x + rect.width - 1, rect.y + rect.height - 1);
    let mut channels: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut take = |x: u32, y: u32| {
        let pixel = image.get_pixel(x, y);
        for (c, values) in channels.iter_mut().enumerate() {
            values.push(pixel[c]);
        }
    };
    for x in rect.x..=x1 {
        take(x, rect.y);
        take(x, y1);
    }
    for y in rect.y..=y1 {
        take(rect.x, y);
        take(x1, y);
    }
    channels.map(|mut values| {
        values.sort_unstable();
        f32::from(values[values.len() / 2])
    })
}

struct ObjectStats {
    mean: [f32; 3],
    deviation: [f32; 3],
    count: usize,
}

fn object_stats(image: &RgbImage, rect: PixelRect, backdrop: [f32; 3]) -> ObjectStats {
    let mut sum = [0.0_f64; 3];
    let mut sum_sq = [0.0_f64; 3];
    let mut count = 0_usize;
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            let c = colour(image, x, y);
            if distance(c, backdrop) < OBJECT_DISTANCE {
                continue;
            }
            count += 1;
            for i in 0..3 {
                sum[i] += f64::from(c[i]);
                sum_sq[i] += f64::from(c[i]) * f64::from(c[i]);
            }
        }
    }
    let n = count.max(1) as f64;
    let mut stats = ObjectStats {
        mean: [0.0; 3],
        deviation: [0.0; 3],
        count,
    };
    for i in 0..3 {
        let mean = sum[i] / n;
        stats.mean[i] = mean as f32;
        stats.deviation[i] = (sum_sq[i] / n - mean * mean).max(0.0).sqrt() as f32;
    }
    stats
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Pull the colours of the object inside each of `rects` in `generated` back to those of the
/// same region in `original`, leaving the backdrop alone. Returns whether any region changed.
///
/// A region is skipped when either image has too little object in it to measure.
pub fn match_object_colors(
    generated: &mut RgbImage,
    original: &RgbImage,
    rects: &[PixelRect],
) -> bool {
    let (width, height) = (
        original.width().min(generated.width()),
        original.height().min(generated.height()),
    );
    let mut corrected = vec![false; width as usize * height as usize];
    let mut changed = false;
    for &rect in rects {
        let rect = PixelRect {
            width: rect.width.min(width.saturating_sub(rect.x)),
            height: rect.height.min(height.saturating_sub(rect.y)),
            ..rect
        };
        if rect.width < 3 || rect.height < 3 {
            continue;
        }
        let area = (rect.width * rect.height) as f32;
        let (original_ground, generated_ground) =
            (backdrop(original, rect), backdrop(generated, rect));
        let before = object_stats(original, rect, original_ground);
        let after = object_stats(generated, rect, generated_ground);
        if (before.count as f32) < MIN_OBJECT_SHARE * area
            || (after.count as f32) < MIN_OBJECT_SHARE * area
        {
            continue;
        }
        let mut gain = [1.0_f32; 3];
        let mut offset = [0.0_f32; 3];
        for c in 0..3 {
            if after.deviation[c] > MIN_DEVIATION {
                gain[c] = (before.deviation[c] / after.deviation[c]).clamp(MIN_GAIN, MAX_GAIN);
            }
            offset[c] = before.mean[c] - gain[c] * after.mean[c];
        }
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                let index = y as usize * width as usize + x as usize;
                if corrected[index] {
                    continue;
                }
                let current = colour(generated, x, y);
                let strength = smoothstep(
                    (distance(current, generated_ground) - OBJECT_DISTANCE / 2.0) / OBJECT_DISTANCE,
                );
                if strength <= 0.0 {
                    continue;
                }
                corrected[index] = true;
                let pixel = generated.get_pixel_mut(x, y);
                for c in 0..3 {
                    let target = gain[c] * current[c] + offset[c];
                    pixel[c] = (current[c] + (target - current[c]) * strength)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
                changed = true;
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use image::Rgb;

    use super::*;

    const WHITE: [u8; 3] = [255, 255, 255];
    const ORANGE: [u8; 3] = [207, 87, 34];
    const FADED: [u8; 3] = [215, 123, 85];

    /// A white photo with a square of `colour` in the middle, and a little texture so the
    /// object has some spread.
    fn photo(colour: [u8; 3]) -> RgbImage {
        RgbImage::from_fn(300, 300, |x, y| {
            if (100..200).contains(&x) && (100..200).contains(&y) {
                let wobble = ((x + y) % 7) as u8;
                Rgb([
                    colour[0].saturating_sub(wobble),
                    colour[1].saturating_sub(wobble),
                    colour[2].saturating_sub(wobble),
                ])
            } else {
                Rgb(WHITE)
            }
        })
    }

    fn region() -> PixelRect {
        PixelRect {
            x: 60,
            y: 60,
            width: 180,
            height: 180,
        }
    }

    fn mean_of_square(image: &RgbImage) -> [f32; 3] {
        let mut sum = [0.0_f32; 3];
        for y in 120..180 {
            for x in 120..180 {
                for (c, total) in sum.iter_mut().enumerate() {
                    *total += f32::from(image.get_pixel(x, y)[c]);
                }
            }
        }
        sum.map(|s| s / 3600.0)
    }

    #[test]
    fn a_faded_object_gets_its_original_colour_back() {
        let original = photo(ORANGE);
        let mut generated = photo(FADED);
        let before = mean_of_square(&generated);
        assert!(match_object_colors(&mut generated, &original, &[region()]));
        let after = mean_of_square(&generated);
        let target = mean_of_square(&original);
        for c in 0..3 {
            assert!(
                (after[c] - target[c]).abs() < 6.0,
                "channel {c}: {} vs {} (was {})",
                after[c],
                target[c],
                before[c]
            );
        }
    }

    #[test]
    fn the_backdrop_is_left_alone() {
        let original = photo(ORANGE);
        let mut generated = photo(FADED);
        match_object_colors(&mut generated, &original, &[region()]);
        for (x, y) in [(70, 70), (230, 70), (70, 230), (150, 65), (65, 150)] {
            assert_eq!(generated.get_pixel(x, y).0, WHITE, "({x}, {y})");
        }
    }

    #[test]
    fn matching_colours_stay_put() {
        let original = photo(ORANGE);
        let mut generated = original.clone();
        match_object_colors(&mut generated, &original, &[region()]);
        let (a, b) = (mean_of_square(&generated), mean_of_square(&original));
        for c in 0..3 {
            assert!((a[c] - b[c]).abs() < 1.5);
        }
    }

    #[test]
    fn regions_without_an_object_are_skipped() {
        let original = RgbImage::from_pixel(300, 300, Rgb(WHITE));
        let mut generated = RgbImage::from_pixel(300, 300, Rgb([250, 250, 250]));
        let snapshot = generated.clone();
        assert!(!match_object_colors(&mut generated, &original, &[region()]));
        assert_eq!(generated, snapshot);
    }

    #[test]
    fn regions_outside_the_image_are_clipped_not_trusted() {
        let original = photo(ORANGE);
        let mut generated = photo(FADED);
        let wild = PixelRect {
            x: 250,
            y: 250,
            width: 500,
            height: 500,
        };
        assert!(!match_object_colors(&mut generated, &original, &[wild]));
    }

    #[test]
    fn overlapping_regions_do_not_correct_a_pixel_twice() {
        let original = photo(ORANGE);
        let mut once = photo(FADED);
        match_object_colors(&mut once, &original, &[region()]);
        let mut twice = photo(FADED);
        match_object_colors(&mut twice, &original, &[region(), region()]);
        assert_eq!(once, twice);
    }
}
