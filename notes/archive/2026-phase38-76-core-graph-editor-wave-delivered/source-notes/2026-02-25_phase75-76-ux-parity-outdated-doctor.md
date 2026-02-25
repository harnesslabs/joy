---
date: 2026-02-25
commit: d5daeae
tags:
  - phase75
  - phase76
  - outdated
  - doctor
  - tree
  - why
  - docs
  - qa
related_components:
  - /Users/autoparallel/Code/joy/src/commands/outdated.rs
  - /Users/autoparallel/Code/joy/src/commands/doctor.rs
  - /Users/autoparallel/Code/joy/src/commands/tree.rs
  - /Users/autoparallel/Code/joy/src/commands/why.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - /Users/autoparallel/Code/joy/tests/smoke.rs
  - /Users/autoparallel/Code/joy/book/src/getting-started/quickstart.md
  - /Users/autoparallel/Code/joy/book/src/troubleshooting/common-failures.md
  - /Users/autoparallel/Code/joy/book/src/dependencies/registry.md
---

# Phase 75-76: UX Parity Follow-Through (`outdated`, `doctor`, provenance, docs)

## Why

After the nested dependency graph and registry v2 work landed, the roadmap still needed the final user-facing loop closures:

- a direct/transitive upgrade visibility command (`joy outdated`)
- diagnostics that explain editor and graph state (`joy doctor`)
- provenance visibility in `tree`/`why`
- docs that teach the compile DB workflow and nested dependency debugging path

Without these, the new graph/editor capabilities existed but were not inspectable enough for everyday usage.

## What Landed

### `joy outdated`

- Added `joy outdated` CLI + command implementation in `/Users/autoparallel/Code/joy/src/commands/outdated.rs`
- Lockfile-driven command with explicit stale/incomplete lockfile errors
- Reports direct + transitive packages from `joy.lock`
- Registry-backed version checks for registry packages
- Preserves visibility for non-registry packages via `unsupported_source` rows (rather than silently omitting them)
- Human and JSON output include:
  - summary counts
  - per-package status
  - latest compatible / latest available when known

### `joy doctor` Artifact + Metadata Diagnostics

- Reworked `/Users/autoparallel/Code/joy/src/commands/doctor.rs` to inspect project context in addition to environment/toolchain state
- Added project diagnostics for:
  - manifest presence and manifest kind
  - direct dependency count
  - `.joy/state/dependency-graph.json` presence/parseability
  - root `compile_commands.json` presence
  - target-specific compile DB count under `.joy/build`
  - `joy.lock` presence/freshness/parseability
  - dependency metadata provenance counts (`recipe`, `package_manifest`, `registry_manifest`, `none`)
- Human output now emits actionable warnings/hints when graph/editor artifacts are missing or stale

### `joy tree` / `joy why` Provenance Polish

- `tree --json` and `tree --locked --json` now include package provenance fields:
  - `metadata_source`
  - `package_manifest_digest`
  - `declared_deps_source`
- `why --json` includes the same provenance fields in `package_info`
- Human `tree` and `why` output surfaces metadata provenance when available
- Unlocked mode overlays provenance from a fresh lockfile when available (best effort)

### Docs / Troubleshooting

- Quickstart updated to document:
  - `joy add` sync-lite behavior
  - auto-generated compile databases (`compile_commands.json`, `.joy/build/compile_commands.<target>.json`)
  - `.joy/state/dependency-graph.json`
- Troubleshooting guide now covers:
  - VSCode/clangd include resolution via compile DB
  - nested dependency debugging (`joy tree`, `joy why`, `joy doctor`)
  - `package_metadata_mismatch`
- Registry docs updated for v1/v2 support and embedded manifest summary fallback/verification

## Tests Added / Updated

- CLI parse test for `joy outdated`
- Integration test for `joy outdated` direct + transitive registry updates
- Integration test covering new doctor project artifact warnings/hints
- Existing nested dependency and lockfile graph tests extended to assert provenance fields in `tree`/`why`
- Smoke test updated to assert expanded doctor JSON sections

## Validation Snapshot

Validated with CI-parity commands after implementation:

- `just ci-pr`
- (includes `just lint`, `just test`, `just compiled-e2e`, `just ci-docs`, and TOML/doc checks)

## Blockers & Solutions

- Blocker: `outdated` needed transitive source/version context not present in manifest-only state.
- Solution: make `outdated` lockfile-driven and require fresh lockfile package metadata, returning explicit recovery guidance when stale/incomplete.

- Blocker: provenance visibility should work in unlocked `tree`/`why` without forcing resolver JSON contract churn.
- Solution: overlay provenance from a fresh lockfile when available and add fields additively to JSON package objects.

- Blocker: doctor output risked becoming a monolith while adding project-specific checks.
- Solution: split project inspection into structured JSON sections (`project`, `artifacts`, `lockfile`, `dependency_metadata`, warnings/hints) and render human output from that structure.

## Fallback Plan

If `joy outdated` proves too strict for early users without fresh lockfiles:

1. Keep the lockfile-first implementation as the canonical path.
2. Downgrade `lockfile_incomplete` to a warning plus partial output (direct packages only) while preserving the current JSON shape.
3. Keep registry-only version checks for now; expand non-registry sources later behind explicit source-specific adapters instead of ad hoc heuristics.
