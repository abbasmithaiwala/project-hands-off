# Hands-Off — Open-Source "Dynamic UI on Canvas" Image Editor

An open-source, self-hostable take on Adobe's *Project Hands On* (Adobe MAX Sneaks 2026, presented by Celso Gomes). You edit photos by **manipulating controls placed on objects** (drag a handle, slide a length, move pose points) instead of writing a text prompt.

---

## 1. Analysis

### The core loop
1. **Open a photo.** The app runs two analysis passes:
   1. **Detect objects.** Each object gets an outline (orange mask) on the canvas.
   2. **Property suggestion.** For each object, a model proposes which properties make sense for it, e.g. perfume bottle → `shape`; couch → `couch length / height / arm shape / seat shape`; person → `arms position`, `biceps size`, `expression`, `head orientation`; beer → `beer volume`, `foam`; burger → `burger height`, `cheese`.
2. **User picks a property.** A second analysis pass positions a **dynamic UI widget** on the object. The widget type depends on the property:
   - **Control-point polygon**: bottle outline, drag corners to make a diamond.
   - **1-D slider / handle**: couch length, beer volume, burger height, with a handle at the object's edge.
   - **Skeleton / puppet-warp points**: arm position (a "light version of puppet warp").
   - **Two independent handles**: left bicep small, right bicep large.
   - **3-D head mesh**: head orientation, rotate around.
   - **Expression points**: face control points, e.g. raise mouth corners to smile.
3. **Instant client-side preview.** A fast, approximate warp as you drag. The presenter stresses: *"it's not perfect, it's just a preview."*
4. **Apply.** The structured instructions (object, property, control-point origins and deltas) are sent to the **image generation model**. Result takes seconds to tens of seconds, and the demo fast-forwards. Accept / Cancel buttons follow.
5. **Batch edits.** Several edits on different objects (beer volume + burger height + head rotation) go out as **one request** with multiple instructions.
6. **Observable reasoning.** A side "Output" panel streams the model's reasoning and the JSON "dynamic UI response" (origin x/y, delta). It is a debug / transparency log.

### Key insight
The UI is **generated per image and per object by a VLM** rather than being hard-coded. Text prompts are bad at *precise* spatial intent ("diamond-shaped with this exact geometry"). Geometry-as-input plus a generative model that "repairs" the rough preview into a photorealistic result is the whole trick.

### Where the user's requirements map

| Requirement | Role in the system |
|---|---|
| Object detection model | Stage 1: find and mask objects, give anchor geometry for widgets |
| AI APIs for image generation | Stage 5: turn control-point deltas + preview into a photoreal edit |
| Summarising the changes | Turn raw deltas into a human-readable change list and a precise generation prompt, plus edit-history labels |
| Typed API (standard) + optional Decisions API | Rust ↔ TypeScript contract so schemas never drift; Decisions API is a server-driven alternative |
| OpenRouter | Single gateway to swap VLM and image models per stage |
| Rust backend | Image processing, ONNX inference, orchestration, streaming |

---

## 2. Product scope

### MVP (v0.1)
- Upload image, auto-detect and outline objects (hover to highlight, click to select).
- Per-object property suggestions (VLM), exposed through the **standard typed REST API**; the optional Decisions API is added behind a Cargo feature / config toggle.
- Three widget types: **control-point polygon**, **slider**, **multi-handle (puppet-warp-lite)**.
- Live client-side warp preview.
- Apply → image edit via OpenRouter → Accept / Cancel, undo / redo.
- Batch several edits into one generation call.
- Edit history with auto-generated change summaries.
- Bring-your-own OpenRouter key; model picker per stage.
- Docker one-command self-host.

### v0.2
- Pose widgets with real keypoints (arms, legs); face expression via landmarks.
- Click-to-segment (SAM) for objects the detector missed; text-labelled open-vocabulary detection.
- Mask-protected compositing (untouched pixels stay byte-identical).
- Project save/load (SQLite), export with edit log.

### v0.3+ (stretch)
- 3-D head orientation mesh.
- Multiple candidate results (n=2–4) with side-by-side picker.
- Plugin API for new widget types and property packs.
- Optional local generation backends (ComfyUI / diffusers) behind the same trait.
- Collaborative sessions.

### Non-goals (for now)
- Full Photoshop-style layer editor, video editing, training custom models.

---

## 3. Architecture

