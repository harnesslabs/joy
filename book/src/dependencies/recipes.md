# Recipes (Compiled Dependencies)

`joy` ships a curated recipe index in `recipes/` for dependencies that require build/install steps.

Examples include:

- `fmtlib/fmt`
- `madler/zlib`
- `gabime/spdlog`
- `Neargye/magic_enum`
- `jarro2783/cxxopts`

## How Recipes Fit the Build Pipeline

When a dependency is recipe-backed and compiled:

1. `joy` resolves the dependency graph and recipe metadata.
2. It computes ABI-sensitive cache keys/hashes.
3. It builds the dependency with CMake + Ninja in the global cache.
4. It installs headers/libs into the project-local `.joy/` environment.

## Validate Recipe Metadata

```bash
joy recipe-check
joy --json recipe-check
```

Use this locally and in CI to validate bundled recipe metadata changes.
