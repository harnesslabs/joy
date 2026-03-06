use assert_cmd::cargo::cargo_bin_cmd;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use tar::Builder;
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

fn git_is_available() -> bool {
  ProcessCommand::new("git")
    .arg("--version")
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

fn run_git(cwd: &Path, args: &[&str]) {
  let output = ProcessCommand::new("git").current_dir(cwd).args(args).output().expect("run git");
  assert!(
    output.status.success(),
    "git command failed: git {}\\nstdout:\\n{}\\nstderr:\\n{}",
    args.join(" "),
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );
}

fn run_git_capture(cwd: &Path, args: &[&str]) -> String {
  let output =
    ProcessCommand::new("git").current_dir(cwd).args(args).output().expect("run git capture");
  assert!(
    output.status.success(),
    "git command failed: git {}\\nstdout:\\n{}\\nstderr:\\n{}",
    args.join(" "),
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );
  String::from_utf8(output.stdout).expect("git output utf8")
}

fn setup_local_git_dependency(project_root: &Path) -> (PathBuf, String) {
  let repo = project_root.join("fixtures").join("mylib-git");
  fs::create_dir_all(repo.join("include").join("mylib")).expect("create fixture include dir");
  fs::write(
    repo.join("include").join("mylib").join("mylib.hpp"),
    "#pragma once\ninline int joy_mylib() { return 42; }\n",
  )
  .expect("write fixture header");
  run_git(&repo, &["init"]);
  run_git(&repo, &["config", "user.email", "joy-tests@example.com"]);
  run_git(&repo, &["config", "user.name", "Joy Tests"]);
  run_git(&repo, &["add", "."]);
  run_git(&repo, &["commit", "-m", "fixture"]);
  let head = run_git_capture(&repo, &["rev-parse", "HEAD"]).trim().to_string();
  (repo, head)
}

fn sha256_hex(path: &Path) -> String {
  let bytes = fs::read(path).expect("read bytes");
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  format!("{:x}", hasher.finalize())
}

fn write_archive_fixture(path: &Path, header_contents: &str) -> String {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("create archive parent");
  }
  let file = fs::File::create(path).expect("create archive file");
  let encoder = GzEncoder::new(file, Compression::default());
  let mut tar = Builder::new(encoder);
  let body = header_contents.as_bytes();
  let mut header = tar::Header::new_gnu();
  header.set_mode(0o644);
  header.set_mtime(0);
  header.set_size(body.len() as u64);
  header.set_cksum();
  tar
    .append_data(&mut header, "archive/include/archive_dep/header.hpp", body)
    .expect("append archive entry");
  tar.finish().expect("finish tar stream");
  tar.into_inner().expect("encoder").finish().expect("finish gzip stream");
  sha256_hex(path)
}

