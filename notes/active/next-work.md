# Next Work (Phase 77-86 Active Wave)

Immediate queue for the Cargo/uv-style usability wave:

1. Complete resolver/fetch provider abstraction for non-github sources (`git`, `path`, `archive`) and graduate from staged-only manifest updates to real sync/build flows.
2. Implement generic recipe-less CMake fallback pipeline (Phase 81) with cross-platform fixtures for GNU + MSVC.
3. Tighten lock provenance and supply-chain policy:
   - source provenance for each backend
   - strict checksum validation for vendored/archive inputs
   - `joy verify` + baseline SBOM output (Phase 84)
4. Expand offline CI matrix for warm-cache and vendored-only builds with deterministic failure codes.
5. Start Phase 85 protocol work:
   - self-hosted registry publish flow
   - auth token handling
   - owner/yank lifecycle commands.
