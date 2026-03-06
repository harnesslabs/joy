# Reproducible Workflows

`joy` provides lockfile and offline controls intended for repeatable CI and local rebuilds.

## `joy sync`

`joy sync` resolves dependencies, materializes headers/libs, refreshes graph artifacts, and updates `joy.lock` when needed without compiling the final binary.

```bash
joy sync
```

## `--locked`

Reject lockfile updates for commands that consume dependency state.

```bash
joy build --locked
joy sync --locked
joy run --locked
joy why nlohmann/json --locked
joy tree --locked
```

Use `--locked` in CI to reject uncommitted dependency or manifest drift.

## `--update-lock`

Force lockfile refresh when intentional dependency/manifest changes are present.

```bash
joy sync --update-lock
joy build --update-lock
joy run --update-lock
```

`--locked` and `--update-lock` are mutually exclusive.

## `--offline`

Disable network access and use only cached dependency data.

```bash
joy --offline sync
joy --offline build
joy --offline run
joy --offline outdated
```

If cache is cold for required sources, commands fail with stable machine-readable error codes.

## `--frozen`

`--frozen` is strict CI mode:

- implies offline behavior
- disallows lockfile changes

```bash
joy --frozen sync
joy --frozen build
joy --frozen run
```

## Cache Warmup and Vendoring

Warm cache before strict offline runs:

```bash
joy fetch
joy sync
```

Vendor resolved lockfile sources for controlled source snapshots:

```bash
joy vendor
joy vendor --output third_party/vendor
```

## Verify and SBOM in CI

Use `joy verify` to check provenance and produce SBOM data.

```bash
joy --json verify --strict --sbom sbom.json
```

This validates lock provenance fields by source backend and emits a baseline `joy-sbom-v1` component list.

## Typical CI Pattern

```bash
joy --json doctor
joy fetch
joy --frozen sync
joy --frozen build
joy --json verify --strict --sbom sbom.json
```

Automation should key off `error.code` and documented payload fields, not human text.
