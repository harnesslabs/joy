---
date: 2026-03-05
commit: 1d9cd45
tags:
  - roadmap
  - phase77
  - phase78
  - phase79
  - phase80
  - phase82
  - dependency
  - registry
  - lockfile
related_components:
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/package_coordinate.rs
  - /Users/autoparallel/Code/joy/src/registry.rs
  - /Users/autoparallel/Code/joy/src/registry_config.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/src/commands/update.rs
  - /Users/autoparallel/Code/joy/src/commands/remove.rs
  - /Users/autoparallel/Code/joy/src/commands/registry_cmd.rs
  - /Users/autoparallel/Code/joy/src/commands/search.rs
  - /Users/autoparallel/Code/joy/src/commands/info.rs
  - /Users/autoparallel/Code/joy/src/commands/fetch.rs
  - /Users/autoparallel/Code/joy/src/commands/vendor.rs
  - /Users/autoparallel/Code/joy/src/commands/cache.rs
  - /Users/autoparallel/Code/joy/tests/multi_source_dependency_commands.rs
---

# Phase 77-86 Wave: Cargo/uv-Style Usability (CLI-first)

## Why

`joy` had strong graph/build ergonomics, but dependency consumption was still bottlenecked by recipe presence and github-only assumptions. This wave starts the Cargo/uv model: package identity separated from source backend, explicit source metadata in manifest/lockfile, and named registries/discovery in CLI.

## What Landed

1. Source/identity foundation:
   - generalized coordinate support via `PackageCoordinate`
   - manifest dependency source expansion: `registry|github|git|path|archive`
   - compatibility path retained for legacy `owner/repo` manifests
2. Multi-source dependency UX:
   - `joy add/update/remove` handle `git/path/archive` in staged-manifest mode
   - registry-first default behavior remains for canonical package ids
   - stable validation errors for unsupported argument/source combinations
3. Named registry and discovery commands:
   - `joy registry list|add|remove|set-default`
   - `joy search <query>`
   - `joy info <package>`
   - project + user scoped registry configuration
4. Lockfile/provenance schema expansion:
   - additive lock fields for source provenance (`source_git`, `source_path`, `source_url`, `source_checksum_sha256`)
   - strict frozen/locked compatibility preserved for legacy lockfiles
5. Offline-first command surface:
   - `joy fetch`
   - `joy vendor`
   - `joy cache gc`

## Validation

- `just lint` (pass)
- `just test` (pass)
- Added integration coverage for non-legacy source flows:
  - add git with alias + rev
  - archive checksum validation + update
  - path remove lifecycle

## Blockers & Solutions

- Blocker: legacy JSON key-shape and frozen lockfile tests regressed after additive source metadata work.
- Solution:
  - preserve historical JSON key sets for legacy github/registry command payloads
  - avoid lock mismatch churn by keeping new provenance fields additive without forcing frozen mode lock rewrites.

## Fallback Plan

If non-legacy source rollout proves too unstable:

1. Keep `git/path/archive` manifest recording enabled but gate sync/build consumption as staged-only.
2. Keep resolver hard-failing unsupported source backends with explicit guidance.
3. Continue shipping registry/github reliability improvements while provider backends mature behind flags.
