# Registry Index Dependencies

Status: `Shipped` (initial git-backed registry index mode)

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

## Current Limitations (Important)

- Registry package aliases (registry name different from canonical source package ID) are intentionally deferred.
- The initial registry cut is git-backed and local-mirror oriented; broader registry protocols are future work.
