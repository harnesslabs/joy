# Packaging Metadata Templates

This directory contains packaging metadata templates for package managers.

Current templates:

- Homebrew formula: `/Users/autoparallel/Code/joy/packaging/homebrew/joy.rb`
- Scoop manifest: `/Users/autoparallel/Code/joy/packaging/scoop/joy.json`

The release workflow now generates release-specific metadata with exact checksums and uploads those files as release artifacts:

- `dist/packaging/homebrew/joy.rb`
- `dist/packaging/scoop/joy.json`

Repository copies are kept as syntax-valid templates. Generated release artifacts should be treated as the source of truth for publishing.

Per-release generated metadata includes:

- `version`
- GitHub Release asset URLs
- SHA256 hashes from the `.sha256` artifacts published by `/Users/autoparallel/Code/joy/.github/workflows/release.yaml`

Local syntax checks:

```bash
just dist-metadata-check
```
