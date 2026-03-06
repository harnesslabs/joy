# Next Work (Post Phase 77-86 Implementation Wave)

Follow-up queue after delivering Phase 81-86 implementation milestones:

1. Complete universal provider backend support in resolver/fetch for non-github sources (`git`, `path`, `archive`) beyond staged-manifest recording.
2. Expand offline CI coverage for cold/warm/vendored matrices and enforce deterministic error-code expectations.
3. Harden publishing protocol for production self-hosted usage:
   - registry auth token handling/policy
   - non-local registry transport support for publish/owner/yank workflows.
4. Close remaining phase-77-80 parity gaps:
   - named-registry selection in core resolver path
   - full source-provenance parity for all backend types.
