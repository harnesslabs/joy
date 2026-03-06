# Offline Fetch, Vendor, and Cache

These commands support deterministic dependency preparation and cache lifecycle control.

## Warm Cache Without Building

```bash
joy fetch
```

`fetch` resolves current manifest dependencies and pre-populates source caches.

## Vendor Lockfile Dependencies

```bash
joy vendor
joy vendor --output third_party/vendor
```

`vendor` copies lockfile-resolved sources into a project-local directory, grouped by package slug and resolved commit.

## Cache Garbage Collection

```bash
joy cache gc
joy cache gc --aggressive
```

- default `gc`: clears temporary cache paths
- `--aggressive`: also clears source/archive cache roots

## Offline + Frozen Workflow

```bash
joy fetch
joy --frozen sync
joy --frozen build
```

For lock/source integrity checks:

```bash
joy verify
joy verify --strict --sbom sbom.json
```

## Common Pattern for CI

```bash
joy --json doctor
joy fetch
joy --frozen sync
joy --frozen build
joy --json verify --strict --sbom sbom.json
```
