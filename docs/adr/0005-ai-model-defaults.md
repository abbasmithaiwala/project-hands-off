# 5. AI model defaults for suggestions and widget planning

Date: 2026-11-14
Status: Accepted

## Context
Two stages call hosted models through OpenRouter: the **suggester** (which properties can this object be edited by?) and the **widget planner** (where does the control sit?). Both need image input and strict structured output, and both sit on the interactive path, so latency and cost matter. `typesafe/jev-router` (a router that picks a model and reasoning effort per request) was also evaluated as an optional choice.

## Benchmark
`crates/llm/examples/benchmark.rs` runs the real prompts, schemas and validators on five golden-image crops (10 calls per model: a suggester and a planner call per image). "Valid" means the answer parsed and passed the validators in `contracts.rs`.

| Model | Valid | Avg latency | Cost (10 calls) |
|---|---|---|---|
| `google/gemini-3.1-flash-lite` | 10/10 | 1.6 s | $0.0072 |
| `google/gemini-2.5-flash-lite` | 10/10 | 2.0 s | $0.0010 |
| `anthropic/claude-haiku-5.5` | 10/10 | 3.9 s | $0.0036 |
| `openai/gpt-6-luna` | 10/10 | 5.1 s | $0.0022 |
| `typesafe/jev-router` | 10/10 | 12.0 s | $0.0865 |
| `openai/gpt-5-nano` | 9/10 | 16.9 s | $0.0096 |

Schema validity is not placement quality. The benchmark does not measure whether widgets sit on the right part of the object; that needs human review of the results.

## Placement quality (visual check)
Validity alone hid a real problem. In a live run on the couch photo, `gemini-3.1-flash-lite` produced schema-valid widgets that were badly placed: shape points bunched in one corner and cushion handles on the sofa legs. Two changes fixed it:

- The planner prompt now includes the object's bounding box in crop coordinates, so the model knows where to look.
- Validation rejects control points that do not spread across the crop and handles that sit on top of each other, which triggers the repair request and then the deterministic default.

After these, the default model placed all three widget kinds sensibly (slider across the couch length, shape points around its outline, handles on the two cushions) in 1.6-3.6 s. `gemini-3.5-flash`, `claude-haiku-5.5` and `gemini-3.1-pro-preview` produced similar placements but took 3.5-56 s per widget, with no visible gain. Only one image and one object type were checked; the golden-set review in `eval/` should grow before changing defaults.

## Decision
- Defaults: **`google/gemini-3.1-flash-lite`** for suggester, planner and summariser; **`google/gemini-3-pro-image`** for generation (ADR 0003).
- Fallback: **`anthropic/claude-haiku-5.5`**, a different vendor, so one provider's outage does not stop editing.
- **`typesafe/jev-router` is supported but not the default.** It is about 7x slower and 12x costlier than the default here, because it spends reasoning effort the task does not need. Users can choose it per stage in Settings; any OpenRouter model id is accepted.
- Every answer is validated and repaired, then falls back to deterministic defaults (`defaults.rs`), so a weak model degrades quality without breaking the flow.

## Consequences
- A suggestion or widget plan costs roughly a tenth of a cent and takes 1-2 s with the defaults. Results are cached per image, object and prompt version.
- Model ids change quickly. Re-run the benchmark before changing defaults.
- A bug found while benchmarking: a schema field named `properties` was misread as a schema keyword, producing invalid schemas that every provider rejected. It is covered by a regression test in `schema.rs`.
