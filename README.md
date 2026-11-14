# Hands-Off

Edit images by direct manipulation, not prompts. Hover an object, pick a property, drag a handle, and an image model turns your rough preview into a photoreal result. Open source and self-hostable.

Inspired by Adobe's *Project Hands On* (Adobe MAX Sneaks 2026).

> Status: early development. See [plan.md](plan.md) for the design and [process.md](process.md) for the build runbook.

## Development

Requirements: Rust (see `rust-toolchain.toml`), Node 22+, [`just`](https://github.com/casey/just), `cargo-nextest`, `cargo-deny`.

```sh
cd web && npm ci && cd ..
just check     # format, lint, test, dependency audit, build
just serve     # start the API on 127.0.0.1:8080
just dev       # start the web app (proxies /api to the server)
just gen-api   # regenerate the OpenAPI spec and TypeScript client
```

The API is documented in [docs/api.md](docs/api.md), including every configuration variable. To try the whole app without a key, start the server with `HANDS_OFF_DEMO_AI=1 just serve`. See [docs/frontend.md](docs/frontend.md) for the UI, [docs/vision.md](docs/vision.md) for detection and [docs/models.md](docs/models.md) for the AI models.

Project conventions live in [CLAUDE.md](CLAUDE.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
