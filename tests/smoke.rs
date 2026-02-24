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
    .stdout(predicate::str::contains("--machine"));
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
