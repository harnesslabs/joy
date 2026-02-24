# Joy Roadmap Tracker

Source: user-provided master roadmap (Feb 24, 2026) and the decision-complete execution plan created in this session.

## Current Status

- Current phase: Phase 2 manifest parsing & local environment
- Active branch: `codex/phase1-cli-foundation`
- Last completed milestone: Phase 1 scaffolding + integration tests (`b7b8b6b`)
- Current focus: commit Phase 2 manifest parsing, `joy add`, and `.joy` environment creation

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
- [ ] Phase 2: manifest parsing & local `.joy` environment
- [ ] Phase 3: fetcher & header-only packages
- [ ] Phase 4: compiler discovery & local build engine
- [ ] Phase 5: dependency graph & compiled libraries
- [ ] Phase 6: lockfiles & production polish

## Immediate Milestones (Execution Order)

- [x] Phase 0 tracker/bootstrap commit (combined with early Phase 1 shell in `7edc164`)
- [x] Phase 1 CLI shape + JSON output commit (combined in `7edc164`)
- [x] Phase 1 scaffolding implementation commit (`b7b8b6b`)
- [x] Phase 1 integration tests + justfile polish commit (`b7b8b6b`)
- [ ] Phase 2 manifest parsing commit
- [ ] Phase 2 `joy add` + `.joy` env commit
- [ ] Phase 2 tests/snapshots commit

## Notes Convention

Per `.codex/AGENTS.md`, create one note per milestone/decision in `notes/` using:

- `YYYY-MM-DD_<commit-hash>_<topic-slug>.md`

Each note must include:

- Why
- Blockers & Solutions
- Fallback Plan
