# Changelog

All notable changes are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [semantic versioning](https://semver.org/).

## [Unreleased]

### Added
- Controls that match the change. Tilt is now a ring you turn, overall size is a corner handle that resizes
  evenly, and position is a handle that carries the object, instead of every property being a slider. Each shows a
  dashed frame that follows the edit, and the first-run tip describes the control on screen. The API gains
  `rotate`, `scale` and `move` widgets and edits (ADR 0014).

### Changed
- The result is now the image model's output, resized to the original's resolution. Region blending and colour
  matching are removed (they clipped objects the model drew larger than the preview), and `regions` is gone from the
  edit request (ADR 0015).
- Property suggestions, widget planning and edit summaries use new prompt versions (`suggester.v2`, `planner.v2`,
  `summariser.v4`) that choose the control from what the change physically is. Cached suggestions and plans are
  refreshed.
- Image prompts (`generator-*.v3`) require the edited object to be shown complete, never cropped or cut off at the
  edited area's border, and to turn the same way and amount as the preview. The summariser states every turn as
  both clockwise or anticlockwise and which way the top leans, since image models confuse the two.
- A single tilt uses its own image prompt (`generator-with-rotation.v1`) that treats the second image as the target pose and
  states, from the angle, which side the top swings to, instead of the general resize-oriented text.
- The reference image sent to the image model (`jobs/<id>/preview.jpg`) and the image it returned
  (`jobs/<id>/generated`), are kept with the job so a wrong result can be compared with what the model was shown
  and what it drew.
- The default image model for generation and retries is `google/gemini-3-pro-image` (ADR 0013).
- The project licence is now the MIT License (it was Apache-2.0).

## [0.1.0] - 2026-11-14

First release.

### Added
- Edit a photo by dragging controls on it instead of writing a prompt: pick an object, pick a property, drag a
  slider, a handle or a point, press Apply, review the result against the original, then accept it.
- Local object detection and masks with RF-DETR-Seg on ONNX Runtime (CPU, about half a second per photo); the
  model is downloaded on first run and never bundled.
- AI-assisted properties and control placement for each object, through OpenRouter, with validation and
  deterministic fallbacks so editing never dead-ends.
- Image generation through OpenRouter with a drawn outline for reshaping and a warped preview for sliders and
  handles; the result is colour-corrected and blended in only inside the edited regions.
- Version history with undo and redo, saved projects (rename, delete, reopen after reload), and export of the image
  and a JSON edit log.
- First-run experience: a sample photo, short tips, and clear recovery when analysis fails or finds nothing.
- Settings for the OpenRouter key and for the model used at each stage; a demo mode that works without a key.
- A typed REST API described by OpenAPI, with a generated TypeScript client.
- One-command start with Docker (`docker compose up --build`), a health check, and release binaries.

### Security
- The API key stays on the server and is never returned or logged; uploads are limited by size, type and
  dimensions; strict CORS and hardening headers.
