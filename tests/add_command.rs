use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use tempfile::TempDir;
use which::which;

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
}

fn read_lockfile_toml(temp: &TempDir) -> toml::Value {
  let raw = fs::read_to_string(temp.path().join("joy.lock")).expect("read joy.lock");
  toml::from_str(&raw).expect("parse joy.lock")
}

fn write_lockfile_toml(temp: &TempDir, lock: &toml::Value) {
  let mut raw = toml::to_string_pretty(lock).expect("serialize joy.lock");
  if !raw.ends_with('\n') {
    raw.push('\n');
  }
  fs::write(temp.path().join("joy.lock"), raw).expect("write joy.lock");
}

fn init_project(temp: &TempDir) {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).arg("init").assert().success();
}

fn git_is_available() -> bool {
  ProcessCommand::new("git")
    .arg("--version")
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

fn setup_local_github_remote(package: &str) -> Option<(TempDir, PathBuf, String)> {
  if !git_is_available() {
    eprintln!("skipping test: git is not available");
    return None;
  }

  let mut parts = package.split('/');
  let owner = parts.next().expect("owner");
  let repo = parts.next().expect("repo");

  let remote_base = TempDir::new().expect("remote base");
  let work = TempDir::new().expect("work repo");
  let bare_repo = remote_base.path().join(owner).join(format!("{repo}.git"));
  fs::create_dir_all(bare_repo.parent().expect("bare parent")).expect("create bare parent");

  run_git(Some(work.path()), ["init"]).expect("git init");
  run_git(Some(work.path()), ["config", "user.email", "joy-tests@example.com"])
    .expect("git config email");
  run_git(Some(work.path()), ["config", "user.name", "Joy Tests"]).expect("git config name");
  fs::create_dir_all(work.path().join("include").join("nlohmann")).expect("header dir");
  fs::write(
    work.path().join("include").join("nlohmann").join("json.hpp"),
    "// header-only fixture\n",
  )
  .expect("write header");
  fs::write(work.path().join("README.md"), "# fixture\n").expect("write readme");
  run_git(Some(work.path()), ["add", "."]).expect("git add");
  run_git(Some(work.path()), ["commit", "-m", "fixture"]).expect("git commit");

  let commit = run_git_capture(Some(work.path()), ["rev-parse", "HEAD"]).expect("rev-parse");
  let commit = commit.trim().to_string();

  run_git_owned(
    None,
    vec!["init".into(), "--bare".into(), bare_repo.to_string_lossy().into_owned()],
  )
  .expect("git init --bare");
  run_git_owned(
    Some(work.path()),
    vec!["remote".into(), "add".into(), "origin".into(), bare_repo.to_string_lossy().into_owned()],
  )
  .expect("git remote add");
  run_git(Some(work.path()), ["push", "origin", "HEAD:refs/heads/main"]).expect("git push");
  run_git_owned(
    None,
    vec![
      "--git-dir".into(),
      bare_repo.to_string_lossy().into_owned(),
      "symbolic-ref".into(),
      "HEAD".into(),
      "refs/heads/main".into(),
    ],
  )
  .expect("set bare HEAD");

  Some((remote_base, bare_repo, commit))
}

fn setup_local_github_remote_fmt_fixture() -> Option<(TempDir, PathBuf, String)> {
  if !git_is_available() {
    eprintln!("skipping test: git is not available");
    return None;
  }

  let remote_base = TempDir::new().expect("remote base");
  let work = TempDir::new().expect("work repo");
  let bare_repo = remote_base.path().join("fmtlib").join("fmt.git");
  fs::create_dir_all(bare_repo.parent().expect("bare parent")).expect("create bare parent");

  run_git(Some(work.path()), ["init"]).expect("git init");
  run_git(Some(work.path()), ["config", "user.email", "joy-tests@example.com"])
    .expect("git config email");
  run_git(Some(work.path()), ["config", "user.name", "Joy Tests"]).expect("git config name");

  fs::create_dir_all(work.path().join("include").join("fmt")).expect("header dir");
  fs::write(
    work.path().join("include").join("fmt").join("core.h"),
    r#"#pragma once
const char* joy_fmt_message();
"#,
  )
  .expect("write header");
  fs::write(
    work.path().join("fmt.cpp"),
    r#"const char* joy_fmt_message() { return "hello-from-fmt-fixture"; }
"#,
  )
  .expect("write source");
  fs::write(
    work.path().join("CMakeLists.txt"),
    r#"cmake_minimum_required(VERSION 3.16)
project(fmt LANGUAGES CXX)
add_library(fmt STATIC fmt.cpp)
target_include_directories(fmt PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
"#,
  )
  .expect("write cmakelists");

  run_git(Some(work.path()), ["add", "."]).expect("git add");
  run_git(Some(work.path()), ["commit", "-m", "fixture"]).expect("git commit");
  let commit = run_git_capture(Some(work.path()), ["rev-parse", "HEAD"]).expect("rev-parse");
  let commit = commit.trim().to_string();

  run_git_owned(
    None,
    vec!["init".into(), "--bare".into(), bare_repo.to_string_lossy().into_owned()],
  )
  .expect("git init --bare");
  run_git_owned(
    Some(work.path()),
    vec!["remote".into(), "add".into(), "origin".into(), bare_repo.to_string_lossy().into_owned()],
  )
  .expect("git remote add");
  run_git(Some(work.path()), ["push", "origin", "HEAD:refs/heads/main"]).expect("git push");
  run_git_owned(
    None,
    vec![
      "--git-dir".into(),
      bare_repo.to_string_lossy().into_owned(),
      "symbolic-ref".into(),
      "HEAD".into(),
      "refs/heads/main".into(),
    ],
  )
  .expect("set bare HEAD");

  Some((remote_base, bare_repo, commit))
}

fn run_git<const N: usize>(cwd: Option<&Path>, args: [&str; N]) -> std::io::Result<()> {
  let mut cmd = ProcessCommand::new("git");
  if let Some(dir) = cwd {
    cmd.arg("-C").arg(dir);
  }
  cmd.args(args);
  let output = cmd.output()?;
  if output.status.success() {
    Ok(())
  } else {
    Err(std::io::Error::other(format!(
      "git failed: {}\nstdout: {}\nstderr: {}",
      output.status,
      String::from_utf8_lossy(&output.stdout),
      String::from_utf8_lossy(&output.stderr)
    )))
  }
}

fn run_git_owned(cwd: Option<&Path>, args: Vec<String>) -> std::io::Result<()> {
  let mut cmd = ProcessCommand::new("git");
  if let Some(dir) = cwd {
    cmd.arg("-C").arg(dir);
  }
  cmd.args(args);
  let output = cmd.output()?;
  if output.status.success() {
    Ok(())
  } else {
    Err(std::io::Error::other(format!(
      "git failed: {}\nstdout: {}\nstderr: {}",
      output.status,
      String::from_utf8_lossy(&output.stdout),
      String::from_utf8_lossy(&output.stderr)
    )))
  }
}

fn run_git_capture<const N: usize>(cwd: Option<&Path>, args: [&str; N]) -> std::io::Result<String> {
  let mut cmd = ProcessCommand::new("git");
  if let Some(dir) = cwd {
    cmd.arg("-C").arg(dir);
  }
  cmd.args(args);
  let output = cmd.output()?;
  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(std::io::Error::other(format!(
      "git failed: {}\nstdout: {}\nstderr: {}",
      output.status,
      String::from_utf8_lossy(&output.stdout),
      String::from_utf8_lossy(&output.stderr)
    )))
  }
}

#[test]
fn add_mutates_manifest_and_creates_local_env() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, expected_commit)) = setup_local_github_remote("nlohmann/json")
  else {
    return;
  };
  let joy_home = temp.path().join("joy-home");

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "add", "nlohmann/json"])
    .assert()
    .success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["data"]["resolved_commit"], expected_commit);
  assert_eq!(payload["data"]["cache_hit"], false);

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
  assert!(
    temp
      .path()
      .join(".joy")
      .join("include")
      .join("deps")
      .join("nlohmann_json")
      .join("nlohmann")
      .join("json.hpp")
      .is_file()
  );
  assert!(
    joy_home.join("cache").join("git").join("github").join("nlohmann").join("json.git").is_dir()
  );
  assert!(
    joy_home
      .join("cache")
      .join("src")
      .join("github")
      .join("nlohmann")
      .join("json")
      .join(expected_commit)
      .is_dir()
  );
}

