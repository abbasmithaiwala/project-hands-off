//! Binary masks: cleanup, outline extraction and PNG encoding.
//!
//! Pixel coordinates are bounded by the validated image dimensions (at most a few thousand),
//! so the integer and float casts below cannot overflow or lose meaningful precision.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::collections::{HashSet, VecDeque};
use std::io::Cursor;

use hands_off_core::{Error, NormalizedPoint, Result};
use image::{GrayImage, ImageFormat, ImageReader};

/// Neighbour offsets in clockwise order starting from west.
const MOORE: [(i32, i32); 8] = [
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
];
const FOUR_CONNECTED: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// A row-major binary mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    width: u32,
    height: u32,
    data: Vec<bool>,
}

impl Mask {
    /// Wrap pixel data (`true` is foreground).
    ///
    /// # Errors
    /// [`Error::Validation`] if the dimensions are zero or do not match `data`.
    pub fn new(width: u32, height: u32, data: Vec<bool>) -> Result<Self> {
        if width == 0 || height == 0 || data.len() != width as usize * height as usize {
            return Err(Error::Validation(
                "mask dimensions do not match its data".into(),
            ));
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Whether `(x, y)` is foreground. Out-of-bounds coordinates are background.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> bool {
        match (u32::try_from(x), u32::try_from(y)) {
            (Ok(x), Ok(y)) if x < self.width && y < self.height => {
                self.data[y as usize * self.width as usize + x as usize]
            }
            _ => false,
        }
    }

    /// Number of foreground pixels.
    #[must_use]
    pub fn area(&self) -> usize {
        self.data.iter().filter(|&&p| p).count()
    }

    fn index(&self, x: i32, y: i32) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// Keep only the largest 4-connected foreground region.
    #[must_use]
    pub fn largest_component(&self) -> Self {
        let mut seen = vec![false; self.data.len()];
        let mut best: Vec<usize> = Vec::new();
        for start in 0..self.data.len() {
            if !self.data[start] || seen[start] {
                continue;
            }
            let mut region = Vec::new();
            let mut queue = VecDeque::from([start]);
            seen[start] = true;
            while let Some(i) = queue.pop_front() {
                region.push(i);
                let (x, y) = (
                    (i % self.width as usize) as i32,
                    (i / self.width as usize) as i32,
                );
                for (dx, dy) in FOUR_CONNECTED {
                    let (nx, ny) = (x + dx, y + dy);
                    if self.get(nx, ny) && !seen[self.index(nx, ny)] {
                        seen[self.index(nx, ny)] = true;
                        queue.push_back(self.index(nx, ny));
                    }
                }
            }
            if region.len() > best.len() {
                best = region;
            }
        }
        let mut data = vec![false; self.data.len()];
        for i in best {
            data[i] = true;
        }
        Self {
            width: self.width,
            height: self.height,
            data,
        }
    }

    /// Fill background regions that do not touch the border.
    #[must_use]
    pub fn fill_holes(&self) -> Self {
        let mut outside = vec![false; self.data.len()];
        let mut queue = VecDeque::new();
        let (w, h) = (self.width as i32, self.height as i32);
        let border = (0..w)
            .flat_map(|x| [(x, 0), (x, h - 1)])
            .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]));
        for (x, y) in border {
            if !self.get(x, y) && !outside[self.index(x, y)] {
                outside[self.index(x, y)] = true;
                queue.push_back((x, y));
            }
        }
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in FOUR_CONNECTED {
                let (nx, ny) = (x + dx, y + dy);
                let inside = nx >= 0 && ny >= 0 && nx < w && ny < h;
                if inside && !self.get(nx, ny) && !outside[self.index(nx, ny)] {
                    outside[self.index(nx, ny)] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        Self {
            width: self.width,
            height: self.height,
            data: outside.iter().map(|&o| !o).collect(),
        }
    }

    /// Trace the outer boundary of the first region found (Moore neighbourhood tracing).
    #[must_use]
    pub fn contour(&self) -> Vec<(i32, i32)> {
        let Some(first) = self.data.iter().position(|&p| p) else {
            return Vec::new();
        };
        let start = (
            (first % self.width as usize) as i32,
            (first / self.width as usize) as i32,
        );
        let initial_backtrack = (start.0 - 1, start.1);

        let mut path = vec![start];
        let (mut current, mut backtrack) = (start, initial_backtrack);
        let mut visited = HashSet::new();
        loop {
            let offset = (backtrack.0 - current.0, backtrack.1 - current.1);
            let Some(from) = MOORE.iter().position(|&d| d == offset) else {
                break;
            };
            let next = (1..=8).find_map(|k| {
                let d = (from + k) % 8;
                let candidate = (current.0 + MOORE[d].0, current.1 + MOORE[d].1);
                self.get(candidate.0, candidate.1).then_some((candidate, d))
            });
            let Some((candidate, d)) = next else { break };
            let before = MOORE[(d + 7) % 8];
            backtrack = (current.0 + before.0, current.1 + before.1);
            current = candidate;
            if current == start && backtrack == initial_backtrack {
                break;
            }
            if !visited.insert((current, backtrack)) {
                break;
            }
            path.push(current);
        }
        path
    }

    /// Simplified outline of the main region, in normalised image coordinates.
    ///
    /// Returns an empty list for an empty mask. `tolerance_px` is the maximum allowed
    /// deviation from the traced boundary, in mask pixels.
    #[must_use]
    pub fn outline(&self, tolerance_px: f32) -> Vec<NormalizedPoint> {
        let region = self.largest_component().fill_holes();
        let mut points: Vec<(f32, f32)> = region
            .contour()
            .into_iter()
            .map(|(x, y)| (x as f32 + 0.5, y as f32 + 0.5))
            .collect();
        let first = points.first().copied();
        points.extend(first);
        let mut simplified = simplify(&points, tolerance_px);
        if simplified.len() > 1 {
            simplified.pop();
        }
        let (w, h) = (self.width as f32, self.height as f32);
        simplified
            .into_iter()
            .filter_map(|(x, y)| {
                NormalizedPoint::new((x / w).clamp(0.0, 1.0), (y / h).clamp(0.0, 1.0)).ok()
            })
            .collect()
    }

    /// Encode as a grayscale PNG (255 foreground, 0 background).
    ///
    /// # Errors
    /// [`Error::Internal`] if encoding fails.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let pixels = self.data.iter().map(|&p| if p { 255 } else { 0 }).collect();
        let gray = GrayImage::from_raw(self.width, self.height, pixels)
            .ok_or_else(|| Error::Validation("mask dimensions do not match its data".into()))?;
        let mut out = Cursor::new(Vec::new());
        gray.write_to(&mut out, ImageFormat::Png)
            .map_err(Error::internal)?;
        Ok(out.into_inner())
    }

