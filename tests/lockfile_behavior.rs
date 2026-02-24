use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;
use which::which;

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
}

fn init_project(temp: &TempDir) {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).arg("init").assert().success();
}

fn build_tools_available_for_test() -> bool {
  (which("ninja").is_ok() || which("ninja-build").is_ok())
    && (which("clang++").is_ok()
      || which("g++").is_ok()
      || which("clang++.exe").is_ok()
      || which("g++.exe").is_ok())
}

#[test]
fn build_locked_requires_existing_lockfile() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert =
    cmd.current_dir(temp.path()).args(["--json", "build", "--locked"]).assert().failure();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "build");
  assert_eq!(payload["error"]["code"], "lockfile_missing");
}

#[test]
fn build_creates_lockfile_and_update_lock_refreshes_stale_manifest_hash() {
  if !build_tools_available_for_test() {
    eprintln!("skipping lockfile build test: compiler/ninja not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut first = cargo_bin_cmd!("joy");
  let first_assert = first.current_dir(temp.path()).args(["--json", "build"]).assert().success();
  let first_payload = json_stdout(&first_assert.get_output().stdout);
  assert_eq!(first_payload["data"]["lockfile_updated"], true);

  let lock_path = temp.path().join("joy.lock");
  assert!(lock_path.is_file(), "joy.lock should be created");
  let first_lock = fs::read_to_string(&lock_path).expect("read joy.lock");
  let first_lock_toml: toml::Value = toml::from_str(&first_lock).expect("parse lock");
  let first_hash = first_lock_toml["manifest_hash"].as_str().expect("manifest_hash").to_string();

  fs::write(
    temp.path().join("joy.toml"),
    fs::read_to_string(temp.path().join("joy.toml")).expect("manifest") + "\n# drift\n",
  )
  .expect("mutate manifest");

  let mut stale = cargo_bin_cmd!("joy");
  let stale_assert = stale.current_dir(temp.path()).args(["--json", "build"]).assert().failure();
  let stale_payload = json_stdout(&stale_assert.get_output().stdout);
  assert_eq!(stale_payload["error"]["code"], "lockfile_stale");

  let mut refresh = cargo_bin_cmd!("joy");
  let refresh_assert =
    refresh.current_dir(temp.path()).args(["--json", "build", "--update-lock"]).assert().success();
  let refresh_payload = json_stdout(&refresh_assert.get_output().stdout);
  assert_eq!(refresh_payload["data"]["lockfile_updated"], true);

  let refreshed_lock = fs::read_to_string(&lock_path).expect("read refreshed lock");
  let refreshed_lock_toml: toml::Value =
    toml::from_str(&refreshed_lock).expect("parse refreshed lock");
  let refreshed_hash =
    refreshed_lock_toml["manifest_hash"].as_str().expect("refreshed manifest_hash");
  assert_ne!(refreshed_hash, first_hash);
}

#[test]
fn build_locked_accepts_matching_lockfile_when_present() {
  if !build_tools_available_for_test() {
    eprintln!("skipping lockfile locked test: compiler/ninja not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut first = cargo_bin_cmd!("joy");
  first.current_dir(temp.path()).args(["build"]).assert().success();

  let mut locked = cargo_bin_cmd!("joy");
  let assert =
    locked.current_dir(temp.path()).args(["--json", "build", "--locked"]).assert().success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["lockfile_updated"], false);
}
