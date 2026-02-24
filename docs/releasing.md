# Releasing `joy`

This document defines the release process and versioning policy for `joy` binaries, GitHub releases, and milestone notes.

## Versioning Policy

- `joy` uses semantic versioning with pre-1.0 expectations (`0.y.z`): minor versions may include additive CLI functionality and behavior tightening; patch versions should focus on bug fixes and documentation.
- `/Users/autoparallel/Code/joy/Cargo.toml` is the source of truth for the version.
- Git tags for releases use the `vX.Y.Z` format (for example `v0.4.0`).
- Release artifact names include version and target triple: `joy-vX.Y.Z-<target>.<ext>`.
- JSON/CLI compatibility during the current roadmap wave remains additive-only for existing commands and envelopes.

## Current Release Targets

The release workflow currently builds and publishes:

- `x86_64-unknown-linux-gnu` (`.tar.gz`)
- `aarch64-apple-darwin` (`.tar.gz`)
- `x86_64-pc-windows-gnu` (`.zip`)

Notes:

- Windows MSVC local/CI build support is available (Phase 10), but the release workflow currently publishes the Windows GNU artifact for the packaged Windows channel.
- Homebrew/Scoop metadata templates are kept in `/Users/autoparallel/Code/joy/packaging/` and updated per release with new version + SHA256 values.

## Release Workflow (GitHub Actions)

- Workflow file: `/Users/autoparallel/Code/joy/.github/workflows/release.yaml`
- Triggered on `v*` tag pushes.
- Also supports manual `workflow_dispatch`, but it must be run on a tag ref (select the tag in the Actions UI).
- Publishes artifacts and checksum files to the GitHub Release for the tag.

## Pre-Release Checklist

Run from `/Users/autoparallel/Code/joy`:

1. Ensure the milestone issues and notes are up to date.
   - update `/Users/autoparallel/Code/joy/notes/roadmap.md`
   - add/archive milestone notes under `/Users/autoparallel/Code/joy/notes/`
2. Confirm local environment health.
   - `cargo run -- --json doctor`
3. Run validation with CI parity commands.
   - `just ci-local`
   - `just compiled-e2e` (recommended before tagged releases)
4. Validate distribution metadata syntax.
   - `just dist-metadata-check`
5. Confirm PR status before merging release prep.
   - `gh pr checks <pr-number>`
6. Bump version in `/Users/autoparallel/Code/joy/Cargo.toml` (and `Cargo.lock` if changed by cargo commands).
7. Refresh release-facing docs if needed.
   - `/Users/autoparallel/Code/joy/README.md`
   - `/Users/autoparallel/Code/joy/CONTRIBUTING.md`
   - `/Users/autoparallel/Code/joy/packaging/homebrew/joy.rb`
   - `/Users/autoparallel/Code/joy/packaging/scoop/joy.json`

## Tagging and Publishing

1. Merge release prep to `main`.
2. Create and push an annotated tag.

```bash
git tag -a vX.Y.Z -m "joy vX.Y.Z"
git push origin vX.Y.Z
```

3. Monitor the release workflow.

```bash
gh run list --workflow Release
gh run watch <run-id>
```

4. Verify the GitHub Release contains all expected artifacts:
   - `joy-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
   - `joy-vX.Y.Z-aarch64-apple-darwin.tar.gz`
   - `joy-vX.Y.Z-x86_64-pc-windows-gnu.zip`
   - matching `.sha256` files
5. Update/publish Homebrew tap and Scoop bucket manifests with the new SHA256 values.
6. Close the release-related roadmap issues and update the milestone tracker.

## Notes / Changelog Expectations

`joy` currently uses milestone notes + GitHub release notes rather than a committed `CHANGELOG.md`.

For each release:

- summarize shipped phases/issues in a note under `/Users/autoparallel/Code/joy/notes/active/` (or archive milestone note if closing a phase wave)
- link the relevant GitHub milestone/tracker issues
- ensure release notes align with the implemented CLI flags/commands and platform caveats
