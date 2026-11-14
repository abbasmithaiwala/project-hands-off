# Contributing to Hands-Off

Thanks for helping. This project aims to feel like a polished desktop tool and to stay easy to
change, so a few conventions matter. They are all in [CLAUDE.md](CLAUDE.md); this page is the short version.

## Set up

Requirements: Rust (the exact version is in `rust-toolchain.toml`), Node 22+,
[`just`](https://github.com/casey/just), `cargo-nextest` and `cargo-deny`.

```sh
cd web && npm ci && cd ..
just models                     # export the detection model once (needs Python 3)
HANDS_OFF_DEMO_AI=1 just serve  # API on :8080, no API key needed
just dev                        # web app on :5173
```

Without `OPENROUTER_API_KEY` or `HANDS_OFF_DEMO_AI=1`, the AI steps report that a key is needed.

## Before you open a pull request

```sh
just check   # format, lint, type-check, unit and integration tests, dependency audit, build
just e2e     # Playwright, against a mocked API (no credits, no network)
```

Both must pass. If you change the API, run `just gen-api` and commit the regenerated
`web/openapi.json` and `web/src/api/schema.d.ts`; CI fails on any difference.

## What a good change looks like

- **Small and focused.** One idea per pull request, with a Conventional Commit title
  (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).
- **Tested.** Add or update tests with the code; cover the failure paths too. Tests never call a
  real model or touch the network: use the mock provider and fake generator.
- **Documented.** Keep `docs/` and rustdoc in the same change. Record a significant technical decision as a short
  ADR in `docs/adr/`.
- **Typed end to end.** Data contracts are defined once in Rust and generated for TypeScript.
  Do not hand-write a type that exists on the other side.
- **No comment noise.** Comments explain *why*, not what.

Guides for common extensions are in [docs/extending.md](docs/extending.md): a new widget type,
property pack, detector or model provider.

## Reporting problems and ideas

Use the issue templates. For security problems, follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

By contributing you agree that your work is licensed under the Apache License 2.0, and you agree to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).
