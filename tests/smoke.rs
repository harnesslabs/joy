use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn help_smoke_test() {
  let mut cmd = cargo_bin_cmd!("joy");
  cmd
    .arg("--help")
    .assert()
    .success()
    .stdout(predicate::str::contains("Native C++ package and build manager"))
    .stdout(predicate::str::contains("new"))
    .stdout(predicate::str::contains("--json"));
}
