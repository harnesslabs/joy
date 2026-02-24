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

fn json_object_keys(value: &Value) -> Vec<String> {
  let mut keys =
    value.as_object().expect("json object").keys().map(ToString::to_string).collect::<Vec<_>>();
  keys.sort();
  keys
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

fn init_project_at(path: &Path) {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(path).arg("init").assert().success();
}

fn append_manifest_dependency(temp: &TempDir, package: &str, rev: &str) {
  let manifest_path = temp.path().join("joy.toml");
  let mut manifest = fs::read_to_string(&manifest_path).expect("read joy.toml");
  manifest.push_str(&format!("\"{package}\" = {{ source = \"github\", rev = \"{rev}\" }}\n"));
  fs::write(&manifest_path, manifest).expect("write joy.toml");
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

fn setup_local_github_remote_two_commits(
  package: &str,
) -> Option<(TempDir, PathBuf, String, String)> {
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
    "// header-only fixture v1\n",
  )
  .expect("write header v1");
  run_git(Some(work.path()), ["add", "."]).expect("git add v1");
  run_git(Some(work.path()), ["commit", "-m", "fixture v1"]).expect("git commit v1");
  let commit1 = run_git_capture(Some(work.path()), ["rev-parse", "HEAD"]).expect("rev-parse v1");
  let commit1 = commit1.trim().to_string();

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
  run_git(Some(work.path()), ["push", "origin", "HEAD:refs/heads/main"]).expect("git push v1");

  fs::write(
    work.path().join("include").join("nlohmann").join("json.hpp"),
    "// header-only fixture v2\n",
  )
  .expect("write header v2");
  fs::write(work.path().join("CHANGELOG.md"), "v2\n").expect("write changelog v2");
  run_git(Some(work.path()), ["add", "."]).expect("git add v2");
  run_git(Some(work.path()), ["commit", "-m", "fixture v2"]).expect("git commit v2");
  let commit2 = run_git_capture(Some(work.path()), ["rev-parse", "HEAD"]).expect("rev-parse v2");
  let commit2 = commit2.trim().to_string();
  run_git(Some(work.path()), ["push", "origin", "HEAD:refs/heads/main"]).expect("git push v2");

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

  Some((remote_base, bare_repo, commit1, commit2))
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
fn add_rejects_frozen_mode() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd
    .current_dir(temp.path())
    .args(["--json", "--frozen", "add", "nlohmann/json"])
    .assert()
    .failure();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["error"]["code"], "frozen_disallows_add");
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

#[test]
fn sync_materializes_header_only_dependencies_and_lockfile_without_app_build() {
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

  let mut sync = cargo_bin_cmd!("joy");
  let sync_assert = sync
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "sync"])
    .assert()
    .success();
  let payload = json_stdout(&sync_assert.get_output().stdout);

  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "sync");
  assert_eq!(payload["data"]["lockfile_updated"], true);
  assert_eq!(payload["data"]["toolchain"], Value::Null);
  assert_eq!(payload["data"]["compiled_dependencies_built"], serde_json::json!([]));
  assert!(
    payload["data"]["include_dirs"].as_array().expect("include_dirs array").iter().any(|v| {
      v.as_str()
        .map(|s| s.replace('\\', "/"))
        .is_some_and(|s| s.ends_with("/.joy/include/deps/nlohmann_json"))
    }),
    "expected staged header include dir in sync output"
  );

  assert!(temp.path().join("joy.lock").is_file(), "expected joy.lock to be written");
  let lock = read_lockfile_toml(&temp);
  let pkg = lock["packages"]
    .as_array()
    .expect("packages")
    .iter()
    .find(|pkg| pkg.get("id").and_then(|v| v.as_str()) == Some("nlohmann/json"))
    .expect("nlohmann/json lock package");
  assert_eq!(pkg["resolved_commit"].as_str(), Some(expected_commit.as_str()));

  assert!(
    !temp.path().join(".joy/build/build.ninja").exists(),
    "sync should not write build.ninja"
  );
  let project_name = temp.path().file_name().and_then(|name| name.to_str()).expect("temp name");
  let binary_name =
    if cfg!(windows) { format!("{project_name}.exe") } else { project_name.to_string() };
  assert!(
    !temp.path().join(".joy/bin").join(binary_name).exists(),
    "sync should not compile or emit app binary"
  );
}