    /// Decode a PNG produced by [`Mask::to_png`].
    ///
    /// # Errors
    /// [`Error::Validation`] if the data is not a decodable image.
    pub fn from_png(bytes: &[u8]) -> Result<Self> {
        let gray = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| Error::Validation("mask is not an image".into()))?
            .decode()
            .map_err(|_| Error::Validation("mask is not a decodable image".into()))?
            .to_luma8();
        let (width, height) = gray.dimensions();
        Self::new(
            width,
            height,
            gray.into_raw().into_iter().map(|p| p >= 128).collect(),
        )
    }
}

/// Ramer–Douglas–Peucker simplification of a closed or open polyline.
#[must_use]
pub fn simplify(points: &[(f32, f32)], tolerance: f32) -> Vec<(f32, f32)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((lo, hi)) = stack.pop() {
        let (mut worst, mut index) = (0.0, lo);
        for i in lo + 1..hi {
            let d = distance_to_segment(points[i], points[lo], points[hi]);
            if d > worst {
                worst = d;
                index = i;
            }
        }
        if worst > tolerance {
            keep[index] = true;
            stack.push((lo, index));
            stack.push((index, hi));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

fn distance_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_sq = dx * dx + dy * dy;
    if length_sq == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_sq).clamp(0.0, 1.0);
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(width: u32, height: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Mask {
        let data = (0..height)
            .flat_map(|y| (0..width).map(move |x| x >= x0 && x < x1 && y >= y0 && y < y1))
            .collect();
        Mask::new(width, height, data).unwrap()
    }

    #[test]
    fn rejects_mismatched_dimensions() {
        assert!(Mask::new(2, 2, vec![true; 3]).is_err());
        assert!(Mask::new(0, 2, vec![]).is_err());
    }

    #[test]
    fn out_of_bounds_reads_as_background() {
        let mask = rect(4, 4, 0, 0, 4, 4);
        assert!(mask.get(0, 0));
        assert!(!mask.get(-1, 0));
        assert!(!mask.get(0, 4));
    }

    #[test]
    fn largest_component_drops_specks() {
        let mut data = rect(20, 20, 2, 2, 12, 12);
        let speck = rect(20, 20, 17, 17, 18, 18);
        for i in 0..data.data.len() {
            data.data[i] |= speck.data[i];
        }
        let cleaned = data.largest_component();
        assert_eq!(cleaned.area(), 100);
        assert!(!cleaned.get(17, 17));
    }

    #[test]
    fn fill_holes_fills_enclosed_background_only() {
        let mut ring = rect(20, 20, 2, 2, 18, 18);
        let hole = rect(20, 20, 8, 8, 12, 12);
        for i in 0..ring.data.len() {
            ring.data[i] &= !hole.data[i];
        }
        let filled = ring.fill_holes();
        assert!(filled.get(10, 10));
        assert!(!filled.get(0, 0));
        assert_eq!(filled.area(), 16 * 16);
    }

    #[test]
    fn fill_holes_keeps_open_notches() {
        let mut shape = rect(20, 20, 2, 2, 18, 18);
        for y in 2..10 {
            shape.data[y * 20 + 10] = false;
        }
        assert!(!shape.fill_holes().get(10, 4));
    }

    #[test]
    fn rectangle_outline_simplifies_to_four_corners() {
        let outline = rect(40, 30, 10, 5, 30, 25).outline(1.0);
        assert_eq!(outline.len(), 4, "{outline:?}");
        let xs: Vec<f32> = outline.iter().map(|p| p.x).collect();
        assert!(xs.iter().all(|&x| (0.0..=1.0).contains(&x)));
    }

    #[test]
    fn outline_ignores_holes_and_specks() {
        let mut shape = rect(40, 40, 5, 5, 35, 35);
        let hole = rect(40, 40, 15, 15, 25, 25);
        for i in 0..shape.data.len() {
            shape.data[i] &= !hole.data[i];
        }
        shape.data[39 * 40 + 39] = true;
        assert_eq!(shape.outline(1.0).len(), 4);
    }

    #[test]
    fn empty_mask_has_no_outline() {
        assert!(rect(8, 8, 0, 0, 0, 0).outline(1.0).is_empty());
        assert!(rect(8, 8, 0, 0, 0, 0).contour().is_empty());
    }

    #[test]
    fn single_pixel_contour_terminates() {
        let mask = rect(5, 5, 2, 2, 3, 3);
        assert_eq!(mask.contour(), [(2, 2)]);
    }

    #[test]
    fn circle_outline_stays_close_to_the_true_boundary() {
        let (w, h, cx, cy, r) = (100_u32, 100_u32, 50.0_f32, 50.0_f32, 30.0_f32);
        let data = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x as f32 - cx).hypot(y as f32 - cy) <= r))
            .collect();
        let outline = Mask::new(w, h, data).unwrap().outline(1.0);
        assert!(outline.len() > 8);
        for p in outline {
            let distance = (p.x * 100.0 - cx).hypot(p.y * 100.0 - cy);
            assert!((distance - r).abs() < 2.5, "{distance}");
        }
    }

    #[test]
    fn png_round_trips() {
        let mask = rect(16, 12, 3, 2, 9, 8);
        assert_eq!(Mask::from_png(&mask.to_png().unwrap()).unwrap(), mask);
    }

    #[test]
    fn from_png_rejects_garbage() {
        assert!(Mask::from_png(b"nope").is_err());
    }

    #[test]
    fn simplify_keeps_endpoints_and_collapses_collinear_points() {
        let line: Vec<(f32, f32)> = (0..10).map(|i| (i as f32, 0.0)).collect();
        assert_eq!(simplify(&line, 0.1), [(0.0, 0.0), (9.0, 0.0)]);
        assert_eq!(simplify(&line[..2], 0.1), line[..2]);
    }
}
