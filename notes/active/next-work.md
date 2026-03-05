# Next Work (Phase 77-86 Active Wave)

Immediate queue for the Cargo/uv-style usability wave:

1. Complete resolver/fetch provider abstraction for non-github sources (`git`, `path`, `archive`) and graduate from staged-only manifest updates to real sync/build flows.
2. Expand offline CI matrix for warm-cache and vendored-only builds with deterministic failure codes.
3. Finish Phase 85 protocol hardening:
   - auth token handling/policy for publish workflows
   - registry transport expansion beyond local-path indexes.
4. Phase 86 usability closeout:
   - polish `doctor`/`metadata` troubleshooting
   - define objective editor-extension gate criteria and default defer behavior.
