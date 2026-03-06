---
date: 2026-03-06
commit: pending
tags:
  - phase87
  - phase88
  - phase89
  - phase90
  - resolver
  - fetch
  - lockfile
  - verify
---

# Phase 87-90 Source Backend Parity

## Why
The existing staged-only model for `git`, `path`, and `archive` dependencies prevented `joy` from feeling usable like `cargo`/`uv`. We moved to end-to-end backend dispatch so add/update/sync/build/run/fetch/vendor/verify can consume non-GitHub sources directly with deterministic lock provenance.

## What changed
- Added backend-specific fetch flows in `/Users/autoparallel/Code/joy/src/fetch.rs` for `git`, `path`, and `archive`.
- Propagated source provenance and deterministic identities through `/Users/autoparallel/Code/joy/src/resolver.rs` and lockfile writes in build pipeline.
- Removed staged-only behavior in add/update/remove/fetch/vendor/build command paths.
- Expanded `outdated --sources` to support `all|registry|github|git|path|archive` with backend-specific result rows.
- Updated strict verification to validate usable provenance for non-GitHub backends and avoid invalid checksum semantics.
- Added integration coverage in `/Users/autoparallel/Code/joy/tests/multi_source_dependency_commands.rs` for mixed-source add/update/remove/outdated/verify flows.

## Blockers and solutions
- **Blocker:** `add` sync-lite exposed named-registry resolution mismatch in resolver (project-scoped registry config not respected).
  - **Solution:** switched resolver registry loads to project-aware paths so lock resolution matches command-level registry behavior.
- **Blocker:** strict verify falsely required `source_checksum_sha256` for all sources.
  - **Solution:** constrained checksum requirement semantics to source-specific rules and archive behavior.

## Fallback plan
If backend parity causes instability:
1. Revert this commit and keep non-GitHub sources as manifest-recorded only.
2. Re-enable staged-only warnings in add/update.
3. Reintroduce GitHub/registry-only dispatch in build/fetch/vendor while preserving additive lockfile fields.
