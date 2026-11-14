# Architecture

Hands-Off lets you change a photo by dragging controls on it instead of writing a prompt. This page
explains how the pieces fit. Decisions and their trade-offs are in [adr/](adr/).

## The flow

```
upload ─► detect ─► suggest ─► place control ─► drag ─► apply ─► review ─► accept
 (API)    (local)    (model)      (local)      (browser) (models)  (browser)  (API)
```

1. **Upload.** The image is validated (type, size, dimensions), stored, and a thumbnail is made.
2. **Detect.** A local RF-DETR-Seg model finds objects and their masks; each gets a traced outline.
   Results are cached per image.
3. **Suggest.** For the object you click, a vision model looks at a crop and proposes properties
   (for example "couch length", "armrest shape") and which control suits each.
4. **Place the control.** A slider's axis or a set of handles is planned from the object's box and mask
   (and validated, repaired, or replaced by a default). Reshape controls are the object's outline reduced to its corners.
5. **Drag.** Entirely in the browser: no network call. A slider stretches the object live; reshape controls move a border.
6. **Apply.** The browser sends the edits, the affected regions and a reference image. The server summarises the
   change, asks an image model for the result, restores the object's colours, and blends it in only inside the edited regions.
7. **Review and accept.** Compare before and after, then accept it as a new version of the project, retry, or discard.

## Crates

Dependencies point inward: `api` depends on the others, never the reverse; `core` depends on nothing in the project.

| Crate | Responsibility |
|---|---|
| `core` | Domain types (images, objects, widgets, edits), errors with stable codes, configuration, storage traits. |
| `imaging` | Decoding, resizing, masks and outlines, compositing and colour correction. Pure functions. |
| `vision` | The `Detector` trait and the RF-DETR-Seg implementation on ONNX Runtime, plus model download and verification. |
| `llm` | The `LlmProvider` and `ImageGenerator` traits, the OpenRouter client, versioned prompts, strict JSON schemas and answer validation. |
| `api` | HTTP routes, jobs and event streams, storage (SQLite and filesystem), the OpenAPI document, serving the web app. |

## Web app

React, TypeScript (strict), Vite, PixiJS for the canvas, TanStack Query for server state and Zustand for UI state.
Canvas and warp logic are plain TypeScript modules in `web/src/lib`, testable without React. The API client is
generated from the server's OpenAPI document, so a type exists in one place only.

## Jobs and progress

Long operations (analysis, applying an edit) are jobs. The server keeps live job state in memory, persists the
record, and streams progress to the browser over server-sent events with replay, so a late subscriber still sees
every stage. Every job can be cancelled and has a timeout. Unaccepted results are removed after 24 hours.

## Storage

Records (images, jobs, settings) are in SQLite. Files (originals, thumbnails, masks, cached AI answers, edit
results) are blobs behind a `BlobStore` trait; the filesystem is the only implementation today. A project is a
chain of image versions: accepting an edit adds a version that points at its parent (ADR 0007).

## Errors

Errors carry a stable code (`not_found`, `unavailable`, `refused`, ...), map to HTTP statuses in one place and
reach the user as a plain sentence with a next step. Internal details never leave the server.

## Models

Local: RF-DETR-Seg for detection and masks. Hosted, all through OpenRouter: a text-and-vision model for suggestions,
control placement and summaries, and an image model for the edit itself. Each stage's model can be changed in
Settings; see [models.md](models.md).
