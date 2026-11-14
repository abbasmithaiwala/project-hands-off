# process.md: Build Hands-Off end to end

The step-by-step execution runbook for [plan.md](plan.md). Standards in [CLAUDE.md](CLAUDE.md) apply to every step; they are not repeated here.

## How to use this file

- Work **top to bottom, one phase at a time**. Do not start a phase until the previous phase's **exit criteria** are met.
- Each task is a checkbox. Tick it only when its **definition of done** is true: code, tests, and docs are in the same change, and the CI gate passes.
- Each task is one small commit (Conventional Commits). A phase ends with a tag (`phase-N-done`).
- **Gates** (marked `GATE`) are go/no-go decisions. If a gate fails, stop, record the outcome in an ADR under `docs/adr/`, and update `plan.md` before continuing.
- Estimates assume one developer; they match plan.md §10.

### Definition of done (applies to every task)
1. Format, lint, type-check and tests pass locally and in CI.
2. New behaviour has tests, including failure paths.
3. Public APIs have rustdoc/TSDoc; affected docs in `docs/` or the README are updated.
4. No duplicated logic, no comment noise, no `unwrap` outside tests.
5. OpenAPI spec and generated TS client are regenerated if the API changed.

### Progress log
Keep a short dated list at the bottom of this file: phase started, phase done, gate outcomes, and decisions.

---

## Phase 0: Foundations (about 1 day)

Goal: an empty but fully wired monorepo where CI is green.

- [x] `git init`; add `.gitignore` (Rust, Node, models, `.env`, build output), `LICENSE` (Apache-2.0 unless you decide otherwise), `.editorconfig`.
- [x] Rust workspace: `Cargo.toml` with members `crates/{api,core,vision,llm,imaging}`; `rust-toolchain.toml` pinned; workspace-level lints (`clippy::pedantic`, `unsafe_code = "forbid"` where possible); `rustfmt.toml`; `deny.toml`.
- [x] Web app: `web/` via Vite + React + TypeScript (strict); ESLint, Prettier, Vitest, Testing Library, Playwright installed; path aliases; npm (see ADR 0002).
- [x] `justfile` (or `Makefile`) with `fmt`, `lint`, `test`, `gen-api`, `dev`, `check` (the full CI gate).
- [x] CI (GitHub Actions): format, clippy, nextest, `cargo deny`, web lint/type-check/test, contract-diff check, build. Cache Cargo and npm.
- [x] Pre-commit hook running format and lint on staged files.
- [x] `.env.example`, `README.md` stub, `docs/` skeleton, `docs/adr/0001-record-decisions.md`.

Exit: `just check` passes on a clean clone; CI is green on the first push.

---

## Phase 1: Spike and risk gates (2-3 days)

Goal: prove the two riskiest assumptions before building product around them. This is throwaway code in `spike/` (not part of the workspace); keep findings, not code.

- [x] **Generation quality.** Script that sends original + mask + warped preview + instruction to an OpenRouter image-edit model. Use 4-5 golden images (bottle, couch, person, burger/beer scene). Save outputs side by side.
- [x] **Model comparison.** Run at least 3 candidate image models. Score edit fidelity, preservation of untouched areas, realism. Record cost and latency per call.
- [x] **Detector export.** Export RF-DETR (Small/Medium, Apache-2.0 sizes) to ONNX; run it from Rust via `ort`. Do the same for YOLO11 and a SAM encoder/decoder.
- [x] **Detector benchmark.** On your Mac with CoreML and with plain CPU: latency, box quality, and SAM box-prompted mask quality on the golden images.
- [x] Commit the golden images and the findings to `eval/` and `docs/adr/`.

**GATE 1 (generation): PASSED, see ADR 0003.** at least one model follows geometric edits well enough on most golden images. If none do, stop and rethink the generation strategy (stronger reference-image use, tighter masks, different model class) before Phase 2.

**GATE 2 (detector): PASSED, see ADR 0004.** RF-DETR exports and runs under `ort` at acceptable speed. If not, switch the default to YOLO11 and record the AGPL implications in an ADR.

Exit: both gates passed, ADRs written, default models chosen.

---

## Phase 2: Backend skeleton and typed contract (about 1 week)

Goal: a running API with a generated, typed client and no model logic yet.

