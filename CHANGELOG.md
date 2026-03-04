# Changelog

All notable changes to `joy` will be documented in this file.

The format is based on Keep a Changelog and this project adheres to Semantic Versioning (`0.y.z` while pre-1.0).

## [Unreleased]

### Added

- Production-readiness baseline hardening:
- CI gating updates (required/optional split, docs checks, semver advisory baseline)
- JSON machine envelope metadata (`schema_version`, `joy_version`)
- `joy version` command
- `joy outdated --sources` source filtering
- GitHub tag-based update checks in `joy outdated`
- Release artifact smoke tests and generated packaging metadata assets
- Governance baseline docs (`LICENSE`, `SECURITY.md`, `CODE_OF_CONDUCT.md`)

### Changed

- mdBook configuration updated for modern `mdbook` compatibility
- Default registry URL fallback now points to the public default registry when `JOY_REGISTRY_DEFAULT` is unset

## [0.1.0] - 2026-02-25

### Added

- Initial public pre-1.0 feature wave for scaffolding, dependency management, reproducible build workflows, and machine-readable output.
