default:
  @just --list

build:
  cargo build --workspace

test:
  cargo test --workspace

lint:
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- --deny warnings

ci: lint test

clean:
  rm -rf target .joy
