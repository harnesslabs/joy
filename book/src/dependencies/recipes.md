# Recipes (Compiled Dependencies)

`joy` ships a curated recipe index in `recipes/` for dependencies that require build/install metadata.

Examples include:

- `fmtlib/fmt`
- `madler/zlib`
- `gabime/spdlog`
- `Neargye/magic_enum`
- `jarro2783/cxxopts`

## How Recipes Fit the Build Pipeline

When a dependency is recipe-backed and compiled:

1. `joy` resolves dependency graph + recipe metadata.
2. It computes ABI-sensitive cache keys/hashes.
3. It builds dependency artifacts with CMake + Ninja in global cache.
4. It installs headers/libs into project-local `.joy/` state.

## Generic CMake Fallback (Shipped)

If a dependency has no recipe but contains a top-level `CMakeLists.txt`, `joy` can run a generic CMake-first compiled fallback path.

This keeps more compiled packages usable without waiting for curated recipe additions.

## Validate Recipe Metadata

```bash
joy recipe-check
joy --json recipe-check
```

Use this in local development and CI whenever recipe metadata changes.
