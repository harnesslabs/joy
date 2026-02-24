# Joy Roadmap Tracker

Source: user-provided master roadmap (Feb 24, 2026) and the decision-complete execution plan created in this session.

## Current Status

- Current phase: Phase 5 dependency graph & compiled libraries (next)
- Active branch: `codex/phase4-local-build-engine`
- Last completed milestone: Phase 4 local build engine (`14aabda`)
- Current focus: start Phase 5 recipe loader + exact-ref resolver groundwork

## Locked Decisions

- Package coordinates: GitHub shorthand first (`owner/repo`)
- Windows priority: MinGW first (`x86_64-pc-windows-gnu`)
- Compiled dependency strategy: CMake adapter first
- Dependency metadata source: curated recipe index
- Versioning policy: exact refs only (`HEAD`, tag, branch, SHA)
- Machine mode: baseline `--json` support in Phase 1

## Phase Checklist

- [x] Phase 0: prep/bootstrap (tracker, justfile, crate split, baseline deps/tests)
- [x] Phase 1: CLI foundation & scaffolding
- [x] Phase 2: manifest parsing & local `.joy` environment
- [x] Phase 3: fetcher & header-only packages
- [x] Phase 4: compiler discovery & local build engine
- [ ] Phase 5: dependency graph & compiled libraries
- [ ] Phase 6: lockfiles & production polish

## Immediate Milestones (Execution Order)

- [x] Phase 0 tracker/bootstrap commit (combined with early Phase 1 shell in `7edc164`)
- [x] Phase 1 CLI shape + JSON output commit (combined in `7edc164`)
- [x] Phase 1 scaffolding implementation commit (`b7b8b6b`)
- [x] Phase 1 integration tests + justfile polish commit (`b7b8b6b`)
- [x] Phase 2 manifest parsing commit (delivered in `dbdcb0e`)
- [x] Phase 2 `joy add` + `.joy` env commit (delivered in `dbdcb0e`)
- [x] Phase 2 tests/snapshots commit (completed with `dbdcb0e` + `fe7d586`)
- [x] Phase 3 cache/fetch core commit (`68f879a`) - git mirror cache + commit materialization
- [x] Phase 3 header linking + `joy add` integration commit (`68f879a`) - `.joy/include/deps/<slug>` install path
- [x] Phase 3 archive fetch backend + mockito tests (`ba47dfc`)
- [x] Phase 3 cross-platform symlink/copy fallback assertions (`9d7a05d`) - explicit copy-path coverage
- [x] Phase 4 toolchain discovery (`14aabda`)
- [x] Phase 4 ninja build file generation (`14aabda`)
- [x] Phase 4 `joy build`/`joy run` execution + E2E tests (`14aabda`)
- [ ] Phase 5 recipes + resolver + DAG
- [ ] Phase 5 ABI hash + cache build layout
- [ ] Phase 5 CMake adapter + linking

## Notes Convention

Per `.codex/AGENTS.md`, create one note per milestone/decision in `notes/` using:

- `YYYY-MM-DD_<commit-hash>_<topic-slug>.md`

Each note must include:

- Why
- Blockers & Solutions
- Fallback Plan
