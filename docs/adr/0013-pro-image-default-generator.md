# 13. Pro image model as the default generator

Date: 2026-11-15
Status: Accepted

## Context
[ADR 0009](0009-outline-reference-for-reshaping.md) made `google/gemini-3.1-flash-image` the default generator and draft model for cost and speed. In hand-run trials it was acceptable, while `google/gemini-3-pro-image` gave the best results ([ADR 0003](0003-image-generation-strategy.md): acceptable on 5/5 golden edits, about $0.14 and 18 s per edit). Result quality is the product, so reliability matters more than saving cents.

## Decision
- The default `generator` and `draft_generator` are both **`google/gemini-3-pro-image`**. "Faster draft" stays hidden while the two are equal.
- Cheaper models (`google/gemini-3.1-flash-image`, `google/gemini-3.1-flash-lite-image`) remain one setting away.

## Consequences
- Edits cost about twice as much and take about twice as long as with the flash model.
- Settings saved earlier keep their stored models; only new installs and reset settings pick up the new default.
