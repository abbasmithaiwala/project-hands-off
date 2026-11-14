# 2. Frontend tooling: npm, oxlint and TypeScript 5

Date: 2026-11-14
Status: Accepted

## Context
`plan.md` and `CLAUDE.md` assumed pnpm and ESLint. On the development machine the pnpm corepack shim was broken, and the Vite template ships with oxlint.

## Decision
Use npm for dependency management and oxlint with Prettier for linting and formatting. Pin TypeScript to 5.x: the Vite template ships TypeScript 7, whose native compiler has no JavaScript API, and `openapi-typescript` (which generates the API types) needs that API.

## Consequences
Fewer moving parts and faster linting. Switching to pnpm or ESLint later is a contained change.
