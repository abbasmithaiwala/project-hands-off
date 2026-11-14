# HTTP API

The API is described by an OpenAPI document generated from the Rust types, so the spec, the server and the TypeScript client cannot drift apart.

- Spec: [`web/openapi.json`](../web/openapi.json)
- Generated types: `web/src/api/schema.d.ts`
- Typed client: `web/src/api/client.ts` (built on `openapi-fetch`)

Regenerate after changing any route or domain type:

```sh
just gen-api
```

CI runs the same command and fails if the committed files differ.

## Conventions

- All routes live under `/api/v1`.
- Coordinates are normalised to `0.0..=1.0` with the origin at the top-left, so previews and full-resolution images never disagree.
- Every response carries an `x-request-id` header that also appears in the server logs.
- Errors are JSON `{ "code": string, "message": string }`. Codes are stable; messages are for humans.

| Code | HTTP status | Meaning |
|---|---|---|
| `validation` | 400 | Malformed or invalid request |
| `unsupported_media` | 415 | Not a JPEG, PNG or WebP image |
| `payload_too_large` | 413 | File or image dimensions exceed the configured limits |
| `not_found` | 404 | Unknown image or job |
| `timeout` | 504 | Operation exceeded its time budget |
| `cancelled` | 409 | Operation was cancelled |
| `internal` | 500 | Unexpected failure; details are logged, never returned |

## Images

| Route | Purpose |
|---|---|
| `POST /images` | Multipart upload with a `file` field. `201` for a new image, `200` if identical bytes were already stored. |
| `GET /images/{id}` | Metadata (format, size, content hash). |
| `GET /images/{id}/original` | Original bytes, with long-lived cache headers. |
| `GET /images/{id}/thumbnail` | JPEG thumbnail (longest side 512 px). |

## Jobs and event streams

Long-running work (analysis, generation) runs as a background job.

| Route | Purpose |
|---|---|
| `GET /jobs/{id}` | Current state: `running`, `completed`, `failed` or `cancelled`. |
| `GET /jobs/{id}/events` | Server-sent events. Replays everything emitted so far, then streams live, and ends after the terminal event. |
| `DELETE /jobs/{id}` | Request cancellation (`202`). Cancelling a finished job has no effect. |

Event types (the `type` field of each JSON payload, also used as the SSE event name): `progress`, `log`, `completed`, `failed`, `cancelled`. Jobs that exceed `HANDS_OFF_JOB_TIMEOUT_SECS` fail with the `timeout` code.

## Configuration

Everything has a default. Set variables in the environment, in `.env`, or in `local.env`.

| Variable | Default | Purpose |
|---|---|---|
| `OPENROUTER_API_KEY` | none | Needed from Phase 4 on, to call hosted models |
| `HANDS_OFF_BIND` | `127.0.0.1:8080` | Listen address |
| `HANDS_OFF_DATA_DIR` | `data` | Database and image blobs |
| `HANDS_OFF_MODEL_DIR` | `models` | Downloaded model weights |
| `HANDS_OFF_MAX_UPLOAD_MB` | `25` | Maximum upload size |
| `HANDS_OFF_MAX_DIMENSION` | `8192` | Maximum image width or height |
| `HANDS_OFF_JOB_TIMEOUT_SECS` | `180` | Maximum job run time |
| `HANDS_OFF_CORS_ORIGINS` | `http://localhost:5173,http://127.0.0.1:5173` | Allowed browser origins |
| `RUST_LOG` | `info,hands_off_api=debug` | Log verbosity |
| `HANDS_OFF_LOG_FORMAT` | text | Set to `json` for structured logs |
