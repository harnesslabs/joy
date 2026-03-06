# Registry Discovery and Configuration

`joy` supports user-level and project-level registry configuration.

## Registry Configuration Commands

List effective registries:

```bash
joy registry list
joy registry list --project
```

Add/remove registries:

```bash
joy registry add internal file:///path/to/internal-index.git
joy registry add internal file:///path/to/internal-index.git --default
joy registry remove internal
joy registry set-default internal
```

Use `--project` to write project-local config under `.joy/`.

## Search and Package Info

Search package IDs:

```bash
joy search json
joy search json --registry internal --limit 50
```

Inspect package versions:

```bash
joy info nlohmann/json
joy info nlohmann/json --registry internal
```

## Typical Setup Flow

```bash
joy registry add internal file:///srv/joy-index.git --default
joy search fmt --registry internal
joy info fmtlib/fmt --registry internal
joy add registry:fmtlib/fmt --registry internal --version ^11
```

## Troubleshooting

If registry commands fail:

- validate registry name and index path/URL
- confirm `joy registry list` shows expected default
- retry with `--json` and inspect `error.code`
- check project-vs-user scope mismatch (`--project` vs default user scope)

Reference details: [Registry Configuration](../reference/registry-config.md).