#[test]
fn sync_offline_reports_stable_error_code_when_cache_is_cold() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, _expected_commit)) =
    setup_local_github_remote("nlohmann/json")
  else {
    return;
  };
  let joy_home = temp.path().join("joy-home");
  append_manifest_dependency(&temp, "nlohmann/json", "HEAD");

  let mut sync = cargo_bin_cmd!("joy");
  let assert = sync
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "--offline", "sync"])
    .assert()
    .failure();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "sync");
  assert_eq!(payload["error"]["code"], "offline_cache_miss");
  assert!(
    payload["error"]["message"].as_str().is_some_and(|msg| msg.contains("offline mode")),
    "expected offline mode guidance"
  );
}

#[test]
fn sync_offline_succeeds_with_warm_cache() {
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

  let bogus_remote_base = temp.path().join("missing-remotes");
  let mut sync = cargo_bin_cmd!("joy");
  let assert = sync
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", &bogus_remote_base)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "--offline", "sync"])
    .assert()
    .success();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "sync");
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["toolchain"], Value::Null);
  assert_eq!(payload["data"]["lockfile_updated"], true);
  let lock = read_lockfile_toml(&temp);
  let pkg = lock["packages"]
    .as_array()
    .expect("packages")
    .iter()
    .find(|pkg| pkg.get("id").and_then(|v| v.as_str()) == Some("nlohmann/json"))
    .expect("nlohmann/json package");
  assert_eq!(pkg["resolved_commit"].as_str(), Some(expected_commit.as_str()));
}

#[test]
fn sync_frozen_implies_locked_and_offline_and_rejects_update_lock() {
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

  let mut first_sync = cargo_bin_cmd!("joy");
  first_sync
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["sync"])
    .assert()
    .success();

  let bogus_remote_base = temp.path().join("missing-remotes");
  let mut frozen_sync = cargo_bin_cmd!("joy");
  let frozen_assert = frozen_sync
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", &bogus_remote_base)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "--frozen", "sync"])
    .assert()
    .success();
  let frozen_payload = json_stdout(&frozen_assert.get_output().stdout);
  assert_eq!(frozen_payload["command"], "sync");
  assert_eq!(frozen_payload["ok"], true);
  assert_eq!(frozen_payload["data"]["lockfile_updated"], false);

  let mut invalid = cargo_bin_cmd!("joy");
  let invalid_assert = invalid
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", &bogus_remote_base)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "--frozen", "sync", "--update-lock"])
    .assert()
    .failure();
  let invalid_payload = json_stdout(&invalid_assert.get_output().stdout);
  assert_eq!(invalid_payload["command"], "sync");
  assert_eq!(invalid_payload["error"]["code"], "invalid_lock_flags");
}

#[test]
fn build_offline_reports_stable_error_code_when_cache_is_cold() {
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
  append_manifest_dependency(&temp, "nlohmann/json", "HEAD");

  let mut build = cargo_bin_cmd!("joy");
  let assert = build
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "--offline", "build"])
    .assert()
    .failure();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "build");
  assert_eq!(payload["error"]["code"], "offline_cache_miss");
}

#[test]
fn build_offline_succeeds_with_warm_cache() {
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

  let bogus_remote_base = temp.path().join("missing-remotes");
  let mut build = cargo_bin_cmd!("joy");
  let assert = build
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", &bogus_remote_base)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "--offline", "build"])
    .assert()
    .success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "build");
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["lockfile_updated"], true);
}

#[test]
fn run_frozen_uses_warm_cache_and_existing_lockfile() {
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

  let mut sync = cargo_bin_cmd!("joy");
  sync
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["sync"])
    .assert()
    .success();

  let bogus_remote_base = temp.path().join("missing-remotes");
  let mut run = cargo_bin_cmd!("joy");
  let assert = run
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", &bogus_remote_base)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "--frozen", "run"])
    .assert()
    .success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "run");
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["lockfile_updated"], false);
  let stdout = payload["data"]["stdout"].as_str().expect("stdout");
  assert!(stdout.contains("Hello from joy!"));
}

