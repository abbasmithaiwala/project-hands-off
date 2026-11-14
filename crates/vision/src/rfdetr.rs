//! RF-DETR-Seg inference through ONNX Runtime.
//!
//! Pixel and tensor indices are bounded by the model's fixed output shapes and the validated
//! image size, so the numeric casts below cannot overflow or lose meaningful precision.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use hands_off_core::{BBox, Error, Result};
use hands_off_imaging::mask::Mask;
use image::{DynamicImage, imageops::FilterType};
use ort::environment::Environment;
use ort::session::Session;
use ort::value::Tensor;

use crate::coco;
use crate::detector::{Detection, Detector};

const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];
const DEFAULT_THRESHOLD: f32 = 0.35;
const MASK_MAX_SIDE: f32 = 512.0;
const MAX_DETECTIONS: usize = 50;
const BOX_MARGIN: f32 = 0.02;

/// RF-DETR-Seg running on the CPU execution provider.
pub struct RfDetrSeg {
    _environment: Environment,
    session: Mutex<Session>,
    input_size: u32,
    threshold: f32,
}

impl RfDetrSeg {
    /// Load an exported RF-DETR-Seg ONNX model.
    ///
    /// # Errors
    /// Returns [`Error::Internal`] if the file cannot be read or is not a usable model, and
    /// [`Error::Validation`] if its input is not a square image tensor.
    pub fn load(path: &Path) -> Result<Self> {
        let environment = ort::init()
            .with_name("hands-off")
            .build()
            .map_err(Error::internal)?;
        let session = Session::builder(&environment)
            .map_err(Error::internal)?
            .commit_from_file(path)
            .map_err(Error::internal)?;
        let input_size = session
            .inputs()
            .first()
            .and_then(|input| input.dtype().tensor_shape().map(|shape| shape.to_vec()))
            .and_then(|shape| match shape.as_slice() {
                [_, 3, h, w] if h == w && *h > 0 => u32::try_from(*h).ok(),
                _ => None,
            })
            .ok_or_else(|| Error::Validation("model input must be a square NCHW image".into()))?;
        Ok(Self {
            _environment: environment,
            session: Mutex::new(session),
            input_size,
            threshold: DEFAULT_THRESHOLD,
        })
    }

    /// Minimum confidence for a detection to be reported.
    #[must_use]
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }
}

impl Detector for RfDetrSeg {
    fn detect(&self, image: &DynamicImage) -> Result<Vec<Detection>> {
        let size = self.input_size as usize;
        let input = Tensor::from_array(([1, 3, size, size], preprocess(image, self.input_size)))
            .map_err(Error::internal)?;

        let mut session = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        let outputs = session
            .run(ort::inputs!["input" => input])
            .map_err(Error::internal)?;
        let (box_shape, boxes) = outputs["dets"]
            .try_extract_tensor::<f32>()
            .map_err(Error::internal)?;
        let (logit_shape, logits) = outputs["labels"]
            .try_extract_tensor::<f32>()
            .map_err(Error::internal)?;
        let (mask_shape, masks) = outputs["masks"]
            .try_extract_tensor::<f32>()
            .map_err(Error::internal)?;

        let dim = |shape: &[i64], index: usize| -> Result<usize> {
            shape
                .get(index)
                .and_then(|&d| usize::try_from(d).ok())
                .ok_or_else(|| Error::Validation("unexpected model output shape".into()))
        };
        let raw = RawOutputs {
            boxes,
            logits,
            masks,
            queries: dim(box_shape, 1)?,
            classes: dim(logit_shape, 2)?,
            mask_height: dim(mask_shape, 2)?,
            mask_width: dim(mask_shape, 3)?,
        };
        decode(&raw, (image.width(), image.height()), self.threshold)
    }
}

/// Resize to the model's square input and normalise to channel-first floats.
fn preprocess(image: &DynamicImage, size: u32) -> Vec<f32> {
    let rgb = image
        .resize_exact(size, size, FilterType::Triangle)
        .to_rgb8();
    let plane = (size * size) as usize;
    let mut data = vec![0.0_f32; 3 * plane];
    for (i, pixel) in rgb.pixels().enumerate() {
        for channel in 0..3 {
            data[channel * plane + i] =
                (f32::from(pixel[channel]) / 255.0 - MEAN[channel]) / STD[channel];
        }
    }
    data
}

