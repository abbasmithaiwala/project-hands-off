# 3. Image generation: Gemini 3 Pro Image with a preview reference

Date: 2026-11-14
Status: Accepted (Gate 1 passed)

## Context
The product only works if an image model can turn a rough, warped preview into a photoreal edit. We tested 5 golden images (bottle, couch, person with cup, burger, person with arms raised) and three OpenRouter image-edit models, each with two prompt styles:
- **text + outline**: original image, an orange outline of the target object, and a text instruction.
- **preview reference**: original image, a rough warped preview of the desired geometry, and the instruction.

30 calls in total, about $2.60. Results are in `eval/spike-results/` (`cmp_*.jpg`, `eval.json`).

## Results

| Model | Cost / call | Latency | Framing preserved | Preview reference | Text + outline |
|---|---|---|---|---|---|
| `google/gemini-3-pro-image` | $0.14 | 17-19 s | Yes (aspect drift under 1%) | **Acceptable on 5/5** | Failed on burger and cup; mild on others |
| `google/gemini-3.1-flash-image` | $0.07 | 9-11 s | Yes (0%) | Acceptable on 3/5 (failed couch, burger) | Acceptable on 2/5 |
| `openai/gpt-5-image-mini` | $0.05 | 41-51 s | **No** (aspect drift 14-50%) | Regenerates the whole scene | Regenerates the whole scene |

Objective check, using the detector on each output: with the preview reference, IoU against the target box rose for the cup (0.48 to 0.96 on flash, 0.64 to 0.91 on pro), the man (0.82 to 0.98 on flash) and the burger (0.68 to 0.99 on pro).

## Decision
- The default generator is **`google/gemini-3-pro-image`**, always given the **warped preview as a geometry reference** plus the summarised instruction.
- **`google/gemini-3.1-flash-image`** is offered as a faster, cheaper "draft" option, with a visible note that it is less reliable.
- **`openai/gpt-5-image-mini` is excluded**: it re-renders the entire image and changes its framing, which breaks local edits and compositing.
- Text-only or outline-only prompts are not enough, which confirms the design: the preview carries the geometric intent.
- Mask-feathered compositing stays in the plan: even the best model shifts untouched pixels slightly (mean absolute difference of 3.6-5.4 on pro outside the edit region).
- Model IDs live in config and are re-evaluated with `eval/` when models change.

## Consequences
- Cost per edit is about $0.14 and latency about 18 s with the default model, so progress streaming and cancellation are essential.
- Quality varies by edit type (shape edits on glass were the weakest), so Phase 7 keeps retry-with-another-model and a candidate picker on the roadmap.
- Only three models were tested, and `gpt-5-image` (the full model) was not. Revisit if quality or cost needs improving.