#[cfg(unix)]
#[test]
fn add_rolls_back_installed_headers_when_manifest_write_fails() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, _expected_commit)) =
    setup_local_github_remote("nlohmann/json")
  else {
    return;
  };
  let joy_home = temp.path().join("joy-home");
  let manifest_path = temp.path().join("joy.toml");

  let mut perms = fs::metadata(&manifest_path).expect("manifest metadata").permissions();
  let original_mode = perms.mode();
  perms.set_mode(0o444);
  fs::set_permissions(&manifest_path, perms).expect("set readonly");

  let mut cmd = cargo_bin_cmd!("joy");
  cmd
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["add", "nlohmann/json"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("manifest_write_error"));

  let manifest = fs::read_to_string(&manifest_path).expect("manifest readable");
  assert!(
    !manifest.contains("nlohmann/json"),
    "manifest should not record dependency when add fails"
  );
  assert!(
    !temp.path().join(".joy/include/deps/nlohmann_json").exists(),
    "installed header path should be rolled back on manifest save failure"
  );

  let mut restore = fs::metadata(&manifest_path).expect("manifest metadata").permissions();
  restore.set_mode(original_mode);
  fs::set_permissions(&manifest_path, restore).expect("restore permissions");
}

#[test]
fn add_is_noop_when_dependency_already_matches() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, _commit)) = setup_local_github_remote("nlohmann/json") else {
    return;
  };
  let joy_home = temp.path().join("joy-home");

  let mut first = cargo_bin_cmd!("joy");
  first
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "add", "nlohmann/json"])
    .assert()
    .success();

  let mut second = cargo_bin_cmd!("joy");
  let assert = second
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "add", "nlohmann/json"])
    .assert()
    .success();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["data"]["changed"], false);
  assert_eq!(payload["data"]["cache_hit"], true);
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
  let assert = cmd.current_dir(temp.path()).args(["--json", "build"]).assert();

  let payload = json_stdout(&assert.get_output().stdout);
  if build_tools_available_for_test() {
    assert.success();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "build");
  } else {
    assert.failure();
    assert_eq!(payload["error"]["code"], "toolchain_not_found");
  }

  for dir in [".joy/include", ".joy/lib", ".joy/build", ".joy/bin", ".joy/state"] {
    assert!(temp.path().join(dir).is_dir(), "missing {dir}");
  }
}