#[test]
fn remove_updates_manifest_cleans_headers_and_warns_stale_lockfile() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, _commit)) = setup_local_github_remote("nlohmann/json") else {
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

  fs::write(temp.path().join("joy.lock"), "version = 1\n").expect("seed lockfile");

  let mut remove = cargo_bin_cmd!("joy");
  let assert = remove
    .current_dir(temp.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "remove", "nlohmann/json"])
    .assert()
    .success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "remove");
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["removed"], true);
  assert!(
    payload["data"]["warnings"]
      .as_array()
      .expect("warnings array")
      .iter()
      .any(|w| w.as_str().is_some_and(|msg| msg.contains("joy.lock"))),
    "expected stale lockfile warning"
  );

  let manifest = fs::read_to_string(temp.path().join("joy.toml")).expect("manifest");
  assert!(!manifest.contains("\"nlohmann/json\""));
  assert!(!temp.path().join(".joy/include/deps/nlohmann_json").exists());
}

#[test]
fn remove_rejects_frozen_mode() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd
    .current_dir(temp.path())
    .args(["--json", "--frozen", "remove", "nlohmann/json"])
    .assert()
    .failure();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "remove");
  assert_eq!(payload["error"]["code"], "frozen_disallows_remove");
}

#[test]
fn update_rejects_frozen_mode() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert =
    cmd.current_dir(temp.path()).args(["--json", "--frozen", "update"]).assert().failure();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "update");
  assert_eq!(payload["error"]["code"], "frozen_disallows_update");
}

#[test]
fn update_changes_manifest_rev_and_warns_stale_lockfile() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, _commit1, commit2)) =
    setup_local_github_remote_two_commits("nlohmann/json")
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

  fs::write(temp.path().join("joy.lock"), "version = 1\n").expect("seed lockfile");

  let mut update = cargo_bin_cmd!("joy");
  let assert = update
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "update", "nlohmann/json", "--rev", &commit2])
    .assert()
    .success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "update");
  assert_eq!(payload["data"]["manifest_changed"], true);
  assert_eq!(payload["data"]["updated_count"], 1);
  assert_eq!(payload["data"]["updated"][0]["package"], "nlohmann/json");
  assert_eq!(payload["data"]["updated"][0]["rev"], commit2);
  assert_eq!(payload["data"]["updated"][0]["resolved_commit"], commit2);
  assert!(
    payload["data"]["warnings"]
      .as_array()
      .expect("warnings array")
      .iter()
      .any(|w| w.as_str().is_some_and(|msg| msg.contains("joy.lock"))),
    "expected stale lockfile warning"
  );

  let manifest = fs::read_to_string(temp.path().join("joy.toml")).expect("manifest");
  assert!(manifest.contains(&format!("rev = \"{commit2}\"")));
}

#[test]
fn tree_outputs_deterministic_json_and_human_for_direct_dependency() {
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

  let mut tree_json = cargo_bin_cmd!("joy");
  let json_assert = tree_json
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "tree"])
    .assert()
    .success();
  let payload = json_stdout(&json_assert.get_output().stdout);
  assert_eq!(payload["command"], "tree");
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["roots"], serde_json::json!(["nlohmann/json"]));
  let packages = payload["data"]["packages"].as_array().expect("packages array");
  assert_eq!(packages.len(), 1);
  assert_eq!(packages[0]["id"], "nlohmann/json");
  assert_eq!(packages[0]["direct"], true);
  assert_eq!(packages[0]["header_only"], true);
  assert_eq!(packages[0]["resolved_commit"], expected_commit);
  assert_eq!(packages[0]["deps"], serde_json::json!([]));

  let mut tree_human = cargo_bin_cmd!("joy");
  let human_assert = tree_human
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .arg("tree")
    .assert()
    .success();
  let stdout = String::from_utf8_lossy(&human_assert.get_output().stdout);
  assert!(stdout.contains("- nlohmann/json"));
  assert!(stdout.contains(expected_commit.as_str()));
}