#[test]
fn add_git_dependency_records_manifest_with_alias_and_rev() {
  if !git_is_available() {
    eprintln!("skipping test: git is not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let (repo_path, repo_head) = setup_local_git_dependency(temp.path());
  let repo_ref = repo_path.to_string_lossy().to_string();

  let mut add = cargo_bin_cmd!("joy");
  let add_assert = add
    .current_dir(temp.path())
    .args(["--json", "add", &format!("git+{repo_ref}"), "--as", "mylib", "--rev", &repo_head])
    .assert()
    .success();

  let payload = json_stdout(&add_assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "add");
  assert_eq!(payload["data"]["key"], "mylib");
  assert_eq!(payload["data"]["source"], "git");
  assert_eq!(payload["data"]["rev"], repo_head);
  assert_eq!(payload["data"]["sync_attempted"], true);
  assert!(payload["data"]["fetched"].is_null());

  let manifest = read_manifest_toml(&temp);
  assert_eq!(manifest["dependencies"]["mylib"]["source"].as_str(), Some("git"));
  assert_eq!(manifest["dependencies"]["mylib"]["git"].as_str(), Some(repo_ref.as_str()));
  assert_eq!(manifest["dependencies"]["mylib"]["rev"].as_str(), Some(repo_head.as_str()));
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
fn update_archive_dependency_updates_sha256_and_rehydrates_cache() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let archive_path = temp.path().join("fixtures").join("archive_dep.tar.gz");
  let checksum_v1 = write_archive_fixture(&archive_path, "v1");
  let archive_ref = format!("file://{}", archive_path.display());

  let mut add = cargo_bin_cmd!("joy");
  add
    .current_dir(temp.path())
    .args([
      "add",
      &format!("archive:{archive_ref}"),
      "--as",
      "archive_dep",
      "--sha256",
      &checksum_v1,
    ])
    .assert()
    .success();

  let checksum_v2 = write_archive_fixture(&archive_path, "v2");

  let mut update = cargo_bin_cmd!("joy");
  let update_assert = update
    .current_dir(temp.path())
    .args(["--json", "update", "archive_dep", "--sha256", &checksum_v2])
    .assert()
    .success();
  let payload = json_stdout(&update_assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["updated_count"], 1);
  assert_eq!(payload["data"]["updated"][0]["source"], "archive");
  assert_eq!(payload["data"]["updated"][0]["staged_only"], false);
  assert!(payload["data"]["updated"][0]["resolved_commit"].as_str().is_some());

  let manifest = read_manifest_toml(&temp);
  assert_eq!(
    manifest["dependencies"]["archive_dep"]["sha256"].as_str(),
    Some(checksum_v2.as_str())
  );
}

#[test]
fn remove_path_dependency_cleans_manifest_entry() {
  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);
  let local_dep = temp.path().join("vendor").join("localdep").join("include");
  fs::create_dir_all(&local_dep).expect("create localdep include dir");
  fs::write(local_dep.join("localdep.hpp"), "#pragma once\n").expect("write localdep header");

  let mut add = cargo_bin_cmd!("joy");
  add
    .current_dir(temp.path())
    .args(["add", "path:vendor/localdep", "--as", "localdep"])
    .assert()
    .success();

  let mut remove = cargo_bin_cmd!("joy");
  let remove_assert =
    remove.current_dir(temp.path()).args(["--json", "remove", "localdep"]).assert().success();
  let payload = json_stdout(&remove_assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["source"], "path");
  assert_eq!(payload["data"]["removed"], true);

  let manifest = read_manifest_toml(&temp);
  let deps = manifest["dependencies"].as_table().expect("dependencies table");
  assert!(!deps.contains_key("localdep"));
}

#[test]
fn outdated_supports_git_path_and_archive_source_filters() {
  if !git_is_available() {
    eprintln!("skipping test: git is not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let (repo_path, repo_head) = setup_local_git_dependency(temp.path());
  let repo_ref = repo_path.to_string_lossy().to_string();

  let local_dep = temp.path().join("vendor").join("localdep").join("include");
  fs::create_dir_all(&local_dep).expect("create localdep include dir");
  fs::write(local_dep.join("localdep.hpp"), "#pragma once\n").expect("write localdep header");

  let archive_path = temp.path().join("fixtures").join("archive_dep.tar.gz");
  let archive_checksum = write_archive_fixture(&archive_path, "v1");
  let archive_ref = format!("file://{}", archive_path.display());

  cargo_bin_cmd!("joy")
    .current_dir(temp.path())
    .args(["add", &format!("git+{repo_ref}"), "--as", "mylib", "--rev", &repo_head])
    .assert()
    .success();

  cargo_bin_cmd!("joy")
    .current_dir(temp.path())
    .args(["add", "path:vendor/localdep", "--as", "localdep"])
    .assert()
    .success();

  cargo_bin_cmd!("joy")
    .current_dir(temp.path())
    .args([
      "add",
      &format!("archive:{archive_ref}"),
      "--as",
      "archive_dep",
      "--sha256",
      &archive_checksum,
    ])
    .assert()
    .success();

  for source in ["git", "path", "archive"] {
    let mut cmd = cargo_bin_cmd!("joy");
    let assert = cmd
      .current_dir(temp.path())
      .args(["--json", "outdated", "--sources", source])
      .assert()
      .success();
    let payload = json_stdout(&assert.get_output().stdout);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["data"]["sources"], source);
    assert_eq!(payload["data"]["summary"]["unsupported_count"], 0);
    assert_eq!(payload["data"]["summary"]["package_count"], 1);
    let rows = payload["data"]["packages"].as_array().expect("outdated rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["source"], source);
    assert_ne!(rows[0]["status"], "unknown_source");
  }
}

#[test]
fn verify_strict_succeeds_for_git_path_and_archive_dependencies() {
  if !git_is_available() {
    eprintln!("skipping test: git is not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let (repo_path, repo_head) = setup_local_git_dependency(temp.path());
  let repo_ref = repo_path.to_string_lossy().to_string();

  let local_dep = temp.path().join("vendor").join("localdep").join("include");
  fs::create_dir_all(&local_dep).expect("create localdep include dir");
  fs::write(local_dep.join("localdep.hpp"), "#pragma once\n").expect("write localdep header");

  let archive_path = temp.path().join("fixtures").join("archive_dep.tar.gz");
  let archive_checksum = write_archive_fixture(&archive_path, "v1");
  let archive_ref = format!("file://{}", archive_path.display());

  cargo_bin_cmd!("joy")
    .current_dir(temp.path())
    .args(["add", &format!("git+{repo_ref}"), "--as", "mylib", "--rev", &repo_head])
    .assert()
    .success();

  cargo_bin_cmd!("joy")
    .current_dir(temp.path())
    .args(["add", "path:vendor/localdep", "--as", "localdep"])
    .assert()
    .success();

  cargo_bin_cmd!("joy")
    .current_dir(temp.path())
    .args([
      "add",
      &format!("archive:{archive_ref}"),
      "--as",
      "archive_dep",
      "--sha256",
      &archive_checksum,
    ])
    .assert()
    .success();

  let mut verify = cargo_bin_cmd!("joy");
  let verify_assert =
    verify.current_dir(temp.path()).args(["--json", "verify", "--strict"]).assert().success();
  let payload = json_stdout(&verify_assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["data"]["summary"]["failed_count"], 0);
  assert_eq!(payload["data"]["summary"]["warning_count"], 0);
}
