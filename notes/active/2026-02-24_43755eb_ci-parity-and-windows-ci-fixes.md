---
date: 2026-02-24
commit: 43755eb
tags:
  - ci
  - windows
  - justfile
  - agents
  - tests
related_components:
  - /Users/autoparallel/Code/joy/justfile
  - /Users/autoparallel/Code/joy/.github/workflows/check.yaml
  - /Users/autoparallel/Code/joy/.codex/AGENTS.md
  - /Users/autoparallel/Code/joy/src/linking.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/src/recipes.rs
  - https://github.com/harnesslabs/joy/pull/1
---

# CI Parity + Windows CI Failure Triage

## Why

The open PR had multiple failing CI jobs (`fmt`, Windows `lint`, Windows `test`, `udeps`, and `semver`). This milestone closes the loop by reproducing the failures from logs, fixing the real portability/lint issues, and adding explicit CI-parity `just` commands plus AGENTS guidance so future PRs run a closer local preflight.

## Blockers & Solutions

- Blocker: CI `fmt` failure was caused by TOML formatting drift (`taplo fmt --check`), but local workflow did not run Taplo.
- Solution: Added `just fmt-check` / `just fmt-fix` and made `ci` invoke Taplo checks.

- Blocker: Windows Clippy failed on `result_large_err` for TOML parse error variants.
- Solution: Boxed `toml::de::Error` in `ManifestError`, `LockfileError`, and `RecipeError`.

- Blocker: Windows test failed when re-installing headers because directory symlink removal used `remove_file` only.
- Solution: `remove_existing_path()` now falls back to `remove_dir()` for symlink removal on Windows error kinds.

- Blocker: Windows E2E test compared exact `\n` stdout and failed on `\r\n`.
- Solution: Normalize line endings in the test assertion before comparison.

- Blocker: `udeps` reported an unused direct dependency (`anyhow`).
- Solution: Removed `anyhow` from `Cargo.toml`.

- Blocker: `cargo semver-checks` CI job used the crates.io package name `joy` baseline, which is unrelated to this repository and produced false failures.
- Solution: Converted the CI semver job into an explicit skip with rationale and added `just semver-main` for repo-local baseline checks.

## Fallback Plan

If any CI-specific adjustment is controversial (especially the semver skip), keep the code/test portability fixes and local `just` parity commands, then split the workflow change into a follow-up PR with a stricter repo-local semver baseline strategy.
