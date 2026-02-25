use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn read_to_string(path: impl AsRef<std::path::Path>) -> String {
  fs::read_to_string(path).expect("read file")
}

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
}

#[test]
fn joy_new_creates_project_files() {
  let temp = TempDir::new().expect("tempdir");
  let project_name = "test_proj";

  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).args(["new", project_name]).assert().success();

  let root = temp.path().join(project_name);
  let manifest = root.join("joy.toml");
  let main_cpp = root.join("src/main.cpp");
  let gitignore = root.join(".gitignore");

  assert!(manifest.is_file());
  assert!(main_cpp.is_file());
  assert!(gitignore.is_file());

  assert_eq!(
    read_to_string(&manifest),
    r#"[project]
name = "test_proj"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"
# extra_sources = ["src/lib.cpp", "src/feature/main.cpp"]
# include_dirs = ["include"]
# [[project.targets]]
# name = "tool"
# entry = "src/tool.cpp"
# extra_sources = ["src/shared.cpp"]
# include_dirs = ["tools/include"]

[dependencies]
"#
  );
  assert_eq!(
    read_to_string(&main_cpp),
    r#"#include <iostream>

int main() {
  std::cout << "Hello from joy!" << std::endl;
  return 0;
}
"#
  );
  assert_eq!(
    read_to_string(&gitignore),
    r#".joy/
compile_commands.json
build/
*.o
*.obj
*.exe
"#
  );
}

#[test]
fn joy_new_fails_for_non_empty_directory_without_force() {
  let temp = TempDir::new().expect("tempdir");
  let root = temp.path().join("existing");
  fs::create_dir_all(&root).expect("create dir");
  fs::write(root.join("README.txt"), "occupied").expect("seed dir");

  let mut cmd = cargo_bin_cmd!("joy");
  cmd
    .current_dir(temp.path())
    .args(["new", "existing"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("non_empty_directory"));
}

#[test]
fn joy_new_force_allows_existing_directory() {
  let temp = TempDir::new().expect("tempdir");
  let root = temp.path().join("existing");
  fs::create_dir_all(root.join("src")).expect("create src");
  fs::write(root.join("notes.txt"), "keep me").expect("seed extra file");

  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).args(["new", "existing", "--force"]).assert().success();

  assert!(root.join("joy.toml").is_file());
  assert!(root.join("src/main.cpp").is_file());
  assert_eq!(read_to_string(root.join("notes.txt")), "keep me");
}

#[test]
fn joy_init_initializes_current_directory() {
  let temp = TempDir::new().expect("tempdir");

  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).arg("init").assert().success();

  assert!(temp.path().join("joy.toml").is_file());
  assert!(temp.path().join("src/main.cpp").is_file());
  assert!(temp.path().join(".gitignore").is_file());

  let dir_name = temp.path().file_name().and_then(|name| name.to_str()).expect("temp dir name");
  let manifest = read_to_string(temp.path().join("joy.toml"));
  assert!(manifest.contains(&format!("name = \"{dir_name}\"")));
}

#[test]
fn joy_init_fails_when_manifest_exists_without_force() {
  let temp = TempDir::new().expect("tempdir");
  fs::write(temp.path().join("joy.toml"), "[project]\nname = \"x\"\n").expect("existing manifest");

  let mut cmd = cargo_bin_cmd!("joy");
  cmd
    .current_dir(temp.path())
    .arg("init")
    .assert()
    .failure()
    .stderr(predicate::str::contains("path_exists"));
}

#[test]
fn joy_new_json_returns_success_envelope() {
  let temp = TempDir::new().expect("tempdir");

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.current_dir(temp.path()).args(["--json", "new", "json_proj"]).assert().success();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "new");
  assert_eq!(payload["data"]["project_name"], "json_proj");
  let actual_root = payload["data"]["project_root"].as_str().expect("project_root string");
  let expected_root = fs::canonicalize(temp.path().join("json_proj")).expect("canonical root");
  let actual_root = fs::canonicalize(actual_root).expect("canonicalized project_root");
  assert_eq!(actual_root, expected_root);

  let created_paths = payload["data"]["created_paths"].as_array().expect("created_paths array");
  let expected_manifest =
    fs::canonicalize(temp.path().join("json_proj").join("joy.toml")).expect("canonical manifest");
  let has_manifest = created_paths
    .iter()
    .filter_map(|path| path.as_str())
    .filter_map(|path| fs::canonicalize(path).ok())
    .any(|path| path == expected_manifest);
  assert!(has_manifest, "expected manifest path in created_paths");
}

#[test]
fn joy_new_uses_target_directory_basename_for_manifest_name_when_given_absolute_path() {
  let temp = TempDir::new().expect("tempdir");
  let root = temp.path().join("abs_project");
  let root_arg = root.to_str().expect("utf-8 temp path");

  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).args(["new", root_arg]).assert().success();

  let manifest = read_to_string(root.join("joy.toml"));
  assert!(manifest.contains("name = \"abs_project\""));
  assert!(!manifest.contains(root_arg));
}

#[test]
fn build_and_run_return_manifest_not_found_in_empty_directory() {
  let temp = TempDir::new().expect("tempdir");
  for (command, args) in [("build", vec!["build"]), ("run", vec!["run"])] {
    let mut cmd = cargo_bin_cmd!("joy");
    let assert = cmd.current_dir(temp.path()).arg("--json").args(args).assert().code(1);

    let payload = json_stdout(&assert.get_output().stdout);
    assert_eq!(payload["ok"], false, "command={command}");
    assert_eq!(payload["command"], command, "command={command}");
    assert_eq!(payload["error"]["code"], "manifest_not_found", "command={command}");
  }
}
