# Extending Hands-Off

Each extension point is a trait or an enum that the compiler checks, so a missing piece is a build error, not a
runtime surprise. Keep the change small, add tests, and update the docs in the same pull request.

## A new widget type

A widget is a control drawn on the canvas for one kind of property (slider, control points, handles, rotate, scale, move). Rotate, scale and move are the most recent worked example: [ADR 0014](adr/0014-controls-that-match-the-change.md).

1. **Core.** Add the variant to `WidgetKind`, to `Widget` (its geometry) and to `EditChange` (what the user did)
   in `crates/core/src/model.rs`. Add checks for the new edit in `check_change` in `crates/api/src/edit.rs`.
2. **Planning.** Handle the new kind where widget plans are parsed (`parse_widget` in `crates/llm/src/contracts.rs`)
   and give it a deterministic fallback in `crates/llm/src/defaults.rs`. Describe it in a new `prompts/planner.vN.md`
   (never edit a shipped version; bump `PLANNER_PROMPT_VERSION`, which also invalidates cached plans). A control that
   is fully determined by the object's box or outline needs no model call: handle it before the model in `plan`
   (`crates/api/src/ai.rs`), as scale, move and control points are.
3. **Telling the model.** Say how the edit is described in words (`describe_edit`) and, if it needs a different
   reference image, how it is drawn (`web/src/lib/guide.ts`) and prompted (`prompts/`).
4. **Contract.** Run `just gen-api`. The TypeScript types now include the new kind.
5. **Frontend.** Add a component in `web/src/features/editor/widgets/` and register it in `registry.ts`. The registry
   is `satisfies { [K in Widget['kind']]: ... }`, so the build fails until every kind has a component.
   Decide how it deforms the preview in `web/src/features/editor/previews.ts`.
6. **Tests.** Core validation, a planner parsing test, a component test with keyboard operation, and a Playwright case.

## A new property pack

Properties are suggested per object by a model, with `default_properties` as the fallback. To make particular
objects better served, adjust the latest `prompts/suggester.vN.md` (give examples for the object classes you care about) and,
for objects that should always offer certain properties, extend `default_properties` in `crates/llm/src/defaults.rs`
keyed on the object's label. Bump the suggester prompt version so cached suggestions are refreshed.

## A new detector

Implement the `Detector` trait (`crates/vision/src/detector.rs`): take an image, return detections (class,
score, box, mask) most confident first. Implementations are blocking and run under `spawn_blocking`.

1. Add the implementation in `crates/vision`, with a golden test in `crates/vision/tests/` that skips when the model is absent.
2. Provide it through a `DetectorProvider` (`crates/api/src/detection.rs`); `LazyRfDetr` is the example, including model download.
3. Mind the licence. Weights are downloaded, never bundled, and anything copyleft must stay optional.
   See [models.md](models.md).

## A new model provider

Text and vision answers go through `LlmProvider`, images through `ImageGenerator` (`crates/llm/src/provider.rs`).
OpenRouter implements both, so any model it offers works without code. To add another service, implement the
traits and pass the implementation to `AppState` (`with_llm`, `with_generator`). Keep the contract the same: return
`Error::Unavailable` when no key or credits are available and `Error::Refused` for content-policy blocks.
The `MockProvider` and `MockGenerator` in `crates/llm/src/mock.rs` show what tests should use.

## A new AI stage or prompt

Prompts live in `prompts/` as versioned Markdown files embedded at build time. Change a prompt by adding a new
version file and updating the constant; never edit a shipped version in place. Describe the expected input and
output in the prompt's file and in `docs/models.md`, and update the snapshot tests.
