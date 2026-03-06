# Dependency Source Backends

`joy` supports five direct dependency source backends.

## Quick Matrix

- `github`: canonical package id (`owner/repo`) with `rev` or `version`
- `registry`: package from configured index, requires `version`
- `git`: explicit git URL/path and explicit `rev`
- `path`: local path source
- `archive`: URL + required SHA-256 checksum

## GitHub Source (`source = "github"`)

```bash
joy add nlohmann/json
joy add fmtlib/fmt --rev 11.0.2
joy add fmtlib/fmt --version ^11
joy update fmtlib/fmt --rev 11.1.0
```

## Registry Source (`source = "registry"`)

```bash
joy add registry:nlohmann/json --version ^3
joy add registry:nlohmann/json --registry internal --version ^3
joy update registry:nlohmann/json --version ^3
```

Registry direct dependencies require `--version`.

## Git Source (`source = "git"`)

```bash
joy add git:https://github.com/fmtlib/fmt.git --as fmtlib/fmt --rev 11.0.2
joy update fmtlib/fmt --rev 11.1.0
```

Git source entries require `--rev` and do not use `--version`.

## Path Source (`source = "path"`)

```bash
joy add path:vendor/localdep --as localdep
joy remove localdep
```

Path dependencies are useful for local integration and monorepo-style source reuse.

## Archive Source (`source = "archive"`)

```bash
joy add archive:https://example.com/libfoo.tar.gz --as acme/libfoo --sha256 <sha256>
joy update acme/libfoo --sha256 <new-sha256>
```

Archive source requires both URL and SHA-256.

## Source Filters in `outdated`

`joy outdated --sources` supports:

- `all`
- `registry`
- `github`
- `git`
- `path`
- `archive`

Example:

```bash
joy outdated --sources git
```

## Provenance in `joy.lock`

Lock entries include backend-specific provenance fields used by `verify`, `vendor`, and deterministic refresh flows.

See [Lockfile Schema](../reference/lockfile-schema.md).
