---
date: 2026-02-24
commit: pending
tags:
  - phase-8
  - fetch
  - retry
  - network
  - determinism
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - https://github.com/harnesslabs/joy/issues/30
---

# Fetch Retry/Backoff for Transient Network Errors

## Why

Phase 8 required retry/backoff behavior for transient fetch failures with deterministic error reporting. The prefetch pipeline had an explicit TODO for this, and transient mirror clone/fetch failures could still fail the command immediately even when a short retry would succeed.

## What Changed

- Added deterministic retry/backoff helpers in `src/fetch.rs` with a fixed retry budget (`3` attempts) and fixed delays.
- Applied retries at the fetch backend layer (mirror clone/fetch and archive download), so both resolver-triggered fetches and prefetch flows benefit without changing higher-level APIs.
- Added a stable terminal error variant when retries are exhausted:
  - `TransientRetriesExhausted { action, attempts, source }`
- Added transient/non-transient classification heuristics for `FetchError` (HTTP, git spawn I/O, and common git stderr network failure text).
- Removed the retry/backoff TODO from `prefetch_github_packages(...)` since the behavior now lives in the lower-level fetch primitives.

## Coverage

Added unit tests in `src/fetch.rs` for:
- retrying transient errors and eventually succeeding
- not retrying non-transient errors
- stable retry-exhaustion error shaping after 3 attempts

## Notes

The retry logic is intentionally deterministic (fixed attempt count + fixed delays, no jitter) so failure messages stay stable for machine consumers and snapshot-style tests. If runtime behavior later needs jitter, preserve deterministic error envelopes and avoid embedding timing details in user-visible error strings.