struct RawOutputs<'a> {
    boxes: &'a [f32],
    logits: &'a [f32],
    masks: &'a [f32],
    queries: usize,
    classes: usize,
    mask_height: usize,
    mask_width: usize,
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Turn raw model outputs into cleaned detections, most confident first.
fn decode(raw: &RawOutputs<'_>, image_size: (u32, u32), threshold: f32) -> Result<Vec<Detection>> {
    let mask_cells = raw.mask_height * raw.mask_width;
    if raw.boxes.len() != raw.queries * 4
        || raw.logits.len() != raw.queries * raw.classes
        || raw.masks.len() != raw.queries * mask_cells
    {
        return Err(Error::Validation(
            "model outputs have inconsistent sizes".into(),
        ));
    }

    let (out_width, out_height) = mask_resolution(image_size);
    let mut detections = Vec::new();
    for query in 0..raw.queries {
        let class_logits = &raw.logits[query * raw.classes..(query + 1) * raw.classes];
        let best = class_logits
            .iter()
            .enumerate()
            .filter_map(|(id, &logit)| {
                let id = u16::try_from(id).ok()?;
                coco::label(id).map(|label| (id, label, sigmoid(logit)))
            })
            .max_by(|a, b| a.2.total_cmp(&b.2));
        let Some((class_id, label, score)) = best else {
            continue;
        };
        if score < threshold {
            continue;
        }

        let b = &raw.boxes[query * 4..query * 4 + 4];
        let (x0, y0) = (
            (b[0] - b[2] / 2.0).clamp(0.0, 1.0),
            (b[1] - b[3] / 2.0).clamp(0.0, 1.0),
        );
        let (x1, y1) = (
            (b[0] + b[2] / 2.0).clamp(0.0, 1.0),
            (b[1] + b[3] / 2.0).clamp(0.0, 1.0),
        );
        let Ok(bbox) = BBox::new(x0, y0, x1 - x0, y1 - y0) else {
            continue;
        };

        let logits = &raw.masks[query * mask_cells..(query + 1) * mask_cells];
        let mask = build_mask(
            logits,
            (raw.mask_width, raw.mask_height),
            (out_width, out_height),
            &bbox,
        )?;
        if mask.area() == 0 {
            continue;
        }
        detections.push(Detection {
            class_id,
            label,
            score,
            bbox,
            mask,
        });
    }
    detections.sort_by(|a, b| b.score.total_cmp(&a.score));
    detections.truncate(MAX_DETECTIONS);
    Ok(detections)
}

fn mask_resolution((width, height): (u32, u32)) -> (u32, u32) {
    let scale = (MASK_MAX_SIDE / width.max(height) as f32).min(1.0);
    (
        ((width as f32 * scale).round() as u32).max(1),
        ((height as f32 * scale).round() as u32).max(1),
    )
}

