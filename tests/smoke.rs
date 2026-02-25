use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;

fn json_stdout(output: &[u8]) -> Value {
  serde_json::from_slice(output).expect("valid json")
}

#[test]
fn help_smoke_test() {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd
    .arg("--help")
    .assert()
    .success()
    .stdout(predicate::str::contains("Native C++ package and build manager"))
    .stdout(predicate::str::contains("new"))
    .stdout(predicate::str::contains("--json"))
    .stdout(predicate::str::contains("--machine"))
    .stdout(predicate::str::contains("--color"))
    .stdout(predicate::str::contains("--progress"))
    .stdout(predicate::str::contains("--glyphs"))
    .stdout(predicate::str::contains("Examples:"))
    .stdout(predicate::str::contains("joy sync"));
}

#[test]
fn build_help_includes_examples() {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd
    .args(["build", "--help"])
    .assert()
    .success()
    .stdout(predicate::str::contains("Examples:"))
    .stdout(predicate::str::contains("joy build --locked"))
    .stdout(predicate::str::contains("joy --offline build"));
}

#[test]
fn json_mode_returns_json_for_missing_subcommand_parse_errors() {
  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.arg("--json").assert().code(2);
  let payload = json_stdout(&assert.get_output().stdout);

  assert_eq!(payload["ok"], false);
  assert_eq!(payload["command"], "cli");
  assert_eq!(payload["error"]["code"], "cli_parse_error");
}

#[test]
fn json_mode_returns_json_for_subcommand_argument_parse_errors() {
  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.args(["--json", "new"]).assert().code(2);
  let payload = json_stdout(&assert.get_output().stdout);

  assert_eq!(payload["ok"], false);
  assert_eq!(payload["command"], "cli");
  assert_eq!(payload["error"]["code"], "cli_parse_error");
}

#[test]
fn recipe_check_json_validates_bundled_recipes() {
  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.args(["--json", "recipe-check"]).assert().success();
  let payload = json_stdout(&assert.get_output().stdout);

  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "recipe-check");
  assert!(payload["data"]["recipe_count"].as_u64().is_some_and(|n| n >= 3));
  assert!(
    payload["data"]["packages"]
      .as_array()
      .expect("packages array")
      .iter()
      .any(|v| v.as_str() == Some("nlohmann/json"))
  );
}

#[test]
fn doctor_json_reports_environment_checks() {
  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.args(["--json", "doctor"]).assert().success();
  let payload = json_stdout(&assert.get_output().stdout);

  assert_eq!(payload["ok"], true);
  assert_eq!(payload["command"], "doctor");
  assert!(payload["data"]["env"]["path_present"].is_boolean());
  assert!(payload["data"]["tools"]["git"]["ok"].is_boolean());
  assert!(payload["data"]["cache"]["ok"].is_boolean());
  assert!(payload["data"]["recipes"]["ok"].is_boolean());
  assert!(payload["data"]["toolchain"]["ok"].is_boolean());
  assert!(payload["data"]["project"].is_object() || payload["data"]["project"].is_null());
  assert!(payload["data"]["artifacts"].is_object() || payload["data"]["artifacts"].is_null());
  assert!(payload["data"]["lockfile"].is_object() || payload["data"]["lockfile"].is_null());
  assert!(payload["data"]["dependency_metadata"].is_object());
  assert!(payload["data"]["project_warnings"].is_array());
  assert!(payload["data"]["project_hints"].is_array());
}

#[test]
fn doctor_human_output_is_sectioned() {
  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.arg("doctor").assert().success();
  let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
  assert!(stdout.contains("Doctor "));
  assert!(stdout.contains("Summary"));
  assert!(stdout.contains("Tools"));
  assert!(stdout.contains("- git:"));
}

#[test]
fn recipe_check_human_output_is_structured() {
  let mut cmd = cargo_bin_cmd!("joy");
  let assert = cmd.arg("recipe-check").assert().success();
  let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
  assert!(stdout.contains("Recipe metadata validation passed"));
  assert!(stdout.contains("- bundled recipes:"));
}
