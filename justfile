set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

fmt:
    cargo fmt --all
    cd web && npm run format

fmt-check:
    cargo fmt --all --check
    cd web && npm run format:check

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cd web && npm run typecheck && npm run lint

test:
    cargo nextest run --workspace --no-tests=pass
    cd web && npm test

deny:
    cargo deny check

gen-api:
    @echo "OpenAPI and TypeScript client generation is added in Phase 2."

build:
    cargo build --workspace
    cd web && npm run build

dev:
    cd web && npm run dev

check: fmt-check lint test deny build
