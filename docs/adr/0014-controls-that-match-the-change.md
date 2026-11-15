# 14. Controls that match the change

Date: 2026-11-15
Status: Accepted

## Context
Every property was forced into one of three controls: a slider, outline points or free handles. The suggester picked from those three, so anything that was not a length, a shape or a pose became a slider. A "Tilt" property showed a bar you stretch, a "Size" showed the same bar, and moving an object was impossible. The controls did not change with the option picked, so the editor felt static and the slider's meaning ("stretch along a line") was wrong for most of what people want to do.

## Decision
Add three controls, each with a matching edit and live preview, so the control follows from what the change physically is:

| Control | For | Widget | Edit | Preview |
|---|---|---|---|---|
| `rotate` | tilt, lean, angle, how far something is open | pivot, a handle on a ring round it, turn limits in degrees | `{ degrees, pivot }`, clockwise positive | rotate about the pivot |
| `scale` | overall size | an anchor and a corner handle, multiplier limits | `{ value, pivot }` | scale evenly about the anchor |
| `move` | position | one handle on the body | `{ dx, dy }`, shares of the image size | translate |

The slider keeps its meaning (stretch one dimension along a line) and the points and handles are unchanged.

- **Placement.** Scale and move are fully determined by the object's box, so they are placed deterministically with no model call, as control points already were. Rotate is planned by the vision model because the right pivot is semantic (the base of a bottle, the hinge of a lid, otherwise the centre), with a deterministic fallback at the centre.
- **Prompts.** The suggester is told to choose the control from what the change is and never to give a tilt a slider. New versions: `suggester.v2`, `planner.v2`, `summariser.v3`. Cache keys include the versions, so earlier suggestions and plans are not reused.
- **Contract.** `rotate`, `scale` and `move` join `WidgetKind`, `Widget` and `EditChange`, validated on the server (turn within 180 degrees, multiplier above 0 and at most 4, a move within one image) and described in words for the image model ("turned 15 degrees clockwise about the point ..."). The plan schema gains nullable `pivot` and `handle` fields; `min`, `max` and `initial` are reused (degrees for rotate).
- **Frontend.** One component per kind, registered under the existing exhaustive `satisfies`. Each draws a dashed frame round the object that follows the edit, handles carry an icon, and the first-run tip is worded for the control on screen. All four rigid controls show the object's own pixels in the preview.
- **Fallback properties** (when no model answers) are now Size (scale), Position (move), Tilt (rotate) and Shape (outline).

## Consequences
- A tilt rotates and a resize grows evenly, which is what people expect, and the image model is told the real change rather than a stretch.
- Model-suggested properties vary more, which depends on the suggester following the new prompt. Judge it on the golden images in `eval/` before relying on it; a weak model still falls back to sensible controls.
- Rotation in the preview is rigid: it moves the object's pixels without redrawing the surfaces that turn into view. The image model does that on Apply, guided by the warped preview.
- A new control costs a component, a deformation and some words for the image model; the compiler enforces each step. See [extending.md](../extending.md).
