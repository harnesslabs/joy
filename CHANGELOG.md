# Changelog

All notable changes to `joy` will be documented in this file.

The format is based on Keep a Changelog and this project adheres to Semantic Versioning (`0.y.z` while pre-1.0).

## [Unreleased]

## [0.2.0](https://github.com/harnesslabs/joy/compare/v0.1.2...v0.2.0) - 2026-03-06

### Added

- phase87-92 source parity and remote registry publish transport ([#181](https://github.com/harnesslabs/joy/pull/181))
- phase77-86 cargo/uv-grade usability wave ([#179](https://github.com/harnesslabs/joy/pull/179))

### Other

- *(book)* recover mdbook parity and add playbook ([#182](https://github.com/harnesslabs/joy/pull/182))

## [0.1.2](https://github.com/harnesslabs/joy/compare/v0.1.1...v0.1.2) - 2026-03-05

### Fixed

- *(release)* avoid same-file copy in github-release job ([#155](https://github.com/harnesslabs/joy/pull/155))

### Other

- deletion-first simplification pass ([#156](https://github.com/harnesslabs/joy/pull/156))

## [0.1.1](https://github.com/harnesslabs/joy/compare/v0.1.0...v0.1.1) - 2026-03-04

### Fixed

- *(release)* unblock sbom and scoop metadata generation ([#153](https://github.com/harnesslabs/joy/pull/153))

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
