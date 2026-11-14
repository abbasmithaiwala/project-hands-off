//! Times each detection stage on one image: `cargo run --release -p hands-off-vision --example profile -- photo.jpg`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::path::Path;
use std::time::Instant;

use hands_off_vision::Detector;
use hands_off_vision::rfdetr::RfDetrSeg;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let started = Instant::now();
    let detector = RfDetrSeg::load(Path::new("models/rfdetr-seg-medium.onnx")).unwrap();
    println!(
        "load model      {:>7.0} ms",
        started.elapsed().as_secs_f64() * 1e3
    );
    let bytes = std::fs::read(&args[1]).unwrap();
    let started = Instant::now();
    let image = hands_off_imaging::decode_oriented(&bytes).unwrap();
    println!(
        "decode          {:>7.0} ms",
        started.elapsed().as_secs_f64() * 1e3
    );
    for run in 0..3 {
        let started = Instant::now();
        let found = detector.detect(&image).unwrap();
        println!(
            "detect #{run}      {:>7.0} ms ({} objects)",
            started.elapsed().as_secs_f64() * 1e3,
            found.len()
        );
        let started = Instant::now();
        for d in &found {
            let _ = d.mask.outline(1.5);
            let _ = d.mask.to_png().unwrap();
        }
        println!(
            "outline+png     {:>7.0} ms",
            started.elapsed().as_secs_f64() * 1e3
        );
    }
}
