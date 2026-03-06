---
date: 2026-03-05
commit: 98de2b1
tags:
  - phase83
  - workspace
  - lockfile
  - profile
related_components:
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
---

# Phase 83: Workspace Lock/Profile Parity

## Why

Workspace routing already existed, but lockfile behavior still skewed per-member and profile defaults were CLI-flag-only. That created drift risk for teams running commands from workspace roots in CI.

## What Landed

1. Workspace execution context propagation:
   - dispatch now carries workspace root/member context into project-scoped runtime flags.
2. Workspace-root lockfile strategy in build/sync/add-sync-lite flows:
   - when routed from a workspace root, lockfile path is `<workspace>/joy.lock`
   - manifest hash is computed deterministically from workspace root manifest plus all member manifests.
3. Workspace-wide lock package aggregation:
   - lockfile writes/locked validation aggregate dependency lock entries across workspace members, not only the selected member.
4. Workspace profile defaults:
   - optional `[workspace] profile = "dev"|"release"` now influences `sync/build/run` profile selection when commands are routed from workspace root
   - `--release` remains an explicit override.
5. Validation and tests:
   - invalid workspace profile names are rejected in manifest validation.
   - integration tests verify root lockfile placement, locked sync behavior across members, member-drift stale detection, and profile default propagation.

## Validation

- `just lint` (pass)
- `just test` (pass)
- new tests in `/Users/autoparallel/Code/joy/tests/add_command.rs`:
  - `workspace_root_add_writes_lockfile_once_at_workspace_root`
  - `workspace_locked_sync_detects_other_member_manifest_drift`
  - `workspace_profile_release_defaults_sync_profile_without_release_flag`

## Blockers & Solutions

- Blocker: workspace routing originally changed cwd to member root, so lockfile code paths could not distinguish workspace-root intent.
- Solution: propagate workspace context explicitly in runtime flags and consume it in build/sync pipeline options.

## Fallback Plan

If workspace-wide lock aggregation causes unacceptable overhead:

1. keep workspace-root lockfile path + hash semantics.
2. temporarily scope package aggregation to selected member while preserving stale-hash detection across all member manifests.
3. revisit full multi-member package aggregation behind a flag.
