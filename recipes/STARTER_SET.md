# Recipe Starter Expansion Set

This document defines the Phase 12 starter expansion set and the acceptance rubric used to add new curated recipes.

## Goals

- Improve out-of-the-box usefulness with commonly used C++ libraries.
- Prefer deterministic metadata and cross-platform-safe defaults over maximal feature coverage.
- Keep recipes small, explicit, and testable.

## Phase 12 Starter Set (Added / Maintained)

Compiled / recipe-backed:

- `fmtlib/fmt`
- `madler/zlib`

Header-only:

- `nlohmann/json`
- `cliutils/CLI11`
- `Neargye/magic_enum`
- `skypjack/entt`
- `jarro2783/cxxopts`
- `TartanLlama/expected`
- `gabime/spdlog` (header-only usage path; declares transitive `fmtlib/fmt`)
- `pybind/pybind11`
- `ericniebler/range-v3`
- `doctest/doctest`
- `microsoft/GSL`
- `Neargye/nameof`
- `foonathan/type_safe`
- `martinmoene/span-lite`
- `martinmoene/optional-lite`
- `martinmoene/string-view-lite`
- `Tessil/robin-map`
- `martinus/nanobench`

## Acceptance Rubric

A recipe should generally meet all of the following before inclusion:

1. The package is widely used or meaningfully improves onboarding for common C++ workflows.
2. The package can be represented with deterministic metadata (headers, deps, build targets, link libs).
3. The recipe favors portable defaults (no platform-specific behavior unless required and documented).
4. Header roots and transitive deps are explicit.
5. Compiled recipes declare `cmake` and `link` metadata sufficient for cacheable builds.
6. The recipe is covered by at least one deterministic local test path (schema/load validation, smoke matrix, or fixture-based build flow).

## Notes

- Recipes intentionally model the subset of package behavior that `joy` can build reproducibly today.
- Optional features, platform-specific switches, and advanced variants should be added only when there is a clear testable use case.
