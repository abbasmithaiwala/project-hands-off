# 2. Frontend tooling: npm and oxlint

Date: 2026-11-14
Status: Accepted

## Context
`plan.md` and `CLAUDE.md` assumed pnpm and ESLint. On the development machine the pnpm corepack shim was broken, and the Vite template ships with oxlint.

## Decision
Use npm for dependency management and oxlint with Prettier for linting and formatting.

## Consequences
Fewer moving parts and faster linting. Switching to pnpm or ESLint later is a contained change.
