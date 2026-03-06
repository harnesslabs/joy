---
date: 2026-03-06
commit: pending
tags:
  - phase91
  - publishing
  - registry
  - git-transport
---

# Phase 91 Remote Registry Transport for Publish/Owner/Yank

## Why
`joy publish`, `joy owner`, and `joy yank` previously rejected configured remote registry URLs, which blocked real self-hosted deployment workflows.

## What changed
- Replaced transport rejection in `/Users/autoparallel/Code/joy/src/commands/publishing.rs` with remote git checkout support.
- Added clone/commit/push workflow for remote registries while preserving local index behavior.
- Added deterministic transport/auth error mapping (`registry_transport_failed`, `registry_auth_failed`) and remote URL redaction in errors.
- Added integration coverage in `/Users/autoparallel/Code/joy/tests/publish_command.rs` for remote file-backed git registry roundtrip.

## Blockers and solutions
- **Blocker:** existing code assumed local filesystem registry paths and committed directly in-place.
  - **Solution:** introduced per-command temporary checkouts under cache temp space and pushed commits to `origin` for remote URLs.
- **Blocker:** raw git failures could leak credential-bearing URLs in error surfaces.
  - **Solution:** redaction helper strips embedded credentials from transport error messages.

## Fallback plan
If remote transport introduces instability:
1. Revert this commit and restore local-only publish behavior.
2. Keep local index workflows active while hardening remote checkout lifecycle separately.
3. Re-enable the old guardrail error for remote URLs until transport reliability is re-established.
