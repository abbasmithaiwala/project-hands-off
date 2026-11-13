# CLAUDE.md

Project guide for Hands-Off, an open-source, self-hostable image editor where you manipulate on-canvas controls (drag a handle, slide a length, move pose points) instead of writing text prompts. It is modelled on Adobe's *Project Hands On*.

`plan.md` is the source of truth for scope, architecture, technology choices and milestones. Read it before starting work. If a change contradicts the plan, update the plan in the same change.

## Stack (see plan.md for rationale)

- **Backend:** Rust, axum + tokio, `ort` (ONNX Runtime) for local vision, OpenRouter for all hosted models, SQLite via sqlx.
- **API contract:** `utoipa` generates OpenAPI from Rust types, which generates the TypeScript client. The standard typed REST API is the default; the Decisions API is optional behind the `decisions-api` Cargo feature.
- **Detection:** RF-DETR (Apache-2.0 sizes) for boxes, SAM for masks, behind a `Detector` trait. YOLO11 is optional and never bundled (AGPL).
- **Frontend:** React + Vite + TypeScript (strict), PixiJS canvas, TanStack Query, Zustand.

## Product and UX principles

The experience should feel like a polished Adobe tool: calm, fast, and obvious without instructions.

- **Zero configuration to start.** Upload an image and edit. Sensible defaults for every model, stage and setting. The only unavoidable input is an OpenRouter API key, requested once, inline, at the moment it is first needed. Everything else lives in an Advanced panel that most users never open.
- **Minimal, canvas-first interface.** The image is the hero. Dark, quiet chrome, one accent colour (the orange selection outline and primary button), generous spacing, no permanent toolbars. Controls appear only in context: hover an object to see its outline, click to see its properties, pick one to see its widget.
- **Direct manipulation.** Dragging a widget gives an instant client-side preview at 60fps with no network call. Generation only happens on Apply.
- **Always show progress, never block.** Stream stage-by-stage progress (analysing, planning, generating). Every long operation is cancellable. Provide a collapsible reasoning/output panel for transparency.
- **Apply / Cancel pattern.** Floating, centred Apply and Cancel actions under the canvas while an edit is pending, then Accept / Cancel on the result. Undo/redo and a compact history with auto-generated change summaries.
- **Honest states.** Skeletons rather than spinners where layout is known. Clear, human error messages with a next step (retry, switch model), never raw errors or stack traces.
- **Visual quality bar.** Consistent 4/8px spacing scale, a single type scale, design tokens for colour, radius, shadow and motion, subtle purposeful animation (150-250ms, respect `prefers-reduced-motion`), light and dark themes, responsive layout down to tablet.
- **Accessible by default.** Full keyboard operation (select object, cycle properties, nudge handles, apply/cancel), visible focus, ARIA labels, WCAG AA contrast, and screen-reader announcements for progress.
- **Fast.** Target time-to-interactive under 2s on a mid-range laptop; no layout shift; heavy work off the main thread (Web Workers/OffscreenCanvas for warping where useful).

## Code quality standards

All code must be clean, typesafe, modular, performant, scalable, deduplicated, efficient and robust.

- **Single source of truth for types.** Define data contracts once in Rust (`serde` + `schemars` + `utoipa`) and generate the TypeScript types and client. Never hand-write a type that exists on the other side. Regenerate the client in the same change that alters the API.
- **No duplication.** Before adding code, search for existing helpers. Extract shared logic into the appropriate crate or package module. Three similar blocks means it is time to abstract; one block does not.
- **Small, single-purpose units.** Functions do one thing, files have one responsibility, modules expose a small public surface. Depend on traits and interfaces (`Detector`, `LlmProvider`, `ImageGenerator`), not concrete implementations, so models and providers stay swappable.
- **Make invalid states unrepresentable.** Use enums and newtypes (`ImageId`, `ObjectId`, `NormalizedPoint`) over primitives and strings. In TypeScript use discriminated unions with exhaustive `switch` and a `never` check.
- **Robust error handling.** Rust: `thiserror` for library crates, `anyhow` only at the binary edge, no `unwrap`/`expect`/`panic!` outside tests and provably-safe initialisation. Map errors to stable API error codes. Validate all external input (uploads, model output) at the boundary and clamp or repair model output rather than trusting it.
- **Performance.** Avoid needless allocations and clones, stream large payloads, cache analysis by image hash, run independent calls concurrently (`tokio::join!`), downscale images for model calls, and never block the async runtime with CPU work (`spawn_blocking`). Measure before optimising; add `tracing` spans on every pipeline stage.
- **Scalability.** Stateless request handling, job state in the store not in memory, storage behind a trait (filesystem now, S3 later), bounded queues and timeouts everywhere.
- **Security.** API keys stay server-side, never logged or sent to the browser. Enforce upload size and type limits, sanitise filenames, and set strict CORS.
- **Dependencies.** Add one only when it clearly earns its place. Prefer well-maintained crates and packages, check licences (`cargo deny`), and keep model weights downloaded rather than bundled.

