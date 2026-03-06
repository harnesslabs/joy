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
2. `joy add` records dependency intent and runs sync-lite refresh by default.
3. `joy run` builds the project and executes the binary.
4. `joy tree` shows resolved dependency graph state.

## Reproducible Follow-Up Flow

After the first run, validate deterministic state:

```bash
joy fetch
joy --frozen sync
joy --frozen build
joy --json verify --strict --sbom sbom.json
```

## What Gets Created

Project-local state is kept under `.joy/` and lock state in `joy.lock`.

Typical layout:

```text
hello_cpp/
  joy.toml
  joy.lock
  compile_commands.json
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

- [Core Commands](../workflows/core-commands.md)
- [Dependency Source Backends](../workflows/dependency-sources.md)
- [Registry Discovery and Configuration](../workflows/registry-discovery.md)
- [Offline Fetch, Vendor, and Cache](../workflows/offline-vendor-cache.md)
- [Troubleshooting](../troubleshooting/common-failures.md)