```
┌────────────────────────── Browser (React + TS) ──────────────────────────┐
│ Canvas (Pixi/Konva)  │ Widget renderers │ WebGL warp preview │ History  │
│        ▲ generated TS client (from OpenAPI) + SSE event stream           │
└────────┼─────────────────────────────────────────────────────────────────┘
         │ HTTP/JSON + SSE
┌────────▼──────────────────── Rust backend (axum + tokio) ────────────────┐
│ API layer (utoipa)                                                       │
│   ├─ Image service     decode/resize/mask ops (image, fast_image_resize) │
│   ├─ Vision service    ONNX Runtime (ort): detector + segmenter + pose   │
│   ├─ Routes            /api/v1 (standard) + /api/decisions (optional)    │
│   ├─ LLM gateway       OpenRouter client (reqwest), structured outputs   │
│   │     ├─ Suggester   (fast cheap VLM)  → properties per object         │
│   │     ├─ UI planner  (VLM)             → widget geometry               │
│   │     ├─ Summariser  (LLM/VLM)         → change summary + gen prompt   │
│   │     └─ Generator   (image-edit model)→ edited image                  │
│   ├─ Job manager       async jobs, cancellation, progress over SSE       │
│   ├─ Cache             analysis keyed by image hash (moka / disk)        │
│   └─ Store             SQLite (sqlx) + blob storage (fs / S3)            │
└──────────────────────────────────────────────────────────────────────────┘
```

### Pipeline per user action

```
Upload ─► [A] Detect+segment (local, Rust/ONNX, ~100–300ms)
       ─► [B] Suggest properties per object (VLM, lazy, cached)
Click property ─► [C] Plan widget (VLM, given crop + mask + property) 
Drag ─► [D] Client-side preview warp (WebGL, 60fps, no network)
Apply ─► [E] Summarise deltas → edit instruction (LLM)
       ─► [F] Generate edit (image model; inputs: original, mask, preview, instruction)
       ─► [G] Post-process: mask-feathered composite, size match, save version
Accept/Cancel
```

---

## 4. Technology decisions

### Backend: Rust
- **axum + tokio + tower-http**: HTTP, SSE, CORS, compression, tracing.
- **reqwest + serde**: OpenRouter client. OpenRouter is OpenAI-compatible, so one thin client covers every model. Write a small `LlmProvider` trait so a direct Anthropic / Google / local backend can slot in later.
- **ort** (ONNX Runtime bindings): run detection, segmentation and pose models in-process, with no Python in the hot path. CoreML / CUDA execution providers are opt-in.
- **image + fast_image_resize + imageproc**: decode, resize, mask morphology, compositing.
- **sqlx (SQLite)** for projects and history; **moka** for in-memory cache; **tracing** + OpenTelemetry.
- **Honest note on speed:** Rust makes the *local* parts (decode, inference, compositing, orchestration) fast and cheap to host. End-to-end latency is dominated by the VLM and image-generation calls, which take seconds. The real latency wins are architectural: client-side preview, lazy and cached analysis, parallel calls, small models for planning, streaming progress. See §8.

### API style: standard typed REST (default) or Decisions API (optional)
The frontend ↔ backend contract comes in two styles. The **standard typed API is the default**; the **Decisions API is an optional alternative** you can switch on. Both sit on the same Rust core (detector, VLM calls via OpenRouter, generator) and produce the same edits.

**1. Standard typed API (default)**
- Conventional REST endpoints, one per step (§6): `analyze`, `properties`, `widget`, `edits`.
- **`utoipa`** generates the OpenAPI spec from Rust types; **`openapi-typescript` / `orval` / `hey-api`** generates a fully typed TS client and React Query hooks. Rust types are the source of truth.
- The **client drives the flow**: it decides which endpoint to call next (analyze → pick object → fetch properties → fetch widget → apply).
- Model outputs (properties, widget geometry, change summary) use OpenRouter structured outputs. Schemas are defined once with `serde` + `schemars` and reused for the model schema, validation and TS types.
- Alternatives to utoipa: **`specta` + `rspc`** (tRPC-like) or **`ts-rs`** (types only).

**2. Decisions API (optional)**
- A **server-driven** style: the backend exposes decision endpoints that return a typed, validated **decision** (a discriminated union), and the frontend simply renders it.

| Decision | Made by | Typed output |
|---|---|---|
| `DetectDecision` | detector (+ VLM boxes) | objects: label, bbox, mask, score |
| `PropertyDecision` | suggester VLM | which properties apply, which widget type each uses |
| `WidgetDecision` | planner VLM | widget geometry: anchors, axis, range, handles, clamped to the mask |
| `EditDecision` | summariser LLM | change summary, generation prompt, strength, regions to protect |
| `AcceptDecision` | user / auto-check | accept, retry with another model, or revert |