### Rust conventions
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` (with `clippy::pedantic` enabled at the workspace level, allowing only justified exceptions) must pass.
- Workspace layout in plan.md §9: `api`, `core`, `vision`, `llm`, `imaging`. Dependencies point inward: `api` depends on the others, never the reverse; `core` depends on nothing project-internal.
- Public items are documented with `///` rustdoc. Prefer `&str`/slices in arguments, return owned types, use `#[must_use]` where ignoring a result is a bug.

### TypeScript / React conventions
- `strict: true`, no `any`, no non-null assertions without justification, no `@ts-ignore`. ESLint and Prettier must pass with zero warnings.
- Feature-based folders. Components are small and presentational; data fetching lives in query hooks; canvas and warp logic live in plain TypeScript modules that are testable without React.
- Server state with TanStack Query only; local UI state with Zustand; never mirror server data into local state.
- Co-locate styles, tests and component files. Use design tokens, never hard-coded colours or spacing.

## Naming and structure

- **Rust:** `snake_case` modules, functions and files; `UpperCamelCase` types and traits; `SCREAMING_SNAKE_CASE` constants. Crates are short nouns (`vision`, `llm`). Traits name capabilities (`Detector`), implementations name the technology (`RfDetrDetector`, `OpenRouterProvider`).
- **TypeScript:** `PascalCase` components and types, `camelCase` functions and variables, `kebab-case` file and folder names (components may use `PascalCase.tsx`; pick one per package and apply it consistently).
- **Booleans** read as questions (`isPending`, `hasMask`). **Functions** are verbs, **types** are nouns. No abbreviations beyond universal ones (`id`, `url`, `api`).
- **Files** are named for their single primary export. No `utils.ts` or `helpers.rs` catch-alls; name the module for what it does (`mask_ops.rs`, `warp.ts`).
- Follow the repo layout in plan.md §9. Do not create new top-level folders without updating the plan.
- Commits use Conventional Commits (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`), small and focused.

## Comments and documentation

- **No comment noise.** Do not narrate what the code does, restate names, leave commented-out code, or add TODOs without an issue link. Good names and small functions replace comments.
- Comment only the **why**: a non-obvious constraint, a workaround with a link, a numerical or algorithmic subtlety, a deliberate trade-off.
- **Rustdoc / TSDoc on every public API**: purpose, inputs, errors, and an example where it helps. Internal code needs none unless it is subtle.
- **Everything necessary is documented:**
  - `README.md`: what it is, a demo GIF, quick start (one command via Docker), configuration.
  - `docs/`: architecture, API usage (generated OpenAPI reference), model configuration and licences, how to add a widget type, a property pack, or a detector/provider, deployment, and contributing.
  - `prompts/` templates are versioned and documented with their expected input and output schemas.
  - An ADR (short decision record) in `docs/adr/` for each significant technical decision, for example choosing RF-DETR over YOLO11.
- Keep docs in the same change as the code they describe. Stale docs are bugs.

## Testing

Tests are required throughout, not added at the end. A change without tests is incomplete.

- **Rust unit tests** next to the code (`#[cfg(test)]`) for coordinate transforms, mask operations, schema validation and repair, strategy and provider logic. Use `proptest` for numeric and geometric invariants and `insta` for snapshot tests of serialised decisions and prompts. Run with `cargo nextest`.
- **Rust integration tests** in each crate's `tests/` folder: API routes through the real router with a **mock `LlmProvider`** and a fake generator, so tests never spend credits or need network access.
- **Contract tests:** CI regenerates the OpenAPI spec and TS client and fails on any diff or type error.
- **Frontend unit tests** with Vitest and Testing Library for components, hooks, and the pure warp/geometry modules. **End-to-end tests** with Playwright for the core flow (upload, select, drag, apply, accept, undo) against the mock provider.
- **Model evals** in `eval/`: a golden image set measuring schema-valid rate, widget placement quality and edit fidelity; re-run when swapping models or prompts. These are separate from the fast test suite.
- **Visual and accessibility checks:** Playwright screenshots for key states, and an automated accessibility scan (axe) on main screens.
- Tests are deterministic, independent and fast. No sleeping, no real network, no shared mutable state. Name tests for behaviour (`rejects_oversized_upload`), and cover the failure paths as thoroughly as the happy paths.
- **CI gate** (all must pass): format, lint, type-check, unit and integration tests, contract check, build, `cargo deny`, and `npm audit`.

## Working in this repo

- Plan first for non-trivial work: confirm the approach against `plan.md`, then implement in small, reviewable steps, each with tests and docs.
- Before reporting work as done: run format, lint, type-check and tests, and state plainly what passed and what did not.
- When a requirement is ambiguous, ask once with a recommendation rather than guessing; record the decision in `plan.md` or an ADR.
- Do not commit secrets, model weights, or generated build output. Keep `.env.example` current.
