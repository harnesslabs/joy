# Registry Configuration

`joy` merges registry config from user scope and optional project scope.

Source of truth: `src/registry_config.rs` and `src/commands/registry_cmd.rs`.

## Scopes

- user scope (default): `$JOY_HOME/config/registries.toml`
- project scope (`--project`): `<project>/.joy/registries.toml`

Effective config merge behavior:

1. user config loaded first
2. project config overlays user entries
3. project default overrides user default

## File Format

```toml
version = 1
default = "internal"

[registries.internal]
index = "file:///srv/joy-index.git"

[registries.default]
index = "https://github.com/harnesslabs/joy-registry.git"
```

Fields:

- `version` (must be `1`)
- `default` (optional registry name)
- `registries.<name>.index` (URL/path)

## CLI Management

```bash
joy registry list
joy registry add internal file:///srv/joy-index.git --default
joy registry remove internal
joy registry set-default internal

joy registry list --project
joy registry add internal file:///srv/joy-index.git --project --default
```

## Validation Rules

- registry name must be non-empty and `[A-Za-z0-9_.-]` only
- registry index path/URL must be non-empty
- unsupported config versions fail with `registry_index_unsupported_version` in publishing/index flows or parse/config errors in config loading paths

## Related Commands

- discovery: `joy search`, `joy info`
- dependency resolution: `joy add registry:<id> --version <range>`
- publish workflows: `joy publish`, `joy owner`, `joy yank`
