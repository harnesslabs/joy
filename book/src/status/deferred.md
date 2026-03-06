# Roadmap / Deferred Features

Status entries below are intentionally explicit so users know what is not ready yet.

Snapshot date: **March 6, 2026**.

## Planned (Near-Term, Not Fully Closed)

- Additional offline CI matrix hardening for cold/warm/vendored permutations and deterministic error-path coverage.
- Further hardening of self-hosted publishing auth/token policy and transport reliability for non-local registry deployments.
- Continued source/backend parity hardening in resolver/fetch edge-case paths beyond current shipped baseline.

## Deferred (Known, Expected Future Work)

- Broader registry protocols beyond current git-backed index mode.
- Registry alias package support (registry name different from canonical source package id).
- Deeper package feature/variant support and additional non-binary target kinds.
- Full-screen TUI mode (current roadmap remains focused on line-oriented CLI and machine JSON interfaces).
- Editor extension productization remains gated behind objective CLI/compile-db criteria; CLI-first remains default.

## Documentation Notes

Roadmap intent may appear in docs where relevant. Planned/deferred items are never implied as shipped behavior.