- **Validate, then repair**: parse → schema check → clamp coordinates into the mask → one retry with the error fed back → fall back to a default widget.
- Decisions are **logged and replayable** (image hash, model ID, prompt version), which powers the Output panel, evals, and regression checks when swapping models.
- The frontend renders with exhaustive `switch`es over the decision enums, so a new widget type won't compile until it has a renderer.

**Making it switchable**
- A Rust trait (`ApiFlow`/`EditStrategy`) with two route sets: `/api/v1/...` (standard) and `/api/decisions/...` (decisions). Both call the same internal services, so there is one implementation of detection, VLM calls and generation.
- Selected by config flag and overridable per session in the UI. The decisions routes are compiled in behind a Cargo feature (`decisions-api`), so a minimal build ships without them.
- Both styles are covered by the same eval set so you can compare them on identical images.

### Object detection / segmentation
The video's outlines are **masks**, not boxes. The default detector is a segmentation model that returns masks directly (ADR 0004); SAM refines or extends it where needed.

All detectors sit behind a Rust `Detector` trait, so they are swappable via config.

| Option | Role |
|---|---|
| **RF-DETR-Seg Medium (ONNX)**, Apache-2.0 | **Default detector and segmenter.** Boxes, masks, labels, scores for the 80 COCO classes (the perfume bottle came back as `vase`, the glass as `bowl`, burger as `sandwich`). Use only the Apache-2.0 sizes (Nano to Large); the XL/2XL sizes use Roboflow's platform licence. Use the CPU execution provider: CoreML is slower for this model |
| **YOLO11 (det or seg, ONNX)**, AGPL-3.0 | **Optional, user-downloaded** alternative behind the same trait; never bundled. Possibly faster on plain CPU. Users take on the AGPL decision themselves |
| **SAM 2 / MobileSAM (ONNX encoder+decoder)** | **Optional.** Click-to-select for anything the detector missed and masks for VLM-proposed boxes. ONNX export still to be verified (Phase 3) |
| **VLM bounding boxes → SAM** | Open-vocabulary: ask the VLM "list editable objects with boxes", feed the boxes to SAM. Covers what COCO detectors miss (cheese, foam) |
| **Grounding DINO / OWL-ViT / Florence-2** | Local open-vocabulary alternative if you don't want VLM boxes |
| **Pose model (RF-DETR Keypoint / RTMPose) + face landmarks (MediaPipe FaceMesh ONNX)** | v0.2: arm keypoints, expression points |

Recommendation (confirmed by the Phase 1 spike, see ADR 0004): MVP = **RF-DETR-Seg Medium** on the CPU execution provider for boxes and masks in one pass (about 0.35 s per image, cached). SAM is kept only for click-to-segment and open-vocabulary objects (VLM boxes to masks).

**Why RF-DETR over YOLO11 for this project:** (1) licence: this is a self-hostable open-source tool and may run as a network service, where YOLO11's AGPL-3.0 is a problem; (2) the detector's accuracy matters less because SAM handles the masks; (3) published benchmarks favour RF-DETR on COCO accuracy, but they are Roboflow's own TensorRT/T4 numbers, so they don't tell us how it runs on laptops.

**Verified in the Phase 1 spike (ADR 0004):** RF-DETR and RF-DETR-Seg export to ONNX and run under ONNX Runtime; box and mask quality is good on the golden images; CPU latency is 270-340 ms and CoreML is slower for RF-DETR. **Still to verify in Phase 3:** the same numbers through the Rust `ort` crate, and SAM ONNX export if click-to-segment ships.

### OpenRouter (model switching)
- One config file maps **stage → model ID**, overridable per request from the UI:
  `suggester`, `ui_planner`, `summariser`, `generator`.
- Use cheap, fast multimodal models for suggester and planner, a stronger model for the summariser if needed, and an image-editing-capable model for the generator. **Spike result (ADR 0003):** default generator was `google/gemini-3-pro-image` always given the warped preview as a reference ($0.14, about 18 s per call), now `google/gemini-3.1-flash-lite-image` for cost (ADR 0008, pending benchmark); `google/gemini-3.1-flash-image` as a faster draft option; `openai/gpt-5-image-mini` rejected because it re-renders the whole scene. **Model IDs change quickly; look up current ones on openrouter.ai/models** rather than hard-coding.
- Use **structured outputs (JSON schema)** for suggester / planner / summariser. Image generation goes through the chat-completions endpoint with image output modalities on models that support image editing.
- Add retries with backoff, fallback model list per stage, per-request cost and latency logging (OpenRouter returns usage).
- The API key is stored server-side (env / encrypted DB) or supplied per session; never shipped to the browser.

