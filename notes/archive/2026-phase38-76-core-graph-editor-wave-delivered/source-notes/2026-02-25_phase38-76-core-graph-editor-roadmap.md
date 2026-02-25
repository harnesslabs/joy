---
date: 2026-02-25
commit: 9c3205c
tags:
  - roadmap
  - phase38
  - phase76
  - dependencies
  - graph
  - nested-deps
  - editor
  - compile-db
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/registry.rs
  - /Users/autoparallel/Code/joy/src/install_index.rs
  - /Users/autoparallel/Code/joy/src/linking.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/notes/roadmap.md
  - https://github.com/harnesslabs/joy/issues/101
  - https://github.com/harnesslabs/joy/issues/102
  - https://github.com/harnesslabs/joy/issues/103
  - https://github.com/harnesslabs/joy/issues/104
  - https://github.com/harnesslabs/joy/issues/105
  - https://github.com/harnesslabs/joy/issues/106
  - https://github.com/harnesslabs/joy/issues/107
  - https://github.com/harnesslabs/joy/issues/108
  - https://github.com/harnesslabs/joy/issues/109
---

# Phase 38-76: Core Dependency Graph + Nested Dependencies + Editor Integration

## Why

The UX/docs wave (Phase 19-37) improved usability and documentation, but core package-management ergonomics still have major gaps:

- `joy` resolves a dependency DAG and writes it into `joy.lock`, but `sync/build` do not reliably materialize transitive headers into project-local state
- editor tooling (VSCode/clangd) cannot consume `joy` compiler/include flags because no compile database is generated
- nested dependency resolution for reusable libraries (for example `harnesslabs/igneous -> xsimd`) needs a self-describing package metadata path in addition to curated recipes

This wave prioritizes dependency correctness and editor usability first, then extends the resolver to support package `joy.toml` recursion and registry metadata v2 summaries.

## Backlog Structure (Seeded)

Umbrella tracker:

- #101 `[roadmap] Joy Phase 38-76 core dependency graph + nested deps + editor integration roadmap`

Milestone trackers:

- #102 `phase38-40` Roadmap Seeding and Failure Baselines
- #103 `phase41-47` Transitive Materialization Correctness
- #104 `phase48-53` Dependency Graph State and Introspection
- #105 `phase54-58` Editor Integration Baseline (Compile DB)
- #106 `phase59-62` Add Sync-Lite Semantics
- #107 `phase63-69` Package Manifest Recursion
- #108 `phase70-74` Registry Metadata v2 and Offline Recursion
- #109 `phase75-76` UX Parity and QA Closeout

Baseline / implementation issues:

- #127 - #130 baseline repros + local notes/fixture seeding
- #131 - #146 feature work and QA closeout (graph materialization, compile DB, package manifests, registry v2, outdated/doctor polish, bug-bash)

## Immediate Execution Notes

Current execution order for the first implementation tranche:

1. Phase 38-40 local notes + baseline tests/fixtures
2. #131 transitive header materialization in `sync/build`
3. #132 install-index tracking improvements
4. #137 compile database generation in `build/sync`
5. Integration tests for transitive headers + compile DB
6. Follow-on work for #138 (`joy add` sync-lite) after shared pipeline stabilizes

## Blockers & Solutions

- Blocker: The first GitHub seeding shell script failed mid-run because issue text contained shell-evaluated backticks.
- Solution: Re-ran issue creation in a safer scripted pass without shell interpolation; created umbrella/trackers plus implementation issues #127-#146.

## Fallback Plan

If the full Phase 38-76 wave needs to split across multiple PRs, keep this shipping order:

1. Transitive header materialization + install-index updates
2. Compile database generation + editor docs
3. Graph artifacts and introspection commands (`metadata`, `why`, `tree --locked`)
4. `joy add` sync-lite default
5. Package manifest recursion + lockfile provenance
6. Registry v2 metadata + offline fallback
7. UX parity (`outdated`, doctor checks) + QA closeout
