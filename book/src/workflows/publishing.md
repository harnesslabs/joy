# Package Publishing and Ownership

`joy` supports self-hosted package publishing and ownership workflows backed by registry index mutation.

## Initialize a Reusable Package Manifest

```bash
joy package init acme/widgets --kind header-only --version 0.1.0
```

Kinds:

- `header-only` (default)
- `cmake`

## Publish

```bash
joy publish --registry internal --rev v0.1.0
```

If `--source-package` is omitted, package id from the local package manifest is used.

## Manage Owners

```bash
joy owner list acme/widgets --registry internal
joy owner add acme/widgets alice --registry internal
joy owner remove acme/widgets alice --registry internal
```

## Yank / Unyank Releases

```bash
joy yank acme/widgets --version 0.1.0 --registry internal
joy yank acme/widgets --version 0.1.0 --undo --registry internal
```

Yanked versions are excluded from normal registry resolution.

## Remote Registry Transport (Shipped)

`publish`, `owner`, and `yank` support remote git-backed registry URLs (for example `file://...` or hosted git remotes) via temporary checkout/commit/push flows.

Transport/auth failures are returned with stable error codes in JSON mode.

## Recommended End-to-End Flow

```bash
joy registry add internal file:///srv/joy-index.git --default
joy package init acme/widgets --kind header-only --version 0.1.0
joy publish --registry internal --rev v0.1.0
joy owner add acme/widgets alice --registry internal
joy yank acme/widgets --version 0.1.0 --registry internal
joy yank acme/widgets --version 0.1.0 --undo --registry internal
```