### Frontend
- **React + Vite + TypeScript**, TanStack Query, Zustand.
- Canvas: **PixiJS** (WebGL, good for mesh warp and masks) or Konva for simpler handles. Recommend Pixi for the warp preview.
- Preview warping: **Moving Least Squares** or triangulated mesh deformation on the object's masked crop, rendered in WebGL; slider widgets = region stretch.
- Reasoning / output log panel like the original (SSE-fed), which also helps debugging.

### Alternatives worth noting
- Tauri desktop wrapper later (same Rust core, no server).
- Svelte/Solid instead of React is fine; the generated client is framework-agnostic.

---

## 5. Core data contracts (sketch)

```jsonc
// Object found in image
{ "id": "obj_3", "label": "couch", "bbox": [x,y,w,h], "mask_url": "/blob/...", "score": 0.94 }

// Property suggestion
{ "object_id": "obj_3",
  "properties": [
    { "key": "length", "label": "Couch length", "widget": "slider" },
    { "key": "shape",  "label": "Seat shape",   "widget": "control_points" } ] }

// Widget plan (VLM output, validated against schema)
{ "object_id": "obj_3", "property": "length", "widget": "slider",
  "axis": [[x1,y1],[x2,y2]], "range": [0.5, 1.5], "anchors": [...] }

// User edit (what the client sends on Apply)
{ "edits": [
  { "object_id": "obj_3", "property": "length", "widget": "slider",
    "origin": [[..]], "target": [[..]], "scalar": 0.55 },
  { "object_id": "obj_7", "property": "volume", ... } ],
  "preview_png": "<optional warped preview>" }
```

Coordinates are normalised (0–1) so the preview resolution and the full-resolution image never mismatch.

---

