# AI models

Hosted models are reached through [OpenRouter](https://openrouter.ai), so any model it offers can be used for any stage without code changes.

## Stages

| Stage | Job | Default |
|---|---|---|
| `suggester` | Looks at a crop of one object and proposes the properties a user could edit (length, shape, arm position...) | `google/gemini-3.1-flash-lite` |
| `ui_planner` | Decides where the on-canvas control for a chosen property sits | `google/gemini-3.1-flash-lite` |
| `summariser` | Turns edits into an instruction and a change summary (Phase 7) | `google/gemini-3.1-flash-lite` |
| `generator` | Produces the edited image from the original and the rough preview (Phase 7) | `google/gemini-3-pro-image` |
| `fallback` | Tried when a stage's own model fails | `anthropic/claude-haiku-5.5` |

Rationale and measurements: [ADR 0003](adr/0003-image-generation-strategy.md) (generation) and [ADR 0005](adr/0005-ai-model-defaults.md) (suggestions and planning).

## Getting started without a key

Set `HANDS_OFF_DEMO_AI=1`. Suggestions and widgets then come from built-in rules, so the whole editor can be tried without an account. Generation still needs a real key.

## Providing a key

The server needs an OpenRouter key to call real models. Any one of:

- `OPENROUTER_API_KEY` in the environment, `.env` or `local.env`.
- `PUT /api/v1/settings/openrouter-key` (the web app asks for it the first time it is needed). It is stored on the server and never returned by the API or written to logs.

## Choosing models

`GET /api/v1/settings` shows the current choice; `PUT /api/v1/settings/models` changes it. Any OpenRouter model id is accepted, for example `typesafe/jev-router`, a router that picks a model and reasoning effort per request. It works with these stages but was about 7x slower and 12x costlier than the default in testing, so it is an option, not the default.

A chosen model must accept image input and support structured outputs (JSON Schema). Requests use `provider.require_parameters`, so OpenRouter only routes to providers that honour the schema.

## How answers are checked

Models propose; the server decides. Every answer is parsed against a strict schema, then validated:

- **Suggestions**: keys must be snake_case, labels short, no duplicates; invalid entries are dropped.
- **Widgets**: the kind must match; slider ranges must make sense; control points must spread out, handles must be distinct; all points are mapped from crop to image coordinates and pulled onto the object's mask.

An unusable answer triggers one repair request that quotes the problem. If that also fails, a deterministic default is used (a generic property list, or a widget placed from the object's box and outline), so editing never dead-ends.

## Reliability

- Transient errors (timeouts, `429`, `5xx`) retry once with backoff, then move to the next model.
- A model that cannot serve the request (for example no structured output) is skipped.
- A rejected key or an empty account fails immediately with a clear message and a `503` `unavailable` code.
- Results are cached per image, object and prompt version, so repeating a request costs nothing. Bump the prompt version in `crates/llm/src/contracts.rs` when changing prompts.

## Prompts

Prompts live in [`prompts/`](../prompts) as versioned Markdown files and are embedded at build time. They describe the task and the allowed widget kinds; the response schema is generated from the Rust types in `contracts.rs`.

## Benchmarking a model

```sh
OPENROUTER_API_KEY=... cargo run -p hands-off-llm --example benchmark -- google/gemini-3.1-flash-lite openai/gpt-6-luna
```

This runs the real prompts and validators on the golden images and prints validity, latency and cost per model. It checks that answers are valid, not that placement is good; review placement visually.