#[test]
fn dependency_command_json_payload_shapes_are_stable() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let Some((remote_base, _bare_repo, _commit)) = setup_local_github_remote("nlohmann/json") else {
    return;
  };
  let joy_home = temp.path().join("joy-home");

  let mut add = cargo_bin_cmd!("joy");
  let add_assert = add
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "add", "nlohmann/json"])
    .assert()
    .success();
  let add_payload = json_stdout(&add_assert.get_output().stdout);
  assert_eq!(
    json_object_keys(&add_payload["data"]),
    vec![
      "cache_hit",
      "cache_source_dir",
      "changed",
      "created_env_paths",
      "header_link_kind",
      "header_link_path",
      "header_root",
      "manifest_path",
      "package",
      "project_root",
      "remote_url",
      "resolved_commit",
      "rev",
      "state_index_path",
      "warnings",
      "workspace_member",
      "workspace_root"
    ]
  );

  let mut tree = cargo_bin_cmd!("joy");
  let tree_assert = tree
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "tree"])
    .assert()
    .success();
  let tree_payload = json_stdout(&tree_assert.get_output().stdout);
  assert_eq!(
    json_object_keys(&tree_payload["data"]),
    vec![
      "manifest_path",
      "packages",
      "project_root",
      "roots",
      "workspace_member",
      "workspace_root"
    ]
  );

  let mut update = cargo_bin_cmd!("joy");
  let update_assert = update
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "update", "nlohmann/json"])
    .assert()
    .success();
  let update_payload = json_stdout(&update_assert.get_output().stdout);
  assert_eq!(
    json_object_keys(&update_payload["data"]),
    vec![
      "manifest_changed",
      "manifest_path",
      "project_root",
      "state_index_path",
      "updated",
      "updated_count",
      "warnings",
      "workspace_member",
      "workspace_root"
    ]
  );

  let mut remove = cargo_bin_cmd!("joy");
  let remove_assert = remove
    .current_dir(temp.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "remove", "nlohmann/json"])
    .assert()
    .success();
  let remove_payload = json_stdout(&remove_assert.get_output().stdout);
  assert_eq!(
    json_object_keys(&remove_payload["data"]),
    vec![
      "header_link_path",
      "header_link_removed",
      "manifest_path",
      "package",
      "project_root",
      "removed",
      "state_index_path",
      "warnings",
      "workspace_member",
      "workspace_root"
    ]
  );
}

#[test]
fn workspace_root_requires_member_when_no_default_member_is_set() {
  let temp = TempDir::new().expect("tempdir");
  let member = temp.path().join("apps").join("app");
  fs::create_dir_all(&member).expect("member dir");
  init_project_at(&member);
  fs::write(
    temp.path().join("joy.toml"),
    r#"[workspace]
members = ["apps/app"]
"#,
  )
  .expect("write workspace manifest");

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.current_dir(temp.path()).args(["--json", "tree"]).assert().failure();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["error"]["code"], "workspace_member_required");
}

#[test]
fn workspace_root_routes_tree_to_default_member_and_emits_workspace_metadata() {
  let temp = TempDir::new().expect("tempdir");
  let member = temp.path().join("apps").join("app");
  fs::create_dir_all(&member).expect("member dir");
  init_project_at(&member);
  fs::write(
    temp.path().join("joy.toml"),
    r#"[workspace]
members = ["apps/app"]
default_member = "apps/app"
"#,
  )
  .expect("write workspace manifest");

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.current_dir(temp.path()).args(["--json", "tree"]).assert().success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "tree");
  assert_eq!(payload["data"]["workspace_member"], "apps/app");
  let ws_root = payload["data"]["workspace_root"].as_str().expect("workspace_root");
  let ws_root = fs::canonicalize(ws_root).expect("canonicalize workspace_root");
  assert_eq!(ws_root, fs::canonicalize(temp.path()).expect("canonical temp"));
}

#[test]
fn workspace_root_add_routes_to_selected_member() {
  let temp = TempDir::new().expect("tempdir");
  let member = temp.path().join("apps").join("app");
  fs::create_dir_all(&member).expect("member dir");
  init_project_at(&member);
  fs::write(
    temp.path().join("joy.toml"),
    r#"[workspace]
members = ["apps/app"]
"#,
  )
  .expect("write workspace manifest");

  let Some((remote_base, _bare_repo, _commit)) = setup_local_github_remote("nlohmann/json") else {
    return;
  };
  let joy_home = temp.path().join("joy-home");

  let mut add = cargo_bin_cmd!("joy");
  let assert = add
    .current_dir(temp.path())
    .env("JOY_GITHUB_BASE", remote_base.path())
    .env("JOY_HOME", &joy_home)
    .args(["--json", "-p", "apps/app", "add", "nlohmann/json"])
    .assert()
    .success();
  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["data"]["workspace_member"], "apps/app");
  assert!(member.join("joy.toml").is_file());
  let member_manifest = fs::read_to_string(member.join("joy.toml")).expect("read member manifest");
  assert!(member_manifest.contains("\"nlohmann/json\""));
}