fn build_tools_available_for_test() -> bool {
  (which("ninja").is_ok() || which("ninja-build").is_ok())
    && (which("clang++").is_ok()
      || which("g++").is_ok()
      || which("clang++.exe").is_ok()
      || which("g++.exe").is_ok())
}

fn compiled_build_tools_available_for_test() -> bool {
  build_tools_available_for_test() && which("cmake").is_ok()
}

#[test]
fn build_and_run_with_local_compiled_recipe_dependency() {
  if !compiled_build_tools_available_for_test() {
    eprintln!("skipping test: compiler/ninja/cmake not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, fmt_commit)) = setup_local_github_remote_fmt_fixture() else {
    return;
  };
  let joy_home = temp.path().join("joy-home");

  let mut add = cargo_bin_cmd!("joy");
  add
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "add", "fmtlib/fmt"])
    .assert()
    .success();

  fs::write(
    temp.path().join("src/main.cpp"),
    r#"#include <fmt/core.h>
#include <iostream>

int main() {
  std::cout << joy_fmt_message() << std::endl;
  return 0;
}
"#,
  )
  .expect("write main.cpp");

  let mut build = cargo_bin_cmd!("joy");
  let build_assert = build
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "build"])
    .assert()
    .success();
  let build_payload = json_stdout(&build_assert.get_output().stdout);
  assert_eq!(build_payload["ok"], true);
  assert!(
    build_payload["data"]["link_libs"]
      .as_array()
      .expect("link_libs")
      .iter()
      .any(|v| v.as_str() == Some("fmt"))
  );
  assert!(
    temp.path().join(".joy").join("lib").read_dir().expect("lib dir").next().is_some(),
    "expected staged compiled library artifacts in .joy/lib"
  );
  let lock = read_lockfile_toml(&temp);
  let packages = lock["packages"].as_array().expect("packages array");
  let fmt_pkg = packages
    .iter()
    .find(|pkg| pkg.get("id").and_then(|v| v.as_str()) == Some("fmtlib/fmt"))
    .expect("fmt package in lockfile");
  assert_eq!(fmt_pkg["source"].as_str(), Some("github"));
  assert_eq!(fmt_pkg["requested_rev"].as_str(), Some("HEAD"));
  assert_eq!(fmt_pkg["resolved_commit"].as_str(), Some(fmt_commit.as_str()));
  assert_eq!(fmt_pkg["recipe"].as_str(), Some("fmt"));
  assert_eq!(fmt_pkg["header_only"].as_bool(), Some(false));
  assert_eq!(fmt_pkg["linkage"].as_str(), Some("static"));
  assert!(
    fmt_pkg["abi_hash"]
      .as_str()
      .is_some_and(|s| s.len() == 64 && s.chars().all(|ch| ch.is_ascii_hexdigit()))
  );
  assert!(
    fmt_pkg["header_roots"]
      .as_array()
      .expect("header_roots array")
      .iter()
      .any(|v| v.as_str() == Some("include"))
  );
  assert!(
    fmt_pkg["libs"].as_array().expect("libs array").iter().any(|v| v.as_str() == Some("fmt"))
  );

  let mut run = cargo_bin_cmd!("joy");
  let run_assert = run
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "run"])
    .assert()
    .success();
  let run_payload = json_stdout(&run_assert.get_output().stdout);
  assert_eq!(run_payload["ok"], true);
  let stdout = run_payload["data"]["stdout"].as_str().expect("stdout string");
  assert_eq!(stdout.replace("\r\n", "\n"), "hello-from-fmt-fixture\n");
}

