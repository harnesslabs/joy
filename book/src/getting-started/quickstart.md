# Quickstart

## 60-Second Flow

```bash
joy new hello_cpp
cd hello_cpp
joy add nlohmann/json
joy run
joy tree
```

What happens:

1. `joy new` scaffolds `joy.toml`, `src/main.cpp`, and `.gitignore`.
2. `joy add` records the dependency and materializes headers into `.joy/`.
3. `joy run` builds the project (and dependencies when needed) and executes the binary.
4. `joy tree` shows the resolved dependency graph.

## What Gets Created

Project-local state is kept under `.joy/` (created lazily) and a lockfile is kept in `joy.lock` for reproducibility.

Typical project layout:

```text
hello_cpp/
  joy.toml
  joy.lock          # created when sync/build/run resolves lock state
  src/
    main.cpp
  .joy/
    build/
    bin/
    include/
    lib/
```

## Next Steps

- Learn the core command set in [Core Commands](../workflows/core-commands.md)
- Use [Reproducible Workflows](../workflows/reproducible.md) for CI and offline runs
- Read [Workspaces and Targets](../workflows/workspaces-targets.md) for multi-project repos
