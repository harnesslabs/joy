use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
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
  let assert = cmd.current_dir(temp.path()).args(["--json", "build"]).assert().code(2);

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["error"]["code"], "not_implemented");

  for dir in [".joy/include", ".joy/lib", ".joy/build", ".joy/bin", ".joy/state"] {
    assert!(temp.path().join(dir).is_dir(), "missing {dir}");
  }
}