- [x] `core`: domain types (`ImageId`, `ObjectId`, `NormalizedPoint`, `Object`, `Property`, `Widget`, `Edit`), error enum, config types. All `serde` + `schemars` + `utoipa`.
- [x] `api`: axum app, graceful shutdown, `tower-http` (trace, CORS, compression, body limits), health route, structured error responses with stable codes.
- [x] Config loading (env + optional file) with sensible defaults and validation; zero required config except the OpenRouter key.
- [x] Storage traits (`BlobStore`, `ProjectStore`) with filesystem and SQLite (sqlx, migrations) implementations.
- [x] `POST /api/v1/images`: validated upload (type, size, dimensions), content hashing, thumbnail generation.
- [x] OpenAPI generation and the TS client generation step wired into `just gen-api`; CI fails on a diff.
- [x] Job manager: async jobs, progress events over SSE, cancellation, timeouts.
- [x] `tracing` setup with request IDs and per-stage spans.

Tests: router integration tests (upload happy and failure paths), job lifecycle and cancellation, config validation, contract diff in CI.

Exit: upload works end to end through the generated client; `docs/api.md` generated.

---

## Phase 3: Vision (about 1 week)

Goal: upload an image, get object outlines.

- [x] `vision`: `Detector` trait; `RfDetrSeg` via `ort` (pre/post-processing, class mapping, score thresholds, NMS only if the model needs it).
- [ ] (Deferred, see ADR 0004) `Segmenter` trait; SAM encoder/decoder implementation: box-prompted and point-prompted masks, mask cleanup (largest component, hole fill, simplification to contour polygons).
- [x] Model manager: first-run download with checksum verification, cache directory, clear progress, execution-provider selection (CoreML/CUDA/CPU) with automatic fallback.
- [x] `imaging`: decode, EXIF orientation, resize (`fast_image_resize`), mask morphology, mask to PNG/RLE, contour extraction.
- [x] `POST /api/v1/images/{id}/analyze` streaming objects over SSE as found; results cached by image hash.
- [ ] (Deferred with SAM) Click-to-segment endpoint (point prompt) for objects the detector missed.
- [ ] (Deferred) Optional `YoloDetector` behind a Cargo feature and the same trait.

Tests: `proptest` on mask and contour invariants, golden-image detection tests with tolerance, checksum failure handling, fallback of execution providers.

Exit: golden images produce correct outlines in under the target latency; documented in `docs/vision.md`.

---

## Phase 4: LLM gateway, suggestions and widget plans (1.5-2 weeks)

Goal: for a selected object, get valid properties and a widget plan from the standard typed API.

- [x] `llm`: `LlmProvider` trait and `OpenRouterProvider` (reqwest, streaming, structured outputs, retries with backoff, fallback model list, usage and cost capture, timeouts).
- [x] `MockProvider` with recorded responses, used by all tests and dev without credits.
- [x] Stage-to-model config with built-in defaults (`suggester`, `ui_planner`, `summariser`, `generator`).
- [x] `prompts/`: versioned templates plus schemas for suggester and planner, each documented with inputs and outputs.
- [x] Suggester: object crop + label in, validated `Property` list out. Lazy, cached by (image hash, object, prompt version).
- [x] Widget planner: crop + mask + chosen property in, validated widget geometry out. Validation clamps points into the mask; invalid output gets one retry with the error fed back, then a default widget per property type.
- [x] Endpoints: `.../objects/{oid}/properties`, `.../objects/{oid}/widget`.
- [x] Explicit API key flow: stored server-side, never logged, never returned; clear error when missing.

Tests: schema validation and repair (table-driven and `proptest`), prompt snapshot tests (`insta`), provider retry and fallback behaviour via mock HTTP server, key redaction in logs.

Eval: run the suggester and planner on the golden set; record schema-valid rate and placement quality in `eval/`.

Exit: schema-valid rate meets the target you set in `eval/` (suggested: 95% or higher after repair); `docs/models.md` written.

---

## Phase 5: Frontend shell and design system (about 1 week)

Goal: a beautiful, minimal, empty-state-to-canvas experience before any editing logic.

- [x] Design tokens (colour, type, spacing, radius, shadow, motion), dark and light themes, reduced-motion support; a small primitive set (Button, IconButton, Panel, Toast, Skeleton, Tooltip, Dialog) built on accessible headless components.
- [x] App shell: drop-zone landing (drag, paste, pick), canvas stage, floating action bar, collapsible output panel, minimal top bar.
- [x] Inline API-key prompt shown only when first needed; Advanced settings drawer (model per stage) hidden by default.
- [x] PixiJS canvas: pan/zoom, fit-to-screen, hi-DPI, object outline overlay with hover and select, keyboard navigation between objects.
- [x] Generated API client + TanStack Query hooks; SSE hook for analysis and jobs; global error boundary with friendly messages.
- [x] Zustand store for editor state (selection, active property, pending edits).

