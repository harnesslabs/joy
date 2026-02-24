# Contributing to joy

## Scope

`joy` is a Rust CLI for C++ project scaffolding, dependency fetching, and local builds. The project is currently optimized for rapid iteration with strong tests and milestone notes.

## Local Prerequisites

- Rust toolchain
- `just`
- `git`
- For build/E2E coverage: `ninja`, `cmake`, and a C++ compiler (`clang++` or `g++` / MinGW on Windows)

## Development Commands

```bash
just build
just lint
just test
just ci
```

## Notes / Long-Term Memory

Development history and milestone notes live in `/Users/autoparallel/Code/joy/notes`.

- active work notes go under `/Users/autoparallel/Code/joy/notes/active`
- completed milestones are archived under `/Users/autoparallel/Code/joy/notes/archive`
- preserve YAML frontmatter (`date`, `commit`, `tags`, `related_components`) for searchability

## PR Expectations

- keep changes coherent and scoped
- add tests for behavior changes
- run `just lint` and `just test` before code-changing commits
- document non-obvious design choices in notes for future contributors
