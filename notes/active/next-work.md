# Next Work (Phase 38-76 Core Graph + Nested Deps + Editor Wave)

Immediate follow-up queue:

1. Implement `joy metadata`/`joy tree`/`joy why` provenance polish for `recipe` vs `package_manifest` vs `registry summary` and decide stable JSON additions (#134, #145)
2. Add `joy outdated` (direct + transitive, registry-backed where available) with stable JSON/human output and tests (#143)
3. Expand `joy doctor` to report compile DB/editor artifact state, toolchain discovery skips, and nested package metadata support/mismatch diagnostics (#144)
4. Add docs/troubleshooting updates for clangd/VSCode compile DB usage and nested dependency/package-manifest behavior (#142)
5. Run `just ci-pr` + targeted bug-bash sweep for nested deps/offline/editor flows; file follow-up issues and close tracker gaps (#146)