Tests: component and hook tests (Vitest), accessibility scan (axe) on main screens, Playwright tests against a mocked API, screenshot baselines for key states (local, macOS).

Exit: upload to selectable outlines feels instant and polished; Lighthouse accessibility check meets your target.

---

## Phase 6: Widgets and live preview (1.5-2 weeks)

Goal: manipulate controls and see an instant preview.

- [ ] Widget renderer registry with exhaustive typing over the widget union; one component per widget type.
- [ ] **Slider/stretch widget** with region-stretch preview.
- [ ] **Control-point polygon widget** with handle dragging, snapping, and constraint to a sensible range.
- [ ] **Multi-handle (puppet-warp-lite) widget** using Moving Least Squares or triangulated mesh warp.
- [ ] WebGL warp pipeline on the masked crop (Web Worker/OffscreenCanvas where it helps); 60fps target; preview clearly labelled as approximate.
- [ ] Edit model: each drag produces a normalised, typed `Edit` (origin, target, scalar); pending edits list with per-edit reset; Apply and Cancel bar.
- [ ] Keyboard control for handles (arrow-key nudging, modifier for fine steps) and screen-reader labels.

Tests: pure unit tests for warp math and coordinate transforms (`proptest`-style invariants in TS where useful), component tests for each widget, Playwright test dragging a handle and asserting the pending edit payload.

Exit: dragging any widget is smooth and the emitted edit payload matches the contract.

---

## Phase 7: Apply pipeline (about 1.5 weeks)

Goal: apply one or many edits and get a photoreal result.

- [ ] `ImageGenerator` trait with the OpenRouter implementation (image output modality), plus a fake generator for tests.
- [ ] Summariser: deltas to change summary and generation prompt (validated output); template fallback if the model fails.
- [ ] `POST /api/v1/images/{id}/edits` batching multiple edits into one generation request; builds the input set (original, mask overlay, warped preview, instruction).
- [ ] Post-processing: mask-feathered composite into the original, size and colour matching, preserve full-resolution original; store the result as a new version.
- [ ] SSE job events: `analysing`, `planning`, `generating`, `compositing`, `done`, with the reasoning log for the output panel; cancellation propagates to the in-flight request.
- [ ] Frontend: progress UI, result reveal with before/after comparison, Accept, Cancel and Retry (optionally with another model).
- [ ] Cost and latency shown per edit in the output panel.

Tests: composite correctness (untouched pixels identical), batch request construction (snapshot), job cancellation, error paths (provider failure, content moderation refusal, timeout) with clear user messages, Playwright full-flow test against the fake generator.

Exit: the video's demo scenarios (bottle shape, couch length, multi-edit) work end to end with a real model.

---

## Phase 8: History, polish and onboarding (about 1 week)

- [ ] Version history with undo/redo, compact timeline, and auto-generated change summaries per version.
- [ ] Project persistence (SQLite + blobs): reopen, rename, delete; export image plus JSON edit log.
- [ ] First-run experience: a sample image, a short unobtrusive hint sequence, empty states, and helpful error recovery.
- [ ] Performance pass: profile with `tracing` and browser tools; fix the worst bottlenecks; check bundle size and time-to-interactive budgets.
- [ ] Accessibility and visual QA pass across themes and viewport sizes; fix findings.
- [ ] Security pass: upload limits, CORS, header hardening, secret handling review, `cargo deny` and `npm audit` clean.

Exit: the full gate is green; a new user can go from landing to a finished edit without reading docs.

---

## Phase 9: Decisions API (optional, about 1 week)

Goal: add the server-driven alternative without duplicating logic. Skip or defer if the standard API meets your needs.

- [ ] Cargo feature `decisions-api`; `/api/decisions/...` routes calling the same services as the standard API.
- [ ] Decision types (`DetectDecision`, `PropertyDecision`, `WidgetDecision`, `EditDecision`, `AcceptDecision`) generated into the TS client; validation and repair shared with the standard path.
- [ ] Decision log and replay (image hash, model, prompt version) feeding the output panel and evals.
- [ ] Frontend flow switch (config flag and per-session toggle) rendering decisions through the same widget registry.
- [ ] Eval comparison of both styles on the golden set; record the result in an ADR.

Tests: parity tests asserting both styles yield the same edit for the same inputs; feature-off build compiles and passes tests.

Exit: toggling styles changes only the route set, with no duplicated business logic.

---

## Phase 10: Packaging and release (about 1 week)

