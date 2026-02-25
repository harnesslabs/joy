# Joy Roadmap Tracker (Phase 38-76 Core Graph + Nested Deps + Editor Wave Seeded)

The Phase 19-37 UX/docs wave was delivered and merged (PR #100). A new core-focused wave is now seeded to close dependency graph/materialization gaps, add nested dependency resolution, and generate editor artifacts (`compile_commands.json`) so `joy` feels closer to `uv`/`cargo`.

## Archived Milestones

- Phase 1-6 milestone archive:
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/MILESTONE.md`
- Phase 7-18 source-note compaction archive:
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase7-18-roadmap-waves-delivered/MILESTONE.md`

## Current Status

- Roadmap umbrella tracker: [#101](https://github.com/harnesslabs/joy/issues/101)
- Current wave: Phase 38-76 (core dependency graph + nested deps + editor integration)
- Branch in use (current local): `codex/phase38-76-core-graph-editor-wave`
- Primary active note: `/Users/autoparallel/Code/joy/notes/active/2026-02-25_phase38-76-core-graph-editor-roadmap.md`
- Prior wave roadmap note (Phase 19-37): `/Users/autoparallel/Code/joy/notes/active/2026-02-25_phase19-37-ux-docs-wave-roadmap.md`
- Prior wave QA closeout note (Phase 37): `/Users/autoparallel/Code/joy/notes/active/2026-02-25_phase37-ux-docs-wave-qa-closeout.md`

## Milestone Trackers (Current Wave)

- Phase 38-40 tracker [#102](https://github.com/harnesslabs/joy/issues/102) (milestone 18: Roadmap Seeding and Failure Baselines)
- Phase 41-47 tracker [#103](https://github.com/harnesslabs/joy/issues/103) (milestone 19: Transitive Materialization Correctness)
- Phase 48-53 tracker [#104](https://github.com/harnesslabs/joy/issues/104) (milestone 20: Dependency Graph State and Introspection)
- Phase 54-58 tracker [#105](https://github.com/harnesslabs/joy/issues/105) (milestone 21: Editor Integration Baseline)
- Phase 59-62 tracker [#106](https://github.com/harnesslabs/joy/issues/106) (milestone 22: Add Sync-Lite Semantics)
- Phase 63-69 tracker [#107](https://github.com/harnesslabs/joy/issues/107) (milestone 23: Package Manifest Recursion)
- Phase 70-74 tracker [#108](https://github.com/harnesslabs/joy/issues/108) (milestone 24: Registry Metadata v2 and Offline Recursion)
- Phase 75-76 tracker [#109](https://github.com/harnesslabs/joy/issues/109) (milestone 25: UX Parity and QA Closeout)

## Implementation Issue Inventory (Current Wave)

- Baselines / seeding:
  - [#127](https://github.com/harnesslabs/joy/issues/127) Nested dependency repro (`harnesslabs/igneous` -> missing `xsimd`)
  - [#128](https://github.com/harnesslabs/joy/issues/128) VSCode/IntelliSense include-resolution repro after `joy add nlohmann/json`
  - [#129](https://github.com/harnesslabs/joy/issues/129) `sync/build` transitive header materialization gap repro
  - [#130](https://github.com/harnesslabs/joy/issues/130) Notes + fixture baseline seeding
- Core graph/materialization:
  - [#131](https://github.com/harnesslabs/joy/issues/131) Graph-driven transitive header materialization
  - [#132](https://github.com/harnesslabs/joy/issues/132) Install index v2 + migration support
  - [#133](https://github.com/harnesslabs/joy/issues/133) `.joy/state/dependency-graph.json` artifact
- Graph introspection:
  - [#134](https://github.com/harnesslabs/joy/issues/134) `joy metadata`
  - [#135](https://github.com/harnesslabs/joy/issues/135) `joy why`
  - [#136](https://github.com/harnesslabs/joy/issues/136) `joy tree --locked`
- Editor + add UX:
  - [#137](https://github.com/harnesslabs/joy/issues/137) Compile DB generation (`compile_commands.json`)
  - [#138](https://github.com/harnesslabs/joy/issues/138) `joy add` sync-lite default + `--no-sync`
- Nested dependency metadata / resolver:
  - [#139](https://github.com/harnesslabs/joy/issues/139) Package/library `joy.toml` support
  - [#140](https://github.com/harnesslabs/joy/issues/140) Recursive nested resolver expansion from package manifests
  - [#141](https://github.com/harnesslabs/joy/issues/141) Lockfile provenance fields for nested graph validation
- Registry v2 / offline recursion:
  - [#142](https://github.com/harnesslabs/joy/issues/142) Registry index v2 embedded metadata summaries
  - [#143](https://github.com/harnesslabs/joy/issues/143) Offline fallback + metadata digest verification
- UX parity / QA closeout:
  - [#144](https://github.com/harnesslabs/joy/issues/144) `joy outdated`
  - [#145](https://github.com/harnesslabs/joy/issues/145) `joy doctor` editor/package-metadata checks
  - [#146](https://github.com/harnesslabs/joy/issues/146) Wave bug-bash + QA closeout

## Immediate Execution Queue

1. Land Phase 38-40 local notes/baselines updates and start implementation on [#131](https://github.com/harnesslabs/joy/issues/131) and [#137](https://github.com/harnesslabs/joy/issues/137)
2. Patch `sync/build` to materialize transitive headers from the resolved graph and track installed header paths safely
3. Add compile database generation (`.joy/build/compile_commands.<target>.json` + root `compile_commands.json`) in `build/sync`
4. Add baseline/feature integration tests for transitive header materialization and compile DB generation
5. Follow with `joy add` sync-lite default work ([#138](https://github.com/harnesslabs/joy/issues/138)) once the shared build/sync pipeline is stable

## Locked Priorities

- Preserve `--json` / machine-envelope compatibility
- Fix dependency usability gaps first (nested/transitive materialization)
- Generate editor artifacts automatically (compile DB baseline; no VSCode extension yet)
- Keep recipes authoritative for compiled build/link metadata while introducing package manifest recursion later in the wave
- Keep `notes/active/` compact and current
