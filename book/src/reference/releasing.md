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

- Release PR + tag automation: `.github/workflows/release-plz.yaml`
- Binary/release asset workflow: `.github/workflows/release.yaml`
- Packaging config for release-plz: `release-plz.toml`

`release-plz` handles version/changelog PRs and tag creation.
Tag pushes (`v*`) trigger the binary release workflow.

## Pre-Release Checklist

1. Ensure milestone issues and notes are up to date.
2. Confirm local environment health (`joy --json doctor`).
3. Run CI-parity commands (`just ci-local`; optionally compiled e2e checks).
4. Validate distribution metadata templates (`just dist-metadata-check`).
5. Confirm PR checks (`gh pr checks <pr-number>`).
6. Ensure `RELEASE_PLZ_TOKEN` secret is configured with `contents` + `pull_requests` write permission.
7. If default-branch protection blocks workflow pushes, set `JOY_RELEASE_PAT` so release metadata commits can update `Formula/joy.rb`.

## Automated Flow (Recommended)

1. Merge regular changes into `main`.
2. `release-plz` opens/updates a release PR with version and changelog updates.
3. Merge the release PR.
4. `release-plz release` creates `vX.Y.Z` tag.
5. `.github/workflows/release.yaml` builds artifacts, publishes GitHub Release assets, and updates package metadata.

Monitor Actions and verify the GitHub Release contains archives/checksums/signatures/SBOM.

## Manual Backstop (If Needed)

```bash
git tag -a vX.Y.Z -m "joy vX.Y.Z"
git push origin vX.Y.Z
```

## Packaging Outputs

The release workflow generates and publishes concrete metadata with release checksums:

- `Formula/joy.rb` (single-repo Homebrew tap source of truth)
- `packaging/scoop/joy.json`

Users can install via Homebrew tap:

```bash
brew tap harnesslabs/joy
brew install harnesslabs/joy/joy
brew upgrade joy
```
