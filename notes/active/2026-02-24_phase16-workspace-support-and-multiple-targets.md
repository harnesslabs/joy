---
date: 2026-02-24
commit: f04d327
tags:
  - phase16
  - workspace
  - targets
  - manifest
  - build
  - ninja
  - tests
  - docs
related_components:
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/run.rs
  - /Users/autoparallel/Code/joy/src/templates.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - /Users/autoparallel/Code/joy/tests/build_run.rs
  - /Users/autoparallel/Code/joy/README.md
  - /Users/autoparallel/Code/joy/docs/machine-interface.md
  - /Users/autoparallel/Code/joy/docs/error-codes.md
  - https://github.com/harnesslabs/joy/issues/62
  - https://github.com/harnesslabs/joy/issues/70
  - https://github.com/harnesslabs/joy/issues/71
  - https://github.com/harnesslabs/joy/issues/72
  - https://github.com/harnesslabs/joy/issues/73
  - https://github.com/harnesslabs/joy/issues/74
---

# Phase 16: Workspace Support and Multiple Targets

## Why

Phase 14 left two major usability/scale gaps in `joy`: no workspace root model and no way to define multiple binary targets in a project. This phase adds both while preserving the existing single-project/single-target workflows and keeping machine JSON changes additive.

## What Shipped

- Manifest schema expansions in `/Users/autoparallel/Code/joy/src/manifest.rs`:
  - `WorkspaceManifest` / `ManifestDocument` (`[workspace] members`, optional `default_member`)
  - additive `[[project.targets]]` support for named binary targets
  - `Manifest::select_target(...)` helper for default/named target resolution
- Global workspace member selection flag in `/Users/autoparallel/Code/joy/src/cli.rs`:
  - `-p, --package <member>` (workspace root command routing)
- Workspace-aware command dispatch in `/Users/autoparallel/Code/joy/src/commands/mod.rs`:
  - routes project-scoped commands from a workspace root to the selected/default member
  - injects additive `workspace_root` / `workspace_member` JSON metadata for project-scoped commands
  - returns new workspace routing errors:
    - `workspace_member_required`
    - `workspace_member_not_found`
    - `workspace_member_invalid`
- Named target build/run support in `/Users/autoparallel/Code/joy/src/commands/build.rs` and `/Users/autoparallel/Code/joy/src/commands/run.rs`:
  - `joy build --target <name>`
  - `joy run --target <name>`
  - target-specific source/include handling
  - target-aware object hashing and non-default build.ninja naming
  - additive JSON fields: `target`, `target_default`
- Scaffold/docs updates:
  - target comments in generated `joy.toml` template
  - README workspace + multiple-target examples
  - machine-interface docs updated for workspace/target additive fields
  - error-code catalog updated for workspace routing errors

## Validation

- `just lint`
- `just test`

## Coverage Added

- `/Users/autoparallel/Code/joy/tests/add_command.rs`
  - workspace root requires `-p/--package` when no default member is set
  - workspace root routes `tree` to `default_member`
  - workspace root routes `add` to selected member and emits workspace JSON metadata
- `/Users/autoparallel/Code/joy/tests/build_run.rs`
  - workspace-root `build`/`run` with `-p` and `--target <name>` for a named target
- Existing payload shape tests updated additively for `workspace_root` / `workspace_member`

## Blockers & Solutions

- Blocker: A global `--package` workspace-member flag collided with command positional `package` args in `clap`, causing dependency names to be misinterpreted as workspace members.
- Solution: Kept the public flag name `-p/--package` but renamed the internal field to `workspace_package` to avoid clap arg-ID collisions.

- Blocker: Existing command handlers rely on `env::current_dir()` throughout the build/dependency pipeline.
- Solution: Added workspace-aware routing in command dispatch (`/Users/autoparallel/Code/joy/src/commands/mod.rs`) that temporarily changes cwd to the selected member root and restores it after handler execution.

- Blocker: Named targets can reuse source basenames and share the same `.joy/build/obj` directory.
- Solution: Folded the selected target name into object-name hashing and use a distinct `build-<target>.ninja` for non-default targets.

## Fallback Plan

If workspace routing proves too intrusive, keep the manifest/target schema and named-target build support, but temporarily restrict workspace-root execution to an explicit future command (`joy workspace exec -p ...`) while preserving the per-member `.joy` / `joy.lock` semantics already implemented. The dispatch routing layer in `/Users/autoparallel/Code/joy/src/commands/mod.rs` is the isolated rollback point.
