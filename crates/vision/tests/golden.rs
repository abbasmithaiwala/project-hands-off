//! Runs the real RF-DETR-Seg model on the golden images and compares against the reference
//! detections recorded by the Python spike.
//!
//! The model is not in the repository. Install it with `just models`. Without it these tests
//! are skipped, unless `HANDS_OFF_REQUIRE_MODELS` is set (as it is in CI).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;
use std::time::Instant;

use hands_off_vision::Detector;
use hands_off_vision::rfdetr::RfDetrSeg;
use serde_json::Value;

const CASES: [(&str, &str); 5] = [
    ("couch", "couch"),
    ("bottle", "vase"),
    ("person_cup", "person"),
    ("burger", "sandwich"),
    ("person_pose", "person"),
];

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_model() -> Option<RfDetrSeg> {
    let dir = std::env::var("HANDS_OFF_MODEL_DIR")
        .map_or_else(|_| workspace().join("models"), PathBuf::from);
    let path = dir.join("rfdetr-seg-medium.onnx");
    if !path.exists() {
        assert!(
            std::env::var_os("HANDS_OFF_REQUIRE_MODELS").is_none(),
            "{} is required but missing",
            path.display()
        );
        eprintln!(
            "skipping: {} not installed (run `just models`)",
            path.display()
        );
        return None;
    }
    Some(RfDetrSeg::load(&path).unwrap())
}

fn iou(a: [f32; 4], b: [f32; 4]) -> f32 {
    let inter_w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let inter_h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let inter = inter_w * inter_h;
    let union = (a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - inter;
    inter / union
}

fn reference_box(reference: &Value, image: &str, label: &str) -> [f32; 4] {
    let detections = reference[image]["rfdetr_seg_m"]["dets"].as_array().unwrap();
    let found = detections.iter().find(|d| d["label"] == label).unwrap();
    let b: Vec<f32> = found["box"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    [b[0], b[1], b[2], b[3]]
}

#[test]
fn matches_the_python_reference_on_every_golden_image() {
    let Some(model) = load_model() else { return };
    let reference: Value = serde_json::from_slice(
        &std::fs::read(workspace().join("eval/spike-results/detections.json")).unwrap(),
    )
    .unwrap();

    for (name, label) in CASES {
        let image = image::open(workspace().join(format!("eval/golden/{name}.jpg"))).unwrap();
        let (w, h) = (image.width() as f32, image.height() as f32);

        let started = Instant::now();
        let detections = model.detect(&image).unwrap();
        let elapsed = started.elapsed();

        let main = detections
            .iter()
            .find(|d| d.label == label)
            .unwrap_or_else(|| {
                panic!(
                    "{name}: no {label} among {:?}",
                    detections.iter().map(|d| d.label).collect::<Vec<_>>()
                )
            });
        assert!(main.score > 0.6, "{name}: score {}", main.score);

        let found = [
            main.bbox.x * w,
            main.bbox.y * h,
            (main.bbox.x + main.bbox.width) * w,
            (main.bbox.y + main.bbox.height) * h,
        ];
        let overlap = iou(found, reference_box(&reference, name, label));
        assert!(
            overlap > 0.85,
            "{name}: box IoU with reference is {overlap}"
        );

        assert!(main.mask.area() > 0, "{name}: empty mask");
        assert!(
            elapsed.as_secs_f32() < 5.0,
            "{name}: inference took {elapsed:?}"
        );
        eprintln!(
            "{name}: {label} score {:.2} iou {overlap:.2} in {elapsed:?}",
            main.score
        );
    }
}

#[test]
fn masks_cover_the_object_inside_its_box() {
    let Some(model) = load_model() else { return };
    let image = image::open(workspace().join("eval/golden/couch.jpg")).unwrap();
    let couch = model
        .detect(&image)
        .unwrap()
        .into_iter()
        .find(|d| d.label == "couch")
        .unwrap();

    let (mw, mh) = (couch.mask.width() as f32, couch.mask.height() as f32);
    let (cx, cy) = (
        ((couch.bbox.x + couch.bbox.width / 2.0) * mw) as i32,
        ((couch.bbox.y + couch.bbox.height / 2.0) * mh) as i32,
    );
    assert!(
        couch.mask.get(cx, cy),
        "centre of the couch box should be in the mask"
    );
    assert!(
        !couch.mask.get(2, 2),
        "image corner is not part of the couch"
    );

    let outline = couch.mask.outline(1.5);
    assert!(
        (6..200).contains(&outline.len()),
        "outline has {} points",
        outline.len()
    );
}
