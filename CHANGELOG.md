# Changelog

All notable changes are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [semantic versioning](https://semver.org/).

## [Unreleased]

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
