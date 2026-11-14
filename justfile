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
    cargo run --quiet -p hands-off-api --bin openapi > web/openapi.json
    cd web && npx openapi-typescript openapi.json -o src/api/schema.d.ts
    cd web && npx prettier --write openapi.json src/api/schema.d.ts

build:
    cargo build --workspace
    cd web && npm run build

dev:
    cd web && npm run dev

serve:
    cargo run -p hands-off-api --bin hands-off

check: fmt-check lint test deny build
