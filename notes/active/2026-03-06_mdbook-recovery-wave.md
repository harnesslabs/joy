---
date: 2026-03-06
commit: pending
tags:
  - docs
  - mdbook
  - reference
  - workflow
  - phase91
related_components:
  - /Users/autoparallel/Code/joy/book/src/
  - /Users/autoparallel/Code/joy/.codex/documentation-playbook.md
  - /Users/autoparallel/Code/joy/.codex/AGENTS.md
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/src/registry_config.rs
---

# mdBook Recovery Wave (Post-Phase 91)

## Why

The mdBook content had fallen behind the shipped CLI surface after Phase 38-76, Phase 77-86, and Phase 87-91 feature waves. We needed a full docs recovery pass with command parity, machine/error contract parity, and explicit planned/deferred labeling.

## Scope

1. Add a reusable documentation process playbook under `.codex/` and link it from `AGENTS.md`.
2. Expand mdBook information architecture for missing command families.
3. Document shipped workflows for source backends, registry discovery, offline/cache flows, and publish/owner/yank lifecycle.
4. Align reference docs with source for:
   - command map
   - manifest schema
   - lockfile schema
   - registry config behavior
   - machine envelope/payload matrix
   - stable error-code catalog
5. Refresh status/troubleshooting/introduction/getting-started chapters with current shipped boundaries.

## Validation Plan

- `just docs-lint`
- command coverage sweep against `joy --help` and grouped subcommand help
- payload/error parity spot checks against `src/commands/*` and tests

## Blockers & Solutions

- Blocker: error-code drift risk due mixed direct literals and mapped code paths.
  - Solution: derive docs from source handlers + mapping helpers (`dependency_common`, `graph_common`, toolchain mapping) rather than stale prior docs.
- Blocker: broad command surface made chapter boundaries fuzzy.
  - Solution: separate task workflows (`workflows/`) from stable interface detail (`reference/`) and keep command index as a dedicated reference page.

## Fallback Plan

If this wave causes docs instability in CI:

1. Keep summary/navigation expansion and critical machine/error docs.
2. Temporarily narrow secondary narrative content while preserving parity pages.
3. Re-run `just docs-lint` after each rollback iteration until stable.
