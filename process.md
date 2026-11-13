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

- [ ] `git init`; add `.gitignore` (Rust, Node, models, `.env`, build output), `LICENSE` (Apache-2.0 unless you decide otherwise), `.editorconfig`.
- [ ] Rust workspace: `Cargo.toml` with members `crates/{api,core,vision,llm,imaging}`; `rust-toolchain.toml` pinned; workspace-level lints (`clippy::pedantic`, `unsafe_code = "forbid"` where possible); `rustfmt.toml`; `deny.toml`.
- [ ] Web app: `web/` via Vite + React + TypeScript (strict); ESLint, Prettier, Vitest, Testing Library, Playwright installed; path aliases; pnpm workspace.
- [ ] `justfile` (or `Makefile`) with `fmt`, `lint`, `test`, `gen-api`, `dev`, `check` (the full CI gate).
- [ ] CI (GitHub Actions): format, clippy, nextest, `cargo deny`, web lint/type-check/test, contract-diff check, build. Cache Cargo and pnpm.
- [ ] Pre-commit hook running format and lint on staged files.
- [ ] `.env.example`, `README.md` stub, `docs/` skeleton, `docs/adr/0001-record-decisions.md`.

Exit: `just check` passes on a clean clone; CI is green on the first push.

---

## Phase 1: Spike and risk gates (2-3 days)

Goal: prove the two riskiest assumptions before building product around them. This is throwaway code in `spike/` (not part of the workspace); keep findings, not code.

- [ ] **Generation quality.** Script that sends original + mask + warped preview + instruction to an OpenRouter image-edit model. Use 4-5 golden images (bottle, couch, person, burger/beer scene). Save outputs side by side.
- [ ] **Model comparison.** Run at least 3 candidate image models. Score edit fidelity, preservation of untouched areas, realism. Record cost and latency per call.
- [ ] **Detector export.** Export RF-DETR (Small/Medium, Apache-2.0 sizes) to ONNX; run it from Rust via `ort`. Do the same for YOLO11 and a SAM encoder/decoder.
- [ ] **Detector benchmark.** On your Mac with CoreML and with plain CPU: latency, box quality, and SAM box-prompted mask quality on the golden images.
- [ ] Commit the golden images and the findings to `eval/` and `docs/adr/`.

**GATE 1 (generation):** at least one model follows geometric edits well enough on most golden images. If none do, stop and rethink the generation strategy (stronger reference-image use, tighter masks, different model class) before Phase 2.

**GATE 2 (detector):** RF-DETR exports and runs under `ort` at acceptable speed. If not, switch the default to YOLO11 and record the AGPL implications in an ADR.

Exit: both gates passed, ADRs written, default models chosen.

---

## Phase 2: Backend skeleton and typed contract (about 1 week)

Goal: a running API with a generated, typed client and no model logic yet.

- [ ] `core`: domain types (`ImageId`, `ObjectId`, `NormalizedPoint`, `Object`, `Property`, `Widget`, `Edit`), error enum, config types. All `serde` + `schemars` + `utoipa`.
- [ ] `api`: axum app, graceful shutdown, `tower-http` (trace, CORS, compression, body limits), health route, structured error responses with stable codes.
- [ ] Config loading (env + optional file) with sensible defaults and validation; zero required config except the OpenRouter key.
- [ ] Storage traits (`BlobStore`, `ProjectStore`) with filesystem and SQLite (sqlx, migrations) implementations.
- [ ] `POST /api/v1/images`: validated upload (type, size, dimensions), content hashing, thumbnail generation.
- [ ] OpenAPI generation and the TS client generation step wired into `just gen-api`; CI fails on a diff.
- [ ] Job manager: async jobs, progress events over SSE, cancellation, timeouts.
- [ ] `tracing` setup with request IDs and per-stage spans.

Tests: router integration tests (upload happy and failure paths), job lifecycle and cancellation, config validation, contract diff in CI.

Exit: upload works end to end through the generated client; `docs/api.md` generated.

---