- [ ] Multi-stage Dockerfile (small final image, non-root, model download at first run) and `docker compose` for one-command start.
- [ ] Release automation: versioning, changelog, binaries for macOS/Linux, container image published to a registry, signed checksums.
- [ ] `README.md` with demo GIF, quick start, configuration reference; `docs/` complete: architecture, API, models and licences, adding a widget/property/detector/provider, deployment, contributing; `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, issue and PR templates.
- [ ] Third-party licence audit and `THIRD_PARTY_NOTICES`; confirm weights are downloaded, not bundled.
- [ ] Clean-machine test: follow the README from scratch on a fresh environment and fix every stumble.
- [ ] Tag `v0.1.0`.

Exit: a stranger can run it with one command and make their first edit in under five minutes.

---

## Phase 11: v0.2 and beyond (see plan.md §2)

Pick up in this order, each as its own mini-phase with the same task template (implement, test, document, eval):
1. Pose and face-landmark widgets (arms, expression).
2. VLM boxes to SAM for open-vocabulary objects (cheese, foam), if not already covered.
3. Multiple candidate results with a picker.
4. 3-D head orientation.
5. Plugin API for widget types and property packs; optional local generation backends.

---

## Risk checkpoints

Review these at the end of every phase.

| Check | Question |
|---|---|
| Quality | Are edits still faithful and realistic on the golden set? |
| Latency | p50/p95 per stage within budget? Any regression? |
| Cost | Average cost per edit acceptable with default models? |
| Simplicity | Did any step add configuration the user must now set? If so, give it a default or remove it. |
| Duplication | Is any logic implemented twice across routes, crates or components? |
| Docs | Does every new public API and flow have up-to-date documentation? |

---

## Progress log

- 2026-11-14: Phase 0 done. `just check` green. Switched frontend to npm and oxlint (ADR 0002): pnpm shim broken locally. `gen-api` is a stub until Phase 2.
- 2026-11-14: Phase 1 done. Gate 1 passed: `google/gemini-3-pro-image` with a preview reference gave acceptable edits on 5/5 golden images ($0.14, ~18 s); text-only prompts and `gpt-5-image-mini` are not viable (ADR 0003). Gate 2 passed: RF-DETR-Seg Medium exports to ONNX and runs at ~0.35 s on CPU; SAM dropped from the MVP critical path (ADR 0004). Rust `ort` confirmation moves to Phase 3.
- 2026-11-14: Phase 2 done. Typed API with upload, images, jobs (SSE, cancel, timeout), SQLite + filesystem storage, OpenAPI to TypeScript client via `just gen-api`. 38 Rust API tests, 30 core, 8 imaging, 4 web. Smoke-tested the real server with curl. TypeScript pinned to 5.x (ADR 0002). `RUSTSEC-2026-0459` (SQLite WAL flaw in the `libsqlite3-sys` that `sqlx` 0.9.0 pins) is ignored in `deny.toml` with a documented reason; remove once `sqlx` allows `libsqlite3-sys` 0.39 or newer.
- 2026-11-14: Phase 3 done (with SAM click-to-segment and YOLO11 deferred). Rust `ort` + RF-DETR-Seg reproduces the Python reference (IoU >= 0.98), ~260-290 ms per image in release. Model manager with checksums and download, EXIF orientation, mask cleanup and outlines, `/analyze` job streaming `object_found` events, cached results. Release builds need LTO off (see ADR 0004). Golden tests skip when the model is not installed.
- 2026-11-14: Phase 4 done. `llm` crate (OpenRouter client with retry, fallback and cost capture; mock and demo providers; strict JSON schemas), versioned prompts, suggester and widget planner with validation, one repair retry and deterministic defaults, settings API (write-only key, model choice), `HANDS_OFF_DEMO_AI` mode. Benchmarked six models including `typesafe/jev-router` (ADR 0005). Fixed a real schema bug and a real placement-quality problem found by live testing. Note: `jev` turned out to be TypeSafe's Jev Router model on OpenRouter, not the typed-decisions API assumed earlier; it is supported as an optional model choice and the Decisions API phase below needs revisiting.
- 2026-11-14: Phase 5 done. Design tokens and primitives, landing (drop, paste, pick), lazy-loaded editor with a PixiJS canvas (pan, zoom, hover, select, keyboard), property menu fed by the AI suggestions, planned widgets drawn on the canvas, activity panel, inline API-key dialog, settings dialog, theme store. 115 unit and component tests, axe clean on four views, 10 Playwright tests, 4 visual baselines. Real-browser review found and fixed: stale hit-test polygons when the image loaded after the objects, theme not applied on the landing page after reload, duplicate accessible name on the file input. Not done: Lighthouse score (no Lighthouse run yet); visual baselines are macOS-only.
