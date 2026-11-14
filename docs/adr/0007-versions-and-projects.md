# 7. Versions and projects: lineage on images, no separate project table

Date: 2026-11-14
Status: Accepted

## Context
Phase 8 needs undo/redo, a history of change summaries, reopening saved work, rename, delete and export.

## Decisions

**Lineage lives on the image record.** An accepted edit creates a new image whose `parent_id` points at the version it was edited from and whose `edit` holds the stored `EditResult` (summary, per-edit changes, instruction, model, cost). There is no separate project table: a project is a chain of versions. The user-facing name is copied down the chain and `PATCH /images/{id}` renames every ancestor too, so the whole project shows one name.

**Recent projects are the tips.** `GET /images` lists images nothing was edited from, newest first. Branching (edit an earlier version) simply produces another tip, which is shown as its own project.

**Undo and redo are client-side navigation.** Versions already exist on the server, so undo is "show the previous version"; the client keeps the later ones for redo until a new edit replaces them. Nothing is deleted by undoing.

**Delete is a cascade.** Deleting an image removes every version edited from it (`ON DELETE CASCADE`) and their stored files.

**Export needs no new endpoints.** The image is the existing `/original` download; the JSON edit log is built in the browser from the history response.

**Unaccepted results expire.** Discard deletes the job's artifacts immediately; a sweeper removes finished jobs older than 24 hours at start-up and hourly. Job records go with them.

**Reload reopens the project** through the address (`#/p/<image id>`), always at the latest version.

## Consequences
Identical bytes still deduplicate to one image, so accepting a result that is byte-identical to an existing image reuses that record and its lineage. Renaming a middle version names its ancestors but not later versions.
