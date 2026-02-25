---
date: 2026-02-25
commit: pending
tags:
  - docs
  - github-pages
  - mdbook
  - ci
related_components:
  - /Users/autoparallel/Code/joy/.github/workflows/docs.yaml
  - /Users/autoparallel/Code/joy/book/book.toml
---

# GitHub Pages 404 Fix: Deploy `book/dist/html` Instead of `book/dist`

## Why

`joy.harnesslabs.dev` returned the generic GitHub Pages 404 page even though the `Docs` workflow and `deploy-pages` job were succeeding on `main`.

The deploy workflow was uploading `book/dist`, but mdBook (with both `html` and `linkcheck` renderers enabled) writes the actual site under `book/dist/html` and linkcheck output under `book/dist/linkcheck`.

That meant the deployed artifact root had no `index.html`, which causes the exact GitHub Pages 404 shown by the user.

## What Changed

- Updated `/Users/autoparallel/Code/joy/.github/workflows/docs.yaml`:
  - write `CNAME` to `book/dist/html/CNAME`
  - upload Pages artifact from `book/dist/html` (the actual site root)

## Evidence

From the successful `deploy-pages` job logs (run `22413559129`), the uploaded artifact contents included:

- `./html/index.html`
- `./linkcheck/cache.json`
- `./CNAME`

and not a root `./index.html`.

## Fallback Plan

If the site still serves 404 after this fix deploys:

1. Check Pages deployment logs for the new run and verify `./index.html` appears at artifact root.
2. Confirm the custom domain setting still points to `joy.harnesslabs.dev`.
3. Temporarily disable the custom domain to validate that the Pages site content is being served on the default Pages URL.
