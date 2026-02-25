---
date: 2026-02-24
commit: pending
tags:
  - phase-12
  - recipes
  - ci
  - validation
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/recipe_check.rs
  - /Users/autoparallel/Code/joy/src/recipes.rs
  - /Users/autoparallel/Code/joy/recipes/index.toml
  - /Users/autoparallel/Code/joy/recipes/STARTER_SET.md
  - /Users/autoparallel/Code/joy/tests/recipe_smoke.rs
  - /Users/autoparallel/Code/joy/tests/smoke.rs
  - /Users/autoparallel/Code/joy/justfile
  - /Users/autoparallel/Code/joy/.github/workflows/check.yaml
  - https://github.com/harnesslabs/joy/issues/47
  - https://github.com/harnesslabs/joy/issues/48
  - https://github.com/harnesslabs/joy/issues/49
  - https://github.com/harnesslabs/joy/issues/50
---

# Phase 12: Recipe Ecosystem Expansion + Validation Gate

## What Shipped

- Added `joy recipe-check` for validating bundled recipe metadata in local development and CI.
- Added duplicate recipe index ID/slug validation in `RecipeStore::load_from_dir(...)`.
- Added a `recipe-check` justfile target and a dedicated recipe validation job in `.github/workflows/check.yaml`.
- Added `recipes/STARTER_SET.md` documenting the starter expansion set and acceptance rubric.
- Expanded the curated recipe index from 3 to 9 packages (6 new header-first recipes).

## Recipe Set Expansion

New recipes added:

- `cliutils/CLI11`
- `Neargye/magic_enum`
- `skypjack/entt`
- `jarro2783/cxxopts`
- `TartanLlama/expected`
- `gabime/spdlog` (header-only usage path with transitive `fmtlib/fmt`)

## Coverage Added

- `tests/smoke.rs`: `recipe-check` JSON smoke test
- `tests/recipe_smoke.rs`:
  - default recipe store representative header-only + compiled smoke matrix
  - deterministic local transitive chain smoke using a synthetic recipe fixture
- `src/recipes.rs` unit coverage for duplicate index ID/slug rejection

## Notes

This phase intentionally emphasizes deterministic metadata quality and validation gates over deep package-feature coverage. Compiled recipe breadth can continue expanding incrementally now that `recipe-check` provides a CI guardrail.