#[test]
fn build_populates_lockfile_package_records_for_header_only_dependency() {
  if !build_tools_available_for_test() {
    eprintln!("skipping test: compiler/ninja not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, expected_commit)) = setup_local_github_remote("nlohmann/json")
  else {
    return;
  };
  let joy_home = temp.path().join("joy-home");

  let mut add = cargo_bin_cmd!("joy");
  add
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["add", "nlohmann/json"])
    .assert()
    .success();

  let mut build = cargo_bin_cmd!("joy");
  build
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["build"])
    .assert()
    .success();

  let lock = read_lockfile_toml(&temp);
  let packages = lock["packages"].as_array().expect("packages array");
  assert_eq!(packages.len(), 1);

  let pkg = &packages[0];
  assert_eq!(pkg["id"].as_str(), Some("nlohmann/json"));
  assert_eq!(pkg["source"].as_str(), Some("github"));
  assert_eq!(pkg["requested_rev"].as_str(), Some("HEAD"));
  assert_eq!(pkg["resolved_commit"].as_str(), Some(expected_commit.as_str()));
  assert_eq!(pkg["recipe"].as_str(), Some("nlohmann_json"));
  assert_eq!(pkg["header_only"].as_bool(), Some(true));
  assert!(
    pkg["header_roots"]
      .as_array()
      .expect("header_roots array")
      .iter()
      .any(|v| v.as_str() == Some("include"))
  );
  assert_eq!(pkg["deps"].as_array().map(Vec::len), Some(0));
  assert_eq!(pkg["libs"].as_array().map(Vec::len), Some(0));
  assert_eq!(pkg["abi_hash"].as_str(), Some(""));
}

#[test]
fn build_locked_rejects_incomplete_and_mismatched_lockfile_package_metadata() {
  if !build_tools_available_for_test() {
    eprintln!("skipping test: compiler/ninja not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, _expected_commit)) =
    setup_local_github_remote("nlohmann/json")
  else {
    return;
  };
  let joy_home = temp.path().join("joy-home");

  let mut add = cargo_bin_cmd!("joy");
  add
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["add", "nlohmann/json"])
    .assert()
    .success();

  let mut build = cargo_bin_cmd!("joy");
  build
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["build"])
    .assert()
    .success();

  let mut incomplete = read_lockfile_toml(&temp);
  incomplete["packages"] = toml::Value::Array(Vec::new());
  write_lockfile_toml(&temp, &incomplete);

  let mut locked_incomplete = cargo_bin_cmd!("joy");
  let incomplete_assert = locked_incomplete
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "build", "--locked"])
    .assert()
    .failure();
  let incomplete_payload = json_stdout(&incomplete_assert.get_output().stdout);
  assert_eq!(incomplete_payload["error"]["code"], "lockfile_incomplete");
  assert!(
    incomplete_payload["error"]["message"]
      .as_str()
      .is_some_and(|msg| msg.contains("--update-lock") && msg.contains("joy build --update-lock"))
  );

  let mut refresh = cargo_bin_cmd!("joy");
  refresh
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["build", "--update-lock"])
    .assert()
    .success();

  let mut mismatch = read_lockfile_toml(&temp);
  let packages = mismatch["packages"].as_array_mut().expect("packages array");
  let pkg = packages
    .iter_mut()
    .find(|pkg| pkg.get("id").and_then(|v| v.as_str()) == Some("nlohmann/json"))
    .expect("nlohmann package");
  pkg["resolved_commit"] = toml::Value::String("deadbeef".to_string());
  write_lockfile_toml(&temp, &mismatch);

  let mut locked_mismatch = cargo_bin_cmd!("joy");
  let mismatch_assert = locked_mismatch
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "build", "--locked"])
    .assert()
    .failure();
  let mismatch_payload = json_stdout(&mismatch_assert.get_output().stdout);
  assert_eq!(mismatch_payload["error"]["code"], "lockfile_mismatch");
  assert!(
    mismatch_payload["error"]["message"]
      .as_str()
      .is_some_and(|msg| msg.contains("--update-lock") && msg.contains("joy build --update-lock"))
  );
}
