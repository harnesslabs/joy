# Registry Index Dependencies

Status: `Shipped` (git-backed registry index mode, with optional v2 embedded metadata summaries)

`joy` can resolve direct dependencies from a configured registry index while still fetching package source code from GitHub.

## Configure the Default Registry

Set `JOY_REGISTRY_DEFAULT` to a git URL or a local git repo path for the registry index.

```bash
export JOY_REGISTRY_DEFAULT=/path/to/joy-registry.git
```

## Add a Registry Dependency

Use the `registry:` prefix with a semver range.

```bash
joy add registry:nlohmann/json --version ^3
```

Registry direct dependencies currently require `--version <range>`.

## What Gets Stored

In `joy.toml`, registry dependencies are stored with `source = "registry"` and a requested semver range.

The selected concrete version/tag/commit is recorded in `joy.lock`.

`joy.lock` also records package metadata provenance for resolved packages (for example `recipe`, `package_manifest`, or `registry_manifest`) when available.

## Registry Index Versions

`joy` supports:

- registry index `version = 1` (release -> source mapping only)
- registry index `version = 2` (optional embedded package metadata summary per release)

Registry v2 embedded summaries can include:

- package kind / exported include roots
- declared nested dependencies
- manifest digest (for verification against fetched package metadata)

When a fetched package checkout does not contain a package `joy.toml`, `joy` can fall back to a registry v2 embedded metadata summary to expand nested dependencies.

If both exist and the digests disagree, resolution fails with `package_metadata_mismatch`.

## Current Limitations (Important)

- Registry package aliases (registry name different from canonical source package ID) are intentionally deferred.
- Registry checks are git-backed and local-mirror oriented; broader registry protocols are future work.
- `joy outdated` currently performs version checks only for registry-backed packages.
