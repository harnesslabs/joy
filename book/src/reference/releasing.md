# Releasing `joy`

This chapter documents the release process for `joy` binaries, GitHub Releases, and release-adjacent notes.

## Versioning Policy

- `joy` uses semantic versioning with pre-1.0 expectations (`0.y.z`).
- `Cargo.toml` is the source of truth for the version.
- Git tags use `vX.Y.Z` (for example `v0.4.0`).

## Current Release Targets

The release workflow currently publishes:

- `x86_64-unknown-linux-gnu`
- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `x86_64-pc-windows-gnu` (compatibility artifact during the beta transition)

## Release Workflow

- Workflow file: `.github/workflows/release.yaml`
- Trigger: tag push matching `v*`
- Also supports manual dispatch (run on a tag ref)

## Pre-Release Checklist

1. Ensure milestone issues and notes are up to date.
2. Confirm local environment health (`joy --json doctor`).
3. Run CI-parity commands (`just ci-local`; optionally compiled e2e checks).
4. Validate distribution metadata templates (`just dist-metadata-check`).
5. Confirm PR checks (`gh pr checks <pr-number>`).
6. Bump version in `Cargo.toml`.
7. Refresh release-facing docs (`README`, packaging metadata, docs).

## Tag and Publish

```bash
git tag -a vX.Y.Z -m "joy vX.Y.Z"
git push origin vX.Y.Z
```

Then monitor the GitHub Actions workflow and verify the GitHub Release contains the expected archives and checksums.

The release workflow also publishes generated packaging metadata with concrete checksums:

- `packaging/homebrew/joy.rb`
- `packaging/scoop/joy.json`

If channel publishing variables/secrets are configured, the workflow can also push updates directly to Homebrew tap and Scoop bucket repositories:

- `JOY_HOMEBREW_TAP_REPO` (repo slug)
- `JOY_SCOOP_BUCKET_REPO` (repo slug)
- `JOY_PACKAGING_PAT` (token secret with repo write access)
