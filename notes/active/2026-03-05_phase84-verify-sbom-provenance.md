---
date: 2026-03-05
commit: a6d8661
tags:
  - phase84
  - security
  - verify
  - sbom
  - supply-chain
related_components:
  - /Users/autoparallel/Code/joy/src/commands/verify.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/output.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
---

# Phase 84: Supply Chain Hardening Baseline (`joy verify`)

## Why

Phase 77-83 improved dependency resolution and lock provenance, but there was no first-class command to validate source integrity or produce a bill of materials. Teams still needed ad-hoc checks for tampering and provenance drift.

## What Landed

1. New `joy verify` command (CLI + dispatch):
   - checks lockfile package provenance shape by source backend
   - verifies checksum mismatches when `source_checksum_sha256` is pinned
   - compares vendored source checksum against fetched source/checksum when vendor snapshots exist
2. Strict policy mode:
   - `joy verify --strict` fails when lock entries do not pin checksum coverage
3. Baseline SBOM generation:
   - `joy verify` emits inline SBOM JSON (`data.sbom`)
   - optional `--sbom <path>` writes SBOM to disk
4. Human error guidance:
   - added `verify_failed` human hint in output renderer

## Validation

- `just lint` (pass)
- `just test` (pass)
- integration coverage in `/Users/autoparallel/Code/joy/tests/add_command.rs`:
  - `verify_emits_sbom_and_reports_non_failing_advisories`
  - `verify_fails_on_lockfile_checksum_mismatch`

## Blockers & Solutions

- Blocker: lockfiles in current wave often lack explicit checksum pins for github/registry entries, which would make strict verification noisy if always-on.
- Solution: default mode treats missing checksum as advisory, while `--strict` upgrades it to a hard failure for policy-enforced CI.

## Fallback Plan

If verification proves too strict for current lockfile coverage:

1. keep provenance validation and SBOM output enabled.
2. limit checksum enforcement to `--strict` CI paths until checksum pin coverage is expanded.
3. retain the same command contract so policy can be tightened later without CLI churn.
