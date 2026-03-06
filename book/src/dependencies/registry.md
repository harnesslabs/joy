# Registry Index Dependencies

Status: `Shipped` (git-backed registry index mode with named registries and publish/owner/yank workflows)

`joy` can resolve direct dependencies from configured registry indexes while fetching source content from package source coordinates.

## Configure Registries

List effective configuration:

```bash
joy registry list
joy registry list --project
```

Add/set default registry:

```bash
joy registry add internal file:///path/to/internal-index.git --default
joy registry set-default internal
```

See [Registry Discovery and Configuration](../workflows/registry-discovery.md).

## Add Registry Dependencies

Use `registry:` package notation with a semver requirement.

```bash
joy add registry:nlohmann/json --version ^3
joy add registry:nlohmann/json --registry internal --version ^3
```

Registry direct dependencies require `--version <range>`.

## Stored Metadata

In `joy.toml`, registry dependencies are stored with `source = "registry"` plus version range and optional named registry.

In `joy.lock`, selected concrete versions/commits and source provenance fields are recorded for deterministic refresh and verification.

## Registry Index Versions

`joy` supports:

- registry index `version = 1` (release -> source mapping)
- registry index `version = 2` (optional embedded package metadata summaries)

Registry v2 summaries can include kind/include roots/dependencies plus digest for mismatch detection.

## Publish/Owner/Yank Integration

Registry commands are part of the self-hosted publish loop:

- `joy publish`
- `joy owner list|add|remove`
- `joy yank [--undo]`

Remote git-backed registry URLs are supported in current shipped behavior.

## Current Limitations (Important)

- Registry alias package support (registry name different from canonical source package id) remains deferred.
- Broader non-git registry transport models are deferred.
