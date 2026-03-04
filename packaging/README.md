# Packaging Metadata Templates

This directory contains packaging metadata templates for package managers.

Current metadata files:

- Homebrew formula (tap source of truth): `/Users/autoparallel/Code/joy/Formula/joy.rb`
- Scoop manifest template: `/Users/autoparallel/Code/joy/packaging/scoop/joy.json`

The release workflow generates release-specific metadata with exact checksums and uploads those files as release artifacts:

- `dist/Formula/joy.rb`
- `dist/packaging/scoop/joy.json`

For Homebrew single-repo tap mode, the workflow also writes generated metadata back to `Formula/joy.rb` on the default branch after each tag release (using `GITHUB_TOKEN` or optional `JOY_RELEASE_PAT`).

Repository copies are syntax-valid bootstraps between releases.

Per-release generated metadata includes:

- `version`
- GitHub Release asset URLs
- SHA256 hashes from the `.sha256` artifacts published by `/Users/autoparallel/Code/joy/.github/workflows/release.yaml`

Local syntax checks:

```bash
just dist-metadata-check
```
