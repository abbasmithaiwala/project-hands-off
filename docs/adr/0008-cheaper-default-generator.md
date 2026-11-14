# 8. Cheaper default image generator

Date: 2026-11-14
Status: Accepted (provisional, pending benchmark)

## Context
[ADR 0003](0003-image-generation-strategy.md) chose `google/gemini-3-pro-image` as the default generator: acceptable on 5/5 golden edits, but about $0.14 and 17-19 s per edit. Image output tokens dominate the cost ($120 per million). Later models in the same family are much cheaper:

| Model | Image output price | Estimated cost per edit |
|---|---|---|
| `google/gemini-3-pro-image` | $120/M | $0.14 (measured) |
| `google/gemini-3.1-flash-image` | $60/M | $0.07 (measured) |
| `google/gemini-3.1-flash-lite-image` | $30/M | about $0.035-0.04 (estimated) |

## Decision
- The default generator is **`google/gemini-3.1-flash-lite-image`**, still given the warped preview as a geometry reference.
- `google/gemini-3-pro-image` remains available in Settings as the proven, most reliable option.
- The "Faster draft" model stays `google/gemini-3.1-flash-image`.

## Consequences
- Cost per edit drops to roughly a quarter, and generation should be faster.
- **Quality is unmeasured.** `gemini-3.1-flash-image` failed the couch and burger edits in ADR 0003, and a lite model may be no better. Run the five-image preview-reference benchmark from ADR 0003 on this model (and `google/gemini-nano-banana-2.1`) and revisit this decision with the results. If it fails more than one image, switch the default back.
- Settings saved earlier on a server keep their stored generator; only new installs and reset settings pick up the new default.
