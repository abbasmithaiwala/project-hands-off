# 11. DeepSeek V4.1 Flash for the suggester, planner and summariser

Date: 2026-11-14
Status: Accepted (provisional, pending the model evals)

## Context
The three text-and-vision stages (suggesting properties, placing controls, writing the summary) defaulted to `google/gemini-3.1-flash-lite`. They need image input (the suggester and planner are shown a crop of the photo and a mask) and strict JSON output.

OpenRouter lists `deepseek/deepseek-v4.1-flash` as accepting text and images, with structured outputs, at about $0.05 per million input and $0.20 per million output tokens, against $0.25 and $1.50 for Gemini 3.1 Flash Lite.

## Decision
The default `suggester`, `ui_planner` and `summariser` model is `deepseek/deepseek-v4.1-flash`. The fallback stays `anthropic/claude-haiku-5.5`. Gemini 3.1 Flash Lite remains selectable in Settings. Settings already saved on a server keep their stored values.

## Consequences
- Roughly a fifth of the cost per call for these stages.
- Unmeasured: how well it reads a cropped photo and mask, and how often its JSON validates. The repair-then-default path in `ai.rs` covers bad answers, but run the model benchmark (`cargo run -p hands-off-llm --example benchmark`) and the golden images before relying on it, and switch back if suggestions or control placement get worse.
