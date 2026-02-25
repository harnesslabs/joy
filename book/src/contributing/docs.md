# Contributing Docs

This chapter is for contributors touching user-facing documentation.

## Source of Truth

- `README.md` is the project overview and entrypoint.
- `book/` is the in-depth user and reference documentation.
- `docs/` contains legacy entrypoints/stubs that should point to the mdBook.

## Writing Guidelines

- Prefer task-oriented explanations first, then reference detail.
- Mark incomplete/deferred features explicitly.
- Keep command examples copy/paste ready.
- When human output examples are likely to change, avoid over-specifying exact formatting unless the formatting itself is the subject.

## Validation

Use local docs commands (defined in `justfile`) before opening a PR:

- `just docs-build`
- `just docs-lint`
