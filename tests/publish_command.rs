use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
}

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
  let output = ProcessCommand::new("git").arg("-C").arg(cwd).args(args).output().expect("run git");
  assert!(
    output.status.success(),
    "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
    args,
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );
}

fn git_is_available() -> bool {
  ProcessCommand::new("git")
    .arg("--version")
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

fn init_project(path: &Path) {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(path).arg("init").assert().success();
}

fn setup_registry_repo() -> TempDir {
  let repo = TempDir::new().expect("registry repo");
  run_git(repo.path(), ["init"]);
  run_git(repo.path(), ["config", "user.email", "joy-tests@example.com"]);
  run_git(repo.path(), ["config", "user.name", "Joy Tests"]);
  fs::write(repo.path().join("index.toml"), "version = 2\n").expect("write index");
  run_git(repo.path(), ["add", "index.toml"]);
  run_git(repo.path(), ["commit", "-m", "init index"]);
  repo
}

fn setup_source_remote_with_tag(package: &str, tag: &str) -> Option<TempDir> {
  if !git_is_available() {
    eprintln!("skipping publish test: git is not available");
    return None;
  }

  let mut parts = package.split('/');
  let owner = parts.next().expect("owner");
  let repo = parts.next().expect("repo");

  let remote_base = TempDir::new().expect("remote base");
  let work = TempDir::new().expect("work repo");
  let bare_repo = remote_base.path().join(owner).join(format!("{repo}.git"));
  fs::create_dir_all(bare_repo.parent().expect("bare parent")).expect("create bare parent");

  run_git(work.path(), ["init"]);
  run_git(work.path(), ["config", "user.email", "joy-tests@example.com"]);
  run_git(work.path(), ["config", "user.name", "Joy Tests"]);
  fs::create_dir_all(work.path().join("include").join("widgets")).expect("header dir");
  fs::write(
    work.path().join("include").join("widgets").join("widgets.hpp"),
    "// widgets fixture header\n",
  )
  .expect("write header");
  fs::write(work.path().join("README.md"), "# widgets fixture\n").expect("write readme");
  run_git(work.path(), ["add", "."]);
  run_git(work.path(), ["commit", "-m", "fixture"]);
  run_git(work.path(), ["tag", tag]);

  let output = ProcessCommand::new("git")
    .args(["init", "--bare", bare_repo.to_string_lossy().as_ref()])
    .output()
    .expect("git init --bare");
  assert!(output.status.success(), "git init --bare failed");

  run_git(work.path(), ["remote", "add", "origin", bare_repo.to_string_lossy().as_ref()]);
  run_git(work.path(), ["push", "origin", "HEAD:refs/heads/main"]);
  run_git(work.path(), ["push", "origin", tag]);

  Some(remote_base)
}

#[test]
fn publish_owner_yank_roundtrip_with_local_registry() {
  if !git_is_available() {
    eprintln!("skipping publish test: git is not available");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  let registry_repo = setup_registry_repo();

  let package_dir = temp.path().join("package");
  let app_dir = temp.path().join("app");
  let app2_dir = temp.path().join("app2");
  fs::create_dir_all(&package_dir).expect("package dir");
  fs::create_dir_all(&app_dir).expect("app dir");
  fs::create_dir_all(&app2_dir).expect("app2 dir");

  let joy_home = temp.path().join("joy-home");

  let mut pkg_init = cargo_bin_cmd!("joy");
  pkg_init
    .current_dir(&package_dir)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "package", "init", "acme/widgets", "--version", "1.2.3"])
    .assert()
    .success();

  let mut reg_add_pkg = cargo_bin_cmd!("joy");
  reg_add_pkg
    .current_dir(&package_dir)
    .env("JOY_HOME", &joy_home)
    .args([
      "registry",
      "add",
      "local",
      registry_repo.path().to_string_lossy().as_ref(),
      "--project",
      "--default",
    ])
    .assert()
    .success();

  let mut publish = cargo_bin_cmd!("joy");
  publish
    .current_dir(&package_dir)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "publish", "--registry", "local", "--rev", "v1.2.3"])
    .assert()
    .success();

  let mut owner_add = cargo_bin_cmd!("joy");
  owner_add
    .current_dir(&package_dir)
    .env("JOY_HOME", &joy_home)
    .args(["owner", "add", "acme/widgets", "alice", "--registry", "local"])
    .assert()
    .success();

  let mut owner_list = cargo_bin_cmd!("joy");
  let owner_assert = owner_list
    .current_dir(&package_dir)
    .env("JOY_HOME", &joy_home)
    .args(["--json", "owner", "list", "acme/widgets", "--registry", "local"])
    .assert()
    .success();
  let owner_payload = json_stdout(&owner_assert.get_output().stdout);
  let owners = owner_payload["data"]["owners"].as_array().expect("owners array");
  assert!(owners.iter().any(|owner| owner.as_str() == Some("alice")));

  let Some(source_remote_base) = setup_source_remote_with_tag("acme/widgets", "v1.2.3") else {
    return;
  };

  init_project(&app_dir);
  let mut reg_add_app = cargo_bin_cmd!("joy");
  reg_add_app
    .current_dir(&app_dir)
    .env("JOY_HOME", &joy_home)
    .args([
      "registry",
      "add",
      "local",
      registry_repo.path().to_string_lossy().as_ref(),
      "--project",
      "--default",
    ])
    .assert()
    .success();

  let mut add_registry = cargo_bin_cmd!("joy");
  add_registry
    .current_dir(&app_dir)
    .env("JOY_HOME", &joy_home)
    .env("JOY_GITHUB_BASE", source_remote_base.path())
    .env("JOY_REGISTRY_DEFAULT", registry_repo.path())
    .args(["add", "registry:acme/widgets", "--registry", "local", "--version", "^1"])
    .assert()
    .success();

  let mut yank = cargo_bin_cmd!("joy");
  yank
    .current_dir(&package_dir)
    .env("JOY_HOME", &joy_home)
    .args(["yank", "acme/widgets", "--version", "1.2.3", "--registry", "local"])
    .assert()
    .success();

  init_project(&app2_dir);
  let mut reg_add_app2 = cargo_bin_cmd!("joy");
  reg_add_app2
    .current_dir(&app2_dir)
    .env("JOY_HOME", &joy_home)
    .args([
      "registry",
      "add",
      "local",
      registry_repo.path().to_string_lossy().as_ref(),
      "--project",
      "--default",
    ])
    .assert()
    .success();

  let mut add_after_yank = cargo_bin_cmd!("joy");
  let fail_assert = add_after_yank
    .current_dir(&app2_dir)
    .env("JOY_HOME", &joy_home)
    .env("JOY_GITHUB_BASE", source_remote_base.path())
    .env("JOY_REGISTRY_DEFAULT", registry_repo.path())
    .args(["--json", "add", "registry:acme/widgets", "--registry", "local", "--version", "^1"])
    .assert()
    .failure();
  let fail_payload = json_stdout(&fail_assert.get_output().stdout);
  assert_eq!(fail_payload["error"]["code"], "version_not_found");

  let mut unyank = cargo_bin_cmd!("joy");
  unyank
    .current_dir(&package_dir)
    .env("JOY_HOME", &joy_home)
    .args(["yank", "acme/widgets", "--version", "1.2.3", "--undo", "--registry", "local"])
    .assert()
    .success();

  let mut add_after_unyank = cargo_bin_cmd!("joy");
  add_after_unyank
    .current_dir(&app2_dir)
    .env("JOY_HOME", &joy_home)
    .env("JOY_GITHUB_BASE", source_remote_base.path())
    .env("JOY_REGISTRY_DEFAULT", registry_repo.path())
    .args(["add", "registry:acme/widgets", "--registry", "local", "--version", "^1"])
    .assert()
    .success();
}