## 6. API surface (v1)

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/images` | Upload; returns `image_id` and a hash |
| POST | `/api/images/{id}/analyze` | Detect + segment; SSE emits objects as they're found |
| POST | `/api/images/{id}/objects/{oid}/properties` | VLM property suggestions (lazy, cached) |
| POST | `/api/images/{id}/objects/{oid}/widget` | Plan widget for a chosen property |
| POST | `/api/images/{id}/edits` | Apply a batch; returns `job_id` |
| GET | `/api/jobs/{id}/events` | SSE: progress, reasoning log, result |
| DELETE | `/api/jobs/{id}` | Cancel |
| GET | `/api/images/{id}/history` | Versions with change summaries |
| POST | `/api/versions/{id}/accept` / `revert` | Commit or undo |
| GET/PUT | `/api/config/models` | Stage → model mapping |

---

## 7. Generation strategy (the part that decides quality)

The edit model needs more than "make the couch shorter". Send:
1. **Original image.**
2. **Mask** of edited object(s), as a second image or a highlighted overlay.
3. **Warped preview** as a *geometry reference*: "match the shape/pose in the reference, but keep photorealistic texture, lighting and identity."
4. **Summarised instruction**, from the LLM summariser (turns deltas into e.g. *"Shorten the orange couch to a single-seat armchair; keep cushions, plant and lighting unchanged."*).

Post-processing:
- **Mask-feathered composite**: blend the generated pixels into the original only around the edited region (plus a dilation margin) so the rest of the photo isn't altered or re-compressed.
- Resize / colour-match to the original; keep the full-res original untouched.
- Optionally request 2–4 candidates and let the user pick.

**Summarising the changes** feeds three places: the generation prompt, the history entry ("Couch length −45%, beer volume +30%"), and the export edit log.

Known risk: image models may ignore subtle geometry. Mitigations: the preview-as-reference trick, tighter masks, retry with a stronger model, and a "strength" knob.

---

## 8. Performance plan
- Run **detection/segmentation locally** (ONNX), well under a second on a modern CPU and faster with CoreML / CUDA.
- **Cache** analysis by content hash; never re-analyse the same image.
- **Lazy property suggestions**: only call the VLM for an object when hovered or clicked; prefetch for likely ones in parallel.
- Downscale images (e.g. long side 1024) for all VLM calls; keep the full-res image for the final composite.
- **Client-side preview** means dragging never touches the network.
- Small / fast model for suggester and planner; **stream** reasoning and partial results to the UI via SSE.
- Parallelise independent calls with `tokio::join!`; cancel in-flight jobs when the user changes their mind.
- Track p50/p95 per stage with `tracing` spans to find real bottlenecks.

---

## 9. Repo layout

```
hands-off/
├─ crates/
│  ├─ api/            # axum server, routes, OpenAPI (utoipa)
│  ├─ core/           # domain types, schemars schemas, job manager
│  ├─ vision/         # ort wrappers: detector, SAM, pose, face
│  ├─ llm/            # OpenRouter client, provider trait, prompts
│  └─ imaging/        # decode, resize, mask ops, compositing
├─ web/               # React + Vite + TS, generated API client
├─ models/            # download script + checksums for ONNX weights
├─ prompts/           # versioned prompt templates (suggester/planner/summariser/generator)
├─ eval/              # test images + golden outputs + scoring scripts
├─ docker/            # Dockerfile, compose
└─ docs/
```

---

## 10. Milestones

| # | Milestone | Deliverable | Est. |
|---|---|---|---|
| 0 | **Spike** (do this first) | Hard-code one image; OpenRouter call that takes original + warped preview + instruction and returns an edit. **Validates whether the generation quality is good enough before building anything else** | 2–3 days |
| 1 | Backend skeleton | axum, OpenAPI → TS client, upload, SQLite, config, tracing | 1 wk |
| 2 | Vision | `Detector` trait; RF-DETR via `ort` + SAM masks, masks API, outline overlay in UI; detector speed/quality comparison vs YOLO11 | 1 wk |
| 3 | Suggest + plan | VLM property suggestion and widget planning with structured outputs; schema validation and repair. Optional Decisions API routes added after the standard API works | 1–2 wk |
| 4 | Widgets + preview | Polygon, slider, multi-handle widgets; WebGL warp preview | 1.5–2 wk |
| 5 | Apply pipeline | Batch edits, summariser, generator, composite, SSE progress, accept / cancel / undo | 1.5 wk |
| 6 | Polish + release | History + summaries UI, model picker, Docker, docs, demo GIFs, licence, CI | 1 wk |
| 7 | v0.2 | SAM click-select, VLM-box → SAM, pose + face widgets | 2–3 wk |

Rough total to a usable MVP: **~7–9 weeks** for one developer. Estimates assume part-time uncertainty around model behaviour.

---

## 11. Testing and evaluation
- **Unit**: coordinate transforms, mask ops, schema validation (property-based tests with `proptest`).
- **Contract**: generated client compiles against the OpenAPI spec in CI.
- **LLM output**: golden-image set (bottle, couch, person, burger/beer scenes) with expected property keys and widget types; track schema-valid rate.
- **Generation quality**: small benchmark with human or VLM-judge scoring for (a) edit fidelity, (b) preservation of untouched regions, (c) realism. Re-run when swapping models, which is the main benefit of OpenRouter.
- **Mock provider**: a recorded-response `LlmProvider` so CI and local dev run without spending credits.

---

## 12. Risks and open questions

| Risk | Mitigation |
|---|---|
| Image models don't follow precise geometry | Spike in M0; preview-as-reference; masks; multi-candidate |
| VLM returns bad widget coordinates | Strict schema validation, clamp to mask, auto-retry, fall back to default widget per property type |
| Cost per edit | Show estimated cost; cheap models for planning; cache; BYO key |
| Licence of weights (YOLO = AGPL, RF-DETR XL/2XL = Roboflow platform licence, some SAM variants differ) | Default to Apache-2.0 models (RF-DETR Nano–Large); YOLO11 optional and user-downloaded; weights downloaded, not bundled |
| 3-D head rotation is hard | Defer to v0.3; use FaceMesh + a warp approximation first |
| Safety / misuse (editing people's bodies and faces) | Terms of use, respect provider moderation, no storage of uploads by default |
| Adobe's UX / branding | Build our own name and look; the *idea* (on-canvas controls) is open, but don't copy their UI assets or name |

**Questions for you:**
1. Project name: keep `hands-off`?
2. Licence: MIT/Apache-2.0 (I recommend Apache-2.0)?
3. Self-hosted web app first, or desktop (Tauri) too?
4. Is cloud-only generation fine for MVP, or do you want a local-model path early?

---
