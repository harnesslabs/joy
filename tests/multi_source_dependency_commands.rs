use assert_cmd::cargo::cargo_bin_cmd;
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

fn read_manifest_toml(temp: &TempDir) -> toml::Value {
  let raw = fs::read_to_string(temp.path().join("joy.toml")).expect("read joy.toml");
  toml::from_str(&raw).expect("parse joy.toml")
}

#[test]
fn add_git_dependency_records_manifest_with_alias_and_rev() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut add = cargo_bin_cmd!("joy");
  let add_assert = add
    .current_dir(temp.path())
    .args([
      "--json",
      "add",
      "git+https://example.com/acme/mylib.git",
      "--as",
      "mylib",
      "--rev",
      "abc123",
    ])
    .assert()
    .success();

  let payload = json_stdout(&add_assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["data"]["key"], "mylib");
  assert_eq!(payload["data"]["source"], "git");
  assert_eq!(payload["data"]["rev"], "abc123");

  let manifest = read_manifest_toml(&temp);
  assert_eq!(manifest["dependencies"]["mylib"]["source"].as_str(), Some("git"));
  assert_eq!(
    manifest["dependencies"]["mylib"]["git"].as_str(),
    Some("https://example.com/acme/mylib.git")
  );
  assert_eq!(manifest["dependencies"]["mylib"]["rev"].as_str(), Some("abc123"));
}

#[test]
fn add_archive_dependency_requires_sha256() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut add = cargo_bin_cmd!("joy");
  let add_assert = add
    .current_dir(temp.path())
    .args(["--json", "add", "archive:https://example.com/lib.tar.gz", "--as", "archive_dep"])
    .assert()
    .failure();
  let payload = json_stdout(&add_assert.get_output().stdout);
  assert_eq!(payload["ok"], false);
  assert_eq!(payload["error"]["code"], "invalid_add_args");
}

#[test]
fn update_archive_dependency_updates_sha256_without_fetching() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut add = cargo_bin_cmd!("joy");
  add
    .current_dir(temp.path())
    .args([
      "add",
      "archive:https://example.com/lib.tar.gz",
      "--as",
      "archive_dep",
      "--sha256",
      "1111",
    ])
    .assert()
    .success();

  let mut update = cargo_bin_cmd!("joy");
  let update_assert = update
    .current_dir(temp.path())
    .args(["--json", "update", "archive_dep", "--sha256", "2222"])
    .assert()
    .success();
  let payload = json_stdout(&update_assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["updated_count"], 1);
  assert_eq!(payload["data"]["updated"][0]["source"], "archive");
  assert_eq!(payload["data"]["updated"][0]["staged_only"], true);

  let manifest = read_manifest_toml(&temp);
  assert_eq!(manifest["dependencies"]["archive_dep"]["sha256"].as_str(), Some("2222"));
}

#[test]
fn remove_path_dependency_cleans_manifest_entry() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut add = cargo_bin_cmd!("joy");
  add
    .current_dir(temp.path())
    .args(["add", "path:../vendor/localdep", "--as", "localdep"])
    .assert()
    .success();

  let mut remove = cargo_bin_cmd!("joy");
  let remove_assert =
    remove.current_dir(temp.path()).args(["--json", "remove", "localdep"]).assert().success();
  let payload = json_stdout(&remove_assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["source"], "path");
  assert_eq!(payload["data"]["removed"], true);
  assert_eq!(payload["data"]["key"], "localdep");

  let manifest = read_manifest_toml(&temp);
  let deps = manifest["dependencies"].as_table().expect("dependencies table");
  assert!(!deps.contains_key("localdep"));
}
