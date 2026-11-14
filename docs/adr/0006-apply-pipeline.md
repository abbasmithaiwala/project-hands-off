# 6. Applying edits: client preview, region compositing, results as job artifacts

Date: 2026-11-14
Status: Accepted

## Context
Applying edits means turning draggable-control changes into a photoreal image. ADR 0003 showed that an image model follows geometric edits when it is given the original plus a rough warped preview, and that it also drifts colour and may alter pixels it was told to leave alone.

## Decisions

**The preview comes from the browser.** The editor already renders the warped preview with WebGL, so it captures exactly what the user sees (with outlines and controls hidden) and uploads it as a base64 PNG with the edits. Re-implementing the warp and rasteriser in Rust would duplicate that logic for no gain.

**The browser also reports the affected regions.** For each edited object it sends the box covering both where the object was and where the warp moved it. The server adds each edited object's own box, pads slightly, and treats the union as the only area allowed to change.

**Slider edits carry their axis.** The summariser has to say which end of a stretched object stays fixed ("keeping its left end where it is"). The server only knew the slider value, so the edit now includes the line the handle moved along.

**Compositing keeps everything else exact.** The generated image is resized to the original's resolution, its colours are corrected using only the pixels outside the edited regions (gain limited to 0.8-1.25, skipped when less than 2% of the image is untouched), and it is blended in with a feathered edge. Pixels outside the regions are copied from the original unchanged, and a test asserts that byte for byte. Edited areas are only as sharp as the model's output, which is typically around 1-1.4 megapixels.

**No mask-overlay image is sent to the model.** The runbook listed one, but the spike (ADR 0003) got acceptable edits from the original and the preview alone, and every extra image adds cost.

**Results are job artifacts until accepted.** The result PNG and its metadata are stored under `jobs/{id}/`. Accepting creates a new image from it (so it can be edited further); discarding leaves nothing behind in the image list. Unaccepted artifacts are discarded or swept after 24 hours (ADR 0007).

**Failures stay recoverable.** Validation happens before the job starts, so malformed requests fail immediately and cost nothing. A failed or cancelled job leaves the pending edits in place. Content-policy blocks (HTTP 400/403/422 with moderation markers, or a model answering in words instead of an image) are reported as `refused`, are not retried on another model, and are kept apart from key and credit problems.

## Live check
A real run (Gemini 3 Pro Image through OpenRouter, couch photo, slider at 55%) completed in about 25 s for $0.14: preparation 1 s, summary 2 s, generation 19 s, compositing 3 s in a debug build. The summariser produced a sensible instruction. In that one run the model did not actually shorten the couch (it returned a near-copy), whereas the spike run with a hand-written prompt did. The prompt then lacked the fixed-end information, which is why the axis was added to the contract. The OpenRouter account ran out of credits before the change could be re-tested live, so **how often the model follows the edit with the new prompt is not yet measured.** The pipeline, error handling and compositing are covered by tests; model reliability is the open question.

## Consequences
- Retry and a faster draft model (`draft_generator` in settings) are cheap escape hatches when the model ignores an edit.
- Because the generated image replaces the region wholesale, a model that changes the proportions is stretched back to fit and the activity log says so.
- Different JPEG decoders disagree by 1-4 levels on some pixels, so a result compared against the original with another tool shows tiny differences outside the region even though our own pixels are identical.
