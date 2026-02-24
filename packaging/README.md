# Packaging Metadata Templates

This directory contains release-channel metadata templates for package managers.

Current templates:

- Homebrew formula: `/Users/autoparallel/Code/joy/packaging/homebrew/joy.rb`
- Scoop manifest: `/Users/autoparallel/Code/joy/packaging/scoop/joy.json`

These files are intended to be copied into a Homebrew tap and Scoop bucket (or used as the source for those repos), then updated per release with:

- `version`
- GitHub Release asset URLs
- SHA256 hashes from the `.sha256` artifacts published by `/Users/autoparallel/Code/joy/.github/workflows/release.yaml`

Local syntax checks:

```bash
just dist-metadata-check
```