## Phase 3: Vision (about 1 week)

Goal: upload an image, get object outlines.

- [ ] `vision`: `Detector` trait; `RfDetrDetector` via `ort` (pre/post-processing, class mapping, score thresholds, NMS only if the model needs it).
- [ ] `Segmenter` trait; SAM encoder/decoder implementation: box-prompted and point-prompted masks, mask cleanup (largest component, hole fill, simplification to contour polygons).
- [ ] Model manager: first-run download with checksum verification, cache directory, clear progress, execution-provider selection (CoreML/CUDA/CPU) with automatic fallback.
- [ ] `imaging`: decode, EXIF orientation, resize (`fast_image_resize`), mask morphology, mask to PNG/RLE, contour extraction.
- [ ] `POST /api/v1/images/{id}/analyze` streaming objects over SSE as found; results cached by image hash.
- [ ] Click-to-segment endpoint (point prompt) for objects the detector missed.
- [ ] Optional `YoloDetector` behind a Cargo feature and the same trait.

Tests: `proptest` on mask and contour invariants, golden-image detection tests with tolerance, checksum failure handling, fallback of execution providers.

Exit: golden images produce correct outlines in under the target latency; documented in `docs/vision.md`.

---

## Phase 4: LLM gateway, suggestions and widget plans (1.5-2 weeks)

Goal: for a selected object, get valid properties and a widget plan from the standard typed API.

- [ ] `llm`: `LlmProvider` trait and `OpenRouterProvider` (reqwest, streaming, structured outputs, retries with backoff, fallback model list, usage and cost capture, timeouts).
- [ ] `MockProvider` with recorded responses, used by all tests and dev without credits.
- [ ] Stage-to-model config with built-in defaults (`suggester`, `ui_planner`, `summariser`, `generator`).
- [ ] `prompts/`: versioned templates plus schemas for suggester and planner, each documented with inputs and outputs.
- [ ] Suggester: object crop + label in, validated `Property` list out. Lazy, cached by (image hash, object, prompt version).
- [ ] Widget planner: crop + mask + chosen property in, validated widget geometry out. Validation clamps points into the mask; invalid output gets one retry with the error fed back, then a default widget per property type.
- [ ] Endpoints: `.../objects/{oid}/properties`, `.../objects/{oid}/widget`.
- [ ] Explicit API key flow: stored server-side, never logged, never returned; clear error when missing.

Tests: schema validation and repair (table-driven and `proptest`), prompt snapshot tests (`insta`), provider retry and fallback behaviour via mock HTTP server, key redaction in logs.

Eval: run the suggester and planner on the golden set; record schema-valid rate and placement quality in `eval/`.

Exit: schema-valid rate meets the target you set in `eval/` (suggested: 95% or higher after repair); `docs/models.md` written.

---

## Phase 5: Frontend shell and design system (about 1 week)

Goal: a beautiful, minimal, empty-state-to-canvas experience before any editing logic.

- [ ] Design tokens (colour, type, spacing, radius, shadow, motion), dark and light themes, reduced-motion support; a small primitive set (Button, IconButton, Panel, Toast, Skeleton, Tooltip, Dialog) built on accessible headless components.
- [ ] App shell: drop-zone landing (drag, paste, pick), canvas stage, floating action bar, collapsible output panel, minimal top bar.
- [ ] Inline API-key prompt shown only when first needed; Advanced settings drawer (model per stage) hidden by default.
- [ ] PixiJS canvas: pan/zoom, fit-to-screen, hi-DPI, object outline overlay with hover and select, keyboard navigation between objects.
- [ ] Generated API client + TanStack Query hooks; SSE hook for analysis and jobs; global error boundary with friendly messages.
- [ ] Zustand store for editor state (selection, active property, pending edits).

Tests: component and hook tests (Vitest), accessibility scan (axe) on main screens, Playwright smoke test (upload, see outlines) against the mock provider, screenshot baselines for key states.

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

- _(add dated entries here as phases start, finish, and gates are decided)_