/// Upsample mask logits bilinearly, keep pixels inside the (slightly grown) box, then clean up.
fn build_mask(
    logits: &[f32],
    (grid_w, grid_h): (usize, usize),
    (out_w, out_h): (u32, u32),
    bbox: &BBox,
) -> Result<Mask> {
    let (left, right) = (
        (bbox.x - BOX_MARGIN) * out_w as f32,
        (bbox.x + bbox.width + BOX_MARGIN) * out_w as f32,
    );
    let (top, bottom) = (
        (bbox.y - BOX_MARGIN) * out_h as f32,
        (bbox.y + bbox.height + BOX_MARGIN) * out_h as f32,
    );

    let sample = |gx: f32, gy: f32| -> f32 {
        let (gx, gy) = (
            gx.clamp(0.0, (grid_w - 1) as f32),
            gy.clamp(0.0, (grid_h - 1) as f32),
        );
        let (x0, y0) = (gx.floor() as usize, gy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(grid_w - 1), (y0 + 1).min(grid_h - 1));
        let (fx, fy) = (gx - x0 as f32, gy - y0 as f32);
        let at = |x: usize, y: usize| logits[y * grid_w + x];
        let top_row = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
        let bottom_row = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
        top_row * (1.0 - fy) + bottom_row * fy
    };

    let mut data = Vec::with_capacity(out_w as usize * out_h as usize);
    for y in 0..out_h {
        for x in 0..out_w {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let inside = px >= left && px <= right && py >= top && py <= bottom;
            let gx = px / out_w as f32 * grid_w as f32 - 0.5;
            let gy = py / out_h as f32 * grid_h as f32 - 0.5;
            data.push(inside && sample(gx, gy) > 0.0);
        }
    }
    Ok(Mask::new(out_w, out_h, data)?
        .largest_component()
        .fill_holes())
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    const COUCH: usize = 63;
    const CLASSES: usize = 91;

    /// One query predicting `class` at `score_logit` with a square mask over `region` of the grid.
    fn one_query(
        class: usize,
        score_logit: f32,
        bbox: [f32; 4],
        region: (usize, usize, usize, usize),
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let mut logits = vec![-10.0; CLASSES];
        logits[class] = score_logit;
        let mut mask = vec![-5.0; 8 * 8];
        for y in region.1..region.3 {
            for x in region.0..region.2 {
                mask[y * 8 + x] = 5.0;
            }
        }
        (bbox.to_vec(), logits, mask)
    }

    fn raw<'a>(boxes: &'a [f32], logits: &'a [f32], masks: &'a [f32]) -> RawOutputs<'a> {
        RawOutputs {
            boxes,
            logits,
            masks,
            queries: boxes.len() / 4,
            classes: CLASSES,
            mask_height: 8,
            mask_width: 8,
        }
    }

    #[test]
    fn preprocess_normalises_with_imagenet_statistics_channel_first() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([255, 0, 0])));
        let data = preprocess(&image, 2);
        assert_eq!(data.len(), 12);
        assert!((data[0] - (1.0 - MEAN[0]) / STD[0]).abs() < 1e-5);
        assert!((data[4] - (0.0 - MEAN[1]) / STD[1]).abs() < 1e-5);
        assert!((data[8] - (0.0 - MEAN[2]) / STD[2]).abs() < 1e-5);
    }

    #[test]
    fn decodes_a_confident_detection_with_box_label_and_mask() {
        let (boxes, logits, masks) = one_query(COUCH, 4.0, [0.5, 0.5, 0.5, 0.5], (2, 2, 6, 6));
        let found = decode(&raw(&boxes, &logits, &masks), (200, 100), 0.35).unwrap();

        assert_eq!(found.len(), 1);
        let d = &found[0];
        assert_eq!((d.class_id, d.label), (63, "couch"));
        assert!(d.score > 0.95);
        assert!((d.bbox.x - 0.25).abs() < 1e-6 && (d.bbox.width - 0.5).abs() < 1e-6);
        assert_eq!((d.mask.width(), d.mask.height()), (200, 100));
        assert!(d.mask.get(100, 50));
        assert!(!d.mask.get(5, 5));
    }

    #[test]
    fn drops_detections_below_the_threshold() {
        let (boxes, logits, masks) = one_query(COUCH, -1.0, [0.5, 0.5, 0.5, 0.5], (2, 2, 6, 6));
        assert!(
            decode(&raw(&boxes, &logits, &masks), (100, 100), 0.35)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn ignores_classes_without_a_coco_label() {
        let (boxes, logits, masks) = one_query(12, 6.0, [0.5, 0.5, 0.5, 0.5], (2, 2, 6, 6));
        assert!(
            decode(&raw(&boxes, &logits, &masks), (100, 100), 0.35)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn mask_is_confined_to_the_box() {
        let (boxes, logits, masks) = one_query(COUCH, 4.0, [0.25, 0.25, 0.3, 0.3], (0, 0, 8, 8));
        let found = decode(&raw(&boxes, &logits, &masks), (100, 100), 0.35).unwrap();
        let mask = &found[0].mask;
        assert!(mask.get(25, 25));
        assert!(!mask.get(90, 90));
    }

    #[test]
    fn clamps_boxes_that_spill_past_the_image() {
        let (boxes, logits, masks) = one_query(COUCH, 4.0, [0.9, 0.9, 0.5, 0.5], (4, 4, 8, 8));
        let found = decode(&raw(&boxes, &logits, &masks), (100, 100), 0.35).unwrap();
        let bbox = found[0].bbox;
        assert!(bbox.x + bbox.width <= 1.0 + 1e-6 && bbox.y + bbox.height <= 1.0 + 1e-6);
    }

    #[test]
    fn skips_detections_whose_mask_is_empty() {
        let (boxes, logits, mut masks) = one_query(COUCH, 4.0, [0.5, 0.5, 0.5, 0.5], (2, 2, 6, 6));
        masks.fill(-5.0);
        assert!(
            decode(&raw(&boxes, &logits, &masks), (100, 100), 0.35)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn orders_by_confidence() {
        let (mut boxes, mut logits, mut masks) =
            one_query(COUCH, 1.0, [0.3, 0.3, 0.2, 0.2], (1, 1, 3, 3));
        let (b2, l2, m2) = one_query(1, 5.0, [0.7, 0.7, 0.2, 0.2], (5, 5, 7, 7));
        boxes.extend(b2);
        logits.extend(l2);
        masks.extend(m2);
        let found = decode(&raw(&boxes, &logits, &masks), (100, 100), 0.35).unwrap();
        assert_eq!(
            found.iter().map(|d| d.label).collect::<Vec<_>>(),
            ["person", "couch"]
        );
    }

    #[test]
    fn rejects_inconsistent_output_sizes() {
        let (boxes, logits, masks) = one_query(COUCH, 4.0, [0.5, 0.5, 0.5, 0.5], (2, 2, 6, 6));
        let mut bad = raw(&boxes, &logits, &masks);
        bad.mask_width = 7;
        assert!(decode(&bad, (100, 100), 0.35).is_err());
    }

    #[test]
    fn mask_resolution_caps_the_long_side_and_keeps_aspect() {
        assert_eq!(mask_resolution((2048, 1024)), (512, 256));
        assert_eq!(mask_resolution((300, 200)), (300, 200));
        assert_eq!(mask_resolution((1, 4000)), (1, 512));
    }
}
