default:
  @just --list

build:
  cargo build --workspace

test:
  cargo test --workspace

lint:
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- --deny warnings

recipe-check:
  cargo run --quiet -- recipe-check --json

docs-build:
  mdbook build book

docs-serve:
  mdbook serve book

docs-lint:
  mdbook build book
  mdbook test book

dist-metadata-check:
  ruby -c Formula/joy.rb
  jq empty packaging/scoop/joy.json

fmt-check:
  cargo fmt --all -- --check
  taplo fmt --check

fmt-fix:
  cargo fmt --all
  taplo fmt

clippy-target target:
  cargo clippy --target {{target}} --all-targets --all-features -- --deny warnings

test-target target:
  cargo test --verbose --target {{target}} --workspace

build-msvc:
  cargo build --target x86_64-pc-windows-msvc --workspace

lint-msvc:
  cargo clippy --target x86_64-pc-windows-msvc --all-targets --all-features -- --deny warnings

test-msvc:
  cargo test --verbose --target x86_64-pc-windows-msvc --workspace

ci-msvc:
  just build-msvc
  just lint-msvc
  just test-msvc

compiled-e2e:
  cargo test --verbose --workspace --test add_command build_and_run_with_local_compiled_recipe_dependency -- --nocapture
  cargo test --verbose --workspace --test lockfile_behavior -- --nocapture

udeps:
  cargo +nightly udeps --workspace

semver:
  cargo semver-checks check-release --workspace --baseline-rev origin/main

semver-main:
  cargo semver-checks check-release --workspace --baseline-rev origin/main

semver-cratesio:
  cargo semver-checks check-release --workspace

ci:
  just fmt-check
  just lint
  just recipe-check
  just test

ci-local:
  just ci
  just compiled-e2e

ci-pr:
  just ci-local
  just ci-docs

ci-docs:
  just docs-lint

clean:
  rm -rf target .joy
