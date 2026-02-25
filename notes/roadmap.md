# Joy Roadmap Tracker (Phase 38-76 Delivered)

Phase 38-76 (core dependency graph + nested dependencies + editor integration) is complete on branch `codex/phase38-76-core-graph-editor-wave`.

## Milestone Status

- Roadmap umbrella tracker: [#101](https://github.com/harnesslabs/joy/issues/101)
- Phase trackers: [#102](https://github.com/harnesslabs/joy/issues/102) through [#109](https://github.com/harnesslabs/joy/issues/109)
- Implementation issue wave: [#127](https://github.com/harnesslabs/joy/issues/127) through [#146](https://github.com/harnesslabs/joy/issues/146)
- Feature delivery commits (local branch): `8524cae`, `d5daeae`
- Phase 38-76 milestone archive:
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase38-76-core-graph-editor-wave-delivered/MILESTONE.md`

## Delivered Capabilities (Phase 38-76)

- Graph-driven transitive header materialization and safer install cleanup tracking
- Canonical header install layout + compatibility aliasing for includes
- Auto-generated dependency graph state artifact (`.joy/state/dependency-graph.json`)
- `joy metadata`, `joy why`, and `joy tree --locked`
- Compile database generation:
  - `.joy/build/compile_commands.<target>.json`
  - root `compile_commands.json`
- `joy add` sync-lite by default + `--no-sync`
- Package `joy.toml` support for reusable package metadata (`ManifestDocument::Package`)
- Recursive nested dependency expansion from package manifests
- Registry index v2 embedded manifest summaries + offline fallback path
- Lockfile provenance metadata (`metadata_source`, `package_manifest_digest`, `declared_deps_source`)
- `joy outdated` (direct + transitive lockfile-driven visibility, registry-backed where available)
- Expanded `joy doctor` project/editor/graph/metadata diagnostics
- Provenance display polish in `joy tree` / `joy why`
- Docs/troubleshooting updates for compile DB/editor use and nested dependency debugging

## Validation Snapshot (Wave Closeout)

- `just ci-pr` (passing)
- Manual offline warm-cache user-flow QA:
  - `joy new`
  - `joy add nlohmann/json --offline`
  - compile DB + graph artifact checks
  - `joy metadata`, `joy tree --locked`, `joy why --locked`, `joy doctor`, `joy outdated`

## Archived Milestones

- Phase 1-6 milestone archive:
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/MILESTONE.md`
- Phase 7-18 source-note compaction archive:
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase7-18-roadmap-waves-delivered/MILESTONE.md`
- Phase 38-76 core graph/editor wave archive:
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase38-76-core-graph-editor-wave-delivered/MILESTONE.md`

## Next Planning Handoff

The next roadmap wave should focus on release hardening and first-public-release readiness (packaging/install experience, platform matrices, publishing workflow, and ecosystem scale-up), using the new graph/editor foundations shipped in Phase 38-76.
