# Reproducible Workflows

`joy` includes explicit commands and flags for repeatable local and CI builds.

## `joy sync`

`joy sync` resolves dependencies, refreshes local materialized state, and updates lockfile state without compiling the final project binary.

```bash
joy sync
```

Use this when you want to prepare dependency/cache state before a later `build` or `run`.

## `--locked`

Rejects lockfile updates during the command.

```bash
joy build --locked
```

Use this in CI to ensure no uncommitted dependency/manifest drift is being introduced.

## `--offline`

Disables network access and uses only cached dependency data.

```bash
joy --offline build
joy --offline run
```

If cache entries are missing, `joy` fails with stable machine-readable error codes (for example `offline_cache_miss`).

## `--frozen`

`--frozen` is the strict CI-friendly mode:

- no network access
- no lockfile changes

It implies `--offline` and locked semantics.

```bash
joy --frozen build
joy --frozen sync
```

## Typical CI Pattern

```bash
joy --json doctor
joy --frozen sync
joy --frozen build
```

For machine consumers, use `--json` and key off `error.code` instead of free-form human text.
