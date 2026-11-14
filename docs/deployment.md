# Deployment

Hands-Off is one process: the server also serves the web app. It keeps its data in one directory.

## Docker (recommended)

```sh
cp .env.example .env        # add OPENROUTER_API_KEY, or leave it and paste the key in the app
docker compose up --build
```

Open <http://localhost:8080>. The `hands-off-data` volume holds the database, images and downloaded models, so
upgrading is `docker compose pull && docker compose up -d` (or rebuild) and nothing is lost.

The image runs as an unprivileged user, has a health check (`hands-off healthcheck`), and does not contain model weights.

### Detection model

The detection model is downloaded on first start from `HANDS_OFF_RFDETR_SEG_URL` (checked against
`HANDS_OFF_RFDETR_SEG_SHA256` when given) into `/data/models`. Release images have both baked in. When you build
the image yourself, either set them in `.env` before building or at runtime, or export the model locally with
`just models` and mount it:

```yaml
    volumes:
      - ./models:/data/models:ro
```

## Release binaries

Each release has archives for Linux and macOS containing the `hands-off` binary and the built web app. Unpack,
then:

```sh
HANDS_OFF_WEB_DIR=./web ./hands-off
```

Set `HANDS_OFF_RFDETR_SEG_URL` as above, or put `rfdetr-seg-medium.onnx` in `./models`. Checksums
(`SHA256SUMS`) and signatures are published with each release.

## Configuration

Every variable, with defaults, is in the table in [api.md](api.md#configuration). The ones you will most often set:

| Variable | Why |
|---|---|
| `OPENROUTER_API_KEY` | Needed for suggestions, control placement and generating edits. |
| `HANDS_OFF_BIND` | Where to listen. The image sets `0.0.0.0:8080`. |
| `HANDS_OFF_MAX_UPLOAD_MB`, `HANDS_OFF_MAX_DIMENSION` | Upload limits. |
| `HANDS_OFF_CORS_ORIGINS` | Only needed when the web app is served from a different origin than the API. |
| `HANDS_OFF_LOG_FORMAT=json` | Structured logs for a log pipeline. |

## Putting it on a network

There are no accounts: anyone who can reach the server can use it, spend your OpenRouter credits and see every
image. Keep it on `localhost` or a private network, or put an authenticating reverse proxy in front of it
(for example one that does single sign-on) and terminate TLS there. Set `HANDS_OFF_CORS_ORIGINS` only if you
serve the web app from another origin.

Run one replica: job state is held in memory and storage is a local directory and a SQLite file.

## Backups

Back up the data directory (`/data` in the container). Stop the server first, or use SQLite's online backup, to
get a consistent copy of the database. Model weights can always be downloaded again.

## Resources

Detection runs on the CPU in about half a second per photo on a recent laptop, and the server uses a few hundred
MB of memory. Generation time is the model's: roughly 10 to 20 seconds per edit.
