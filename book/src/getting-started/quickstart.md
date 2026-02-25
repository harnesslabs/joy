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
2. `joy add` records the dependency, runs a sync-lite refresh, and materializes headers into `.joy/`.
3. `joy run` builds the project (and dependencies when needed), refreshes dependency/editor artifacts, and executes the binary.
4. `joy tree` shows the resolved dependency graph.

`joy sync`, `joy build`, and `joy run` also refresh `compile_commands.json` at the project root (best effort) so clangd / VSCode can resolve dependency includes.

## What Gets Created

Project-local state is kept under `.joy/` (created lazily) and a lockfile is kept in `joy.lock` for reproducibility.

Typical project layout:

```text
hello_cpp/
  joy.toml
  joy.lock          # created when sync/build/run resolves lock state
  compile_commands.json  # auto-generated when toolchain discovery succeeds
  src/
    main.cpp
  .joy/
    build/
      compile_commands.<target>.json
    bin/
    include/
    lib/
    state/
      dependency-graph.json
```

## Next Steps

- Learn the core command set in [Core Commands](../workflows/core-commands.md)
- If your editor still cannot resolve includes, read [Troubleshooting](../troubleshooting/common-failures.md) (clangd / VSCode `compile_commands.json` setup)
- Use [Reproducible Workflows](../workflows/reproducible.md) for CI and offline runs
- Read [Workspaces and Targets](../workflows/workspaces-targets.md) for multi-project repos
