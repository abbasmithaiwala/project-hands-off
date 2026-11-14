//! Try the object colour restoration on two files: `recolor original.png generated.png out.png`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use hands_off_imaging::composite::PixelRect;
use hands_off_imaging::object_color::match_object_colors;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let original = image::open(&args[1]).unwrap().to_rgb8();
    let mut generated = image::open(&args[2]).unwrap().to_rgb8();
    let whole = PixelRect {
        x: 0,
        y: 0,
        width: original.width(),
        height: original.height(),
    };
    let changed = match_object_colors(&mut generated, &original, &[whole]);
    println!("changed: {changed}");
    generated.save(&args[3]).unwrap();
}
