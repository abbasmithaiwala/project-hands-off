# 4. Detector and segmentation: RF-DETR-Seg as default

Date: 2026-11-14
Status: Accepted (Gate 2 passed); supersedes the "boxes plus SAM" default in `plan.md`

## Context
Hands-Off needs an outline mask per object and a fast first analysis. We compared RF-DETR Medium, RF-DETR-Seg Medium and YOLO11m-seg on the five golden images, and SAM 2.1 tiny with box prompts. ONNX export and benchmarking used ONNX Runtime on an Apple M4 (the same runtime the Rust `ort` crate wraps).

## Results
- **Detection:** all models found the right objects (couch, sandwich, person, bowl for the glass, vase for the perfume bottle). `bottle` and `cup` are not reliable COCO labels for these objects, so open-vocabulary detection is still needed later.
- **Masks:** RF-DETR-Seg masks were clean on every image. YOLO11m-seg masks were comparable. SAM 2.1 tiny was slightly tighter but not needed.
- **ONNX export:** RF-DETR (detection and segmentation) exports with a single `export()` call and runs under ONNX Runtime without changes.

| Model (ONNX Runtime, median) | CPU | CoreML |
|---|---|---|
| RF-DETR Medium (576 px) | 268 ms | 396 ms |
| RF-DETR-Seg Medium (432 px) | 342 ms | 1016 ms |
| YOLO11m-seg (640 px) | 334 ms | 63 ms |

SAM 2.1 tiny in PyTorch took about 800 ms per box prompt; an ONNX export was not attempted.

## Decision
- The default is **RF-DETR-Seg Medium** on the **CPU execution provider**. It gives boxes and masks in one pass at about 0.35 s, which is acceptable because analysis runs once per image and is cached. Apache-2.0 licence.
- **CoreML is not used for RF-DETR**: it is slower than CPU for the transformer graph.
- **SAM is removed from the MVP critical path** and kept for click-to-segment and open-vocabulary objects (VLM boxes to masks), where its ONNX export still has to be verified in Phase 3.
- **YOLO11 stays optional** and user-downloaded behind the `Detector` trait. It is about 5x faster with CoreML on Apple Silicon, but the AGPL licence rules out bundling it.

## Consequences
- One model instead of two in the default path, so a simpler pipeline and fewer downloads.
- Phase 3 must confirm the numbers through the Rust `ort` crate (they were measured with Python ONNX Runtime) and verify SAM ONNX export if click-to-segment is to ship.
- Linux or CUDA users may see different performance; benchmark again when targeting them.

## Addendum: confirmed in Rust (Phase 3)

The Rust `ort` implementation reproduces the Python reference: box IoU of 0.98 or higher on every golden image. Release-build latency on the M4, after warm-up, is about 260-290 ms per image including preprocessing and mask cleanup. Two build notes:

- `ort` 2.0.0-rc.15 is a release candidate; the version is pinned in `Cargo.toml`.
- The release profile has LTO off: thin LTO drops `___isPlatformVersionAtLeast`, which ONNX Runtime's prebuilt macOS libraries need, and the link fails.

SAM click-to-segment and the optional YOLO11 detector are deferred (they are outside the MVP path).
