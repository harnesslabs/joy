---
date: 2026-03-05
commit: 88c8c9e
tags:
  - phase85
  - publish
  - registry
  - owner
  - yank
related_components:
  - /Users/autoparallel/Code/joy/src/commands/publishing.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/registry.rs
  - /Users/autoparallel/Code/joy/tests/publish_command.rs
---

# Phase 85: Self-Hosted Publishing Workflow

## Why

Consumption UX was improving, but there was still no built-in consume-to-publish loop for self-hosted ecosystems. Teams needed package-init, publish, owner management, and yank control without relying on centralized Joy infrastructure.

## What Landed

1. New CLI surfaces:
   - `joy package init`
   - `joy publish`
   - `joy owner list|add|remove`
   - `joy yank [--undo]`
2. Local/self-hosted registry index mutation workflow:
   - publish writes/updates `index.toml` entries with version, source package/rev, manifest summary, and `yanked` state.
   - owner commands mutate per-package owner lists.
   - yank toggles release yanked state.
3. Git-backed index update support:
   - when index path is a git working tree, mutations stage+commit `index.toml` automatically.
4. Resolver behavior hardening:
   - yanked versions are filtered out during registry resolution and version listing.
5. End-to-end integration test:
   - package init → publish → owner add/list → registry add/install
   - yank prevents new install
   - unyank restores installability.

## Validation

- `just lint` (pass)
- `just test` (pass)
- integration coverage:
  - `/Users/autoparallel/Code/joy/tests/publish_command.rs`

## Blockers & Solutions

- Blocker: resolver currently loads the default registry store globally, so explicit named-registry dependency routing is not fully wired yet.
- Solution: tests pin `JOY_REGISTRY_DEFAULT` for roundtrip reliability while preserving named registry metadata in command/index flows.

## Fallback Plan

If publish protocol support needs to be narrowed temporarily:

1. keep `package init` and index editing command contracts.
2. restrict git auto-commit behavior behind an explicit flag while retaining raw index file writes.
3. keep yanked filtering in resolver to preserve install safety.
