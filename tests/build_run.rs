use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
}

fn init_project(temp: &TempDir) {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd.current_dir(temp.path()).arg("init").assert().success();
}

fn build_tools_available() -> bool {
  (has_on_path("ninja") || has_on_path("ninja-build"))
    && (has_on_path("clang++")
      || has_on_path("g++")
      || has_on_path("clang++.exe")
      || has_on_path("g++.exe"))
}

fn has_on_path(program: &str) -> bool {
  ProcessCommand::new(program)
    .arg("--version")
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

#[test]
fn build_json_compiles_template_project_when_tooling_is_available() {
  if !build_tools_available() {
    eprintln!("skipping build E2E: ninja and/or compiler unavailable");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.current_dir(temp.path()).args(["--json", "build"]).assert().success();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "build");
  assert!(payload["data"]["toolchain"]["compiler_kind"].is_string());
  assert!(payload["data"]["toolchain"]["ninja_path"].is_string());

  assert!(temp.path().join(".joy/build/build.ninja").is_file());
  let project_name = temp.path().file_name().and_then(|name| name.to_str()).expect("temp name");
  let binary_name =
    if cfg!(windows) { format!("{project_name}.exe") } else { project_name.to_string() };
  assert!(temp.path().join(".joy/bin").join(binary_name).is_file());
}

#[test]
fn run_json_executes_template_project_when_tooling_is_available() {
  if !build_tools_available() {
    eprintln!("skipping run E2E: ninja and/or compiler unavailable");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let mut cmd = cargo_bin_cmd!("joy");
  let assert =
    cmd.current_dir(temp.path()).args(["--json", "run", "--", "arg1"]).assert().success();

  let payload = json_stdout(&assert.get_output().stdout);
  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "run");
  assert_eq!(payload["data"]["exit_code"], 0);
  let stdout = payload["data"]["stdout"].as_str().expect("stdout string");
  assert!(stdout.contains("Hello from joy!"));
}
