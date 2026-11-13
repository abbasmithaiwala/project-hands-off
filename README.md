# Hands-Off

Edit images by direct manipulation, not prompts. Hover an object, pick a property, drag a handle, and an image model turns your rough preview into a photoreal result. Open source and self-hostable.

Inspired by Adobe's *Project Hands On* (Adobe MAX Sneaks 2026).

> Status: early development. See [plan.md](plan.md) for the design and [process.md](process.md) for the build runbook.

## Development

Requirements: Rust (see `rust-toolchain.toml`), Node 22+, [`just`](https://github.com/casey/just), `cargo-nextest`, `cargo-deny`.

```sh
cd web && npm ci && cd ..
just check   # format, lint, test, dependency audit, build
just dev     # start the web app
```

Project conventions live in [CLAUDE.md](CLAUDE.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
