# Dependencies

`joy` supports direct dependencies from multiple source backends and uses lockfile provenance to keep resolution deterministic.

## Supported Direct Source Backends

- `github`
- `registry`
- `git`
- `path`
- `archive`

See [Dependency Source Backends](../workflows/dependency-sources.md) for user workflows and [Manifest Schema](../reference/manifest-schema.md) for field-level contract details.

## Source-Aware Update Reporting

`joy outdated --sources` supports:

- `all` (default)
- `registry`
- `github`
- `git`
- `path`
- `archive`

## Provenance + Integrity

`joy.lock` records source provenance fields used by:

- `joy verify`
- `joy vendor`
- offline/frozen dependency reuse

See [Lockfile Schema](../reference/lockfile-schema.md).
