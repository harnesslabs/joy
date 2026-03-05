# Joy Roadmap Tracker (Phase 77-86 Wave Active)

Phase 38-76 is archived and complete. The active roadmap wave is Phase 77-86: Cargo/uv-grade C++ usability with multi-source dependency consumption, named registries, deterministic lock provenance, and offline-first workflows.

## Current Wave Note

- Active wave detail:
  - `/Users/autoparallel/Code/joy/notes/active/2026-03-05_phase77-86-cargo-uv-usability-roadmap.md`

## Phase Status (77-86)

- Phase 77 Source/Identity foundation: in progress (core model + compatibility path implemented)
- Phase 78 Multi-source add/update/remove UX: in progress (CLI + manifest integration implemented; resolver support remains limited to github/registry)
- Phase 79 Named registries + discovery commands: in progress (`joy registry`, `joy search`, `joy info` implemented)
- Phase 80 Universal resolver + lock provenance: in progress (additive lockfile fields + unsupported-source diagnostics; full provider backend support pending)
- Phase 81 Generic compiled dependency support (CMake-first): completed (`1dab34d`)
- Phase 82 Offline-first operations: in progress (`joy fetch`, `joy vendor`, `joy cache gc` implemented)
- Phase 83 Workspace/profile parity: completed (`98de2b1`)
- Phase 84 Supply-chain hardening (`joy verify`, SBOM, signatures): completed (`a6d8661`)
- Phase 85 Self-hosted publishing workflow: completed (`88c8c9e`)
- Phase 86 Usability closeout + editor gate: completed (`bb9790b`)

## Validation Snapshot

- `just lint` passing
- `just test` passing
- New multi-source integration coverage:
  - `/Users/autoparallel/Code/joy/tests/multi_source_dependency_commands.rs`

## Archived Milestones

- `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/MILESTONE.md`
- `/Users/autoparallel/Code/joy/notes/archive/2026-phase7-18-roadmap-waves-delivered/MILESTONE.md`
- `/Users/autoparallel/Code/joy/notes/archive/2026-phase38-76-core-graph-editor-wave-delivered/MILESTONE.md`
