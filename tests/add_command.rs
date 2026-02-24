use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
}

fn init_project(temp: &TempDir) {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).arg("init").assert().success();
}

#[test]
fn add_mutates_manifest_and_creates_local_env() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).args(["add", "nlohmann/json"]).assert().success();

  let manifest = fs::read_to_string(temp.path().join("joy.toml")).expect("manifest");
  let project_name = temp.path().file_name().and_then(|name| name.to_str()).expect("tempdir name");
  assert_eq!(
    manifest,
    format!(
      r#"[project]
name = "{project_name}"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"

[dependencies."nlohmann/json"]
source = "github"
rev = "HEAD"
"#
    )
  );

  for dir in [".joy/include", ".joy/lib", ".joy/build", ".joy/bin", ".joy/state"] {
    assert!(temp.path().join(dir).is_dir(), "missing {dir}");
  }
}

#[test]
fn add_is_noop_when_dependency_already_matches() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut first = cargo_bin_cmd!("joy");
  first.current_dir(temp.path()).args(["--json", "add", "nlohmann/json"]).assert().success();

  let mut second = cargo_bin_cmd!("joy");
  let assert =
    second.current_dir(temp.path()).args(["--json", "add", "nlohmann/json"]).assert().success();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["data"]["changed"], false);
}

#[test]
fn add_rejects_invalid_package_id() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.current_dir(temp.path()).args(["--json", "add", "invalid"]).assert().failure();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["ok"], false);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["error"]["code"], "invalid_package_id");
}

#[test]
fn add_fails_without_manifest() {
  let temp = TempDir::new().expect("tempdir");

  let mut cmd = cargo_bin_cmd!("joy");
  cmd
    .current_dir(temp.path())
    .args(["add", "nlohmann/json"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("manifest_not_found"));
}

#[test]
fn build_stub_creates_local_env_when_manifest_exists() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.current_dir(temp.path()).args(["--json", "build"]).assert().code(2);

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["error"]["code"], "not_implemented");

  for dir in [".joy/include", ".joy/lib", ".joy/build", ".joy/bin", ".joy/state"] {
    assert!(temp.path().join(dir).is_dir(), "missing {dir}");
  }
}
