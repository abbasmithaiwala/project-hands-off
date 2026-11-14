# Vision pipeline

Object detection and segmentation run locally in Rust through ONNX Runtime (`ort`). No image leaves the machine for this step.

## What it does

`POST /api/v1/images/{id}/analyze` starts a job that:

1. Decodes the original and applies its EXIF orientation, so outlines line up with what a viewer shows.
2. Runs RF-DETR-Seg Medium (Apache-2.0) to get boxes, class labels and per-object masks.
3. Cleans each mask (largest connected region, holes filled) and traces a simplified outline polygon.
4. Streams one `object_found` event per object, stores each mask as a PNG, and caches the result.

Analysing the same image again returns the cached result immediately. Fetch results with `GET /images/{id}/objects` and `GET /images/{id}/objects/{object_id}/mask`.

Coordinates are normalised (`0..1`), so outlines apply to any rendering size. Masks are at most 512 px on the long side.

## Installing the model

The ONNX file is not stored in the repository.

```sh
just models   # exports models/rfdetr-seg-medium.onnx (needs Python; about 130 MB)
```

Alternatives:

- Put a ready-made `rfdetr-seg-medium.onnx` in `HANDS_OFF_MODEL_DIR` (default `models/`).
- Set `HANDS_OFF_RFDETR_SEG_URL` (and optionally `HANDS_OFF_RFDETR_SEG_SHA256`) and the server downloads it on first use, verifying the checksum and logging progress to the job's event stream.

Checksums are recorded in `models/manifest.json` the first time a file is used. If a file later changes, the server refuses to load it. The server starts without the model; analysis fails with a `503` and a message explaining how to install it.

## Performance

Measured on an Apple M4, CPU execution provider, release build, after warm-up: about 260-290 ms per image, including preprocessing, inference and mask cleanup. The first call also loads the model (about a second). CoreML is not used because it is slower than CPU for this model (see ADR 0004).

## Tests

- Unit tests cover preprocessing, output decoding, mask upsampling and the model manager.
- `crates/vision/tests/golden.rs` runs the real model on the golden images in `eval/golden/` and checks the boxes against the reference detections from the Python spike (IoU above 0.85; in practice 0.98 or higher). It is skipped when the model is not installed, unless `HANDS_OFF_REQUIRE_MODELS` is set.

## Not yet implemented

- Click-to-segment with SAM (kept out of the MVP path; see ADR 0004).
- An optional YOLO11 detector behind the same `Detector` trait.
