# Lockfile Schema

`joy.lock` is a TOML lockfile used for reproducibility and source provenance.

Source of truth: `src/lockfile.rs` and lockfile assembly logic in `src/commands/build.rs`.

## Top-Level Fields

```toml
version = 1
manifest_hash = "<sha256>"
generated_by = "joy <version>"

[[packages]]
...
```

- `version`: lockfile schema version (currently `1`)
- `manifest_hash`: hash used for stale-lock detection
- `generated_by`: generator version string
- `packages`: resolved package entries

## Package Entry Fields

Each `[[packages]]` entry includes:

- identity and resolution:
  - `id`
  - `source`
  - `requested_rev`
  - `requested_requirement` (optional)
  - `resolved_version` (optional)
  - `resolved_commit`
  - `resolved_ref` (optional)
- source provenance:
  - `registry` (optional)
  - `source_package` (optional)
  - `source_git` (optional)
  - `source_path` (optional)
  - `source_url` (optional)
  - `source_checksum_sha256` (optional, required for archive source)
- dependency/build metadata:
  - `header_only`
  - `header_roots[]`
  - `deps[]`
  - `recipe` (optional)
  - `metadata_source` (optional)
  - `package_manifest_digest` (optional)
  - `declared_deps_source` (optional)
  - `abi_hash`
  - `libs[]`
  - `linkage` (optional)

## Source Provenance Expectations

- `github`: no extra provenance field required
- `registry`: should include `registry`
- `git`: should include `source_git`
- `path`: should include `source_path`
- `archive`: should include `source_url` and `source_checksum_sha256`

`joy verify` enforces source-specific provenance checks and checksum policy.

## Lockfile Lifecycle

- `sync/build/run` may write or refresh lockfile based on lock mode flags.
- `--locked` rejects drift.
- `--update-lock` forces refresh.
- `--frozen` implies locked/offline behavior.

Workspace-routed commands use workspace lockfile semantics when invoked from workspace root with selected member routing.
