# 12. MIT licence

Date: 2026-11-14
Status: Accepted (supersedes the Apache-2.0 default chosen in Phase 0)

## Context
The project started under Apache-2.0, the choice recommended in the plan for its patent grant. The goal is for anyone to be able to use, change and self-host Hands-Off with the least friction, and the project has a single copyright holder, so the licence can be changed.

## Decision
Hands-Off is released under the MIT License. `LICENSE`, the workspace `license` in `Cargo.toml`, the OpenAPI document, the README and CONTRIBUTING reflect it.

## Consequences
- Simpler terms, widely understood, no NOTICE-file handling for downstream users.
- No explicit patent grant from contributors, which Apache-2.0 would have provided.
- Nothing else changes: every dependency licence in `THIRD_PARTY_NOTICES.md` is compatible with MIT distribution (all are permissive, and `cargo deny` still enforces an allow-list that includes Apache-2.0 for dependencies). Detection model weights stay Apache-2.0 and are downloaded, never bundled. The sample photo stays CC BY-SA 4.0.
- Releases already published under Apache-2.0 keep that licence for those copies.
