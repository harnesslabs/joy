use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
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
      || has_on_path("g++.exe")
      || (cfg!(windows) && has_on_path("cl.exe")))
}

fn has_on_path(program: &str) -> bool {
  which::which(program).is_ok()
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

#[test]
fn build_and_run_supports_extra_sources_include_dirs_and_duplicate_basenames() {
  if !build_tools_available() {
    eprintln!("skipping multi-file E2E: ninja and/or compiler unavailable");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  let manifest_path = temp.path().join("joy.toml");
  let manifest = fs::read_to_string(&manifest_path).expect("read manifest");
  let manifest = manifest.replace(
    "entry = \"src/main.cpp\"\n",
    "entry = \"src/main.cpp\"\nextra_sources = [\"src/dup/main.cpp\"]\ninclude_dirs = [\"include\"]\n",
  );
  fs::write(&manifest_path, manifest).expect("write manifest");

  fs::create_dir_all(temp.path().join("src").join("dup")).expect("dup src dir");
  fs::create_dir_all(temp.path().join("include").join("demo")).expect("include dir");

  fs::write(
    temp.path().join("include").join("demo").join("msg.hpp"),
    "const char* joy_message();\n",
  )
  .expect("write header");
  fs::write(
    temp.path().join("src").join("dup").join("main.cpp"),
    "#include <demo/msg.hpp>\nconst char* joy_message() { return \"multi-file\"; }\n",
  )
  .expect("write extra source");
  fs::write(
    temp.path().join("src").join("main.cpp"),
    "#include <demo/msg.hpp>\n#include <iostream>\n\nint main() { std::cout << joy_message() << std::endl; return 0; }\n",
  )
  .expect("write entry source");

  let mut build = cargo_bin_cmd!("joy");
  let build_assert = build.current_dir(temp.path()).args(["--json", "build"]).assert().success();
  let build_payload = json_stdout(&build_assert.get_output().stdout);
  assert_eq!(build_payload["command"], "build");
  assert_eq!(build_payload["ok"], true);
  let compiled_sources =
    build_payload["data"]["compiled_sources"].as_array().expect("compiled_sources");
  assert_eq!(compiled_sources.len(), 2);
  assert!(
    compiled_sources.iter().any(|v| v.as_str().is_some_and(|s| s.ends_with("/src/main.cpp")))
  );
  assert!(
    compiled_sources.iter().any(|v| v.as_str().is_some_and(|s| s.ends_with("/src/dup/main.cpp")))
  );
  assert!(
    build_payload["data"]["include_dirs"]
      .as_array()
      .expect("include_dirs")
      .iter()
      .any(|v| v.as_str().is_some_and(|s| s.ends_with("/include")))
  );

  let obj_dir = temp.path().join(".joy").join("build").join("obj");
  let object_files = fs::read_dir(&obj_dir)
    .expect("obj dir")
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("o"))
    .collect::<Vec<_>>();
  assert_eq!(object_files.len(), 2, "expected one object per source");
  let mut names = object_files
    .iter()
    .map(|path| path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string())
    .collect::<Vec<_>>();
  names.sort();
  names.dedup();
  assert_eq!(names.len(), 2, "object names should not collide for duplicate basenames");

  let mut run = cargo_bin_cmd!("joy");
  let run_assert = run.current_dir(temp.path()).args(["--json", "run"]).assert().success();
  let run_payload = json_stdout(&run_assert.get_output().stdout);
  let stdout = run_payload["data"]["stdout"].as_str().expect("stdout");
  assert_eq!(stdout.replace("\r\n", "\n"), "multi-file\n");
}

#[cfg(windows)]
#[test]
fn build_handles_crlf_sources_on_windows() {
  if !build_tools_available() {
    eprintln!("skipping windows CRLF E2E: ninja and/or compiler unavailable");
    return;
  }

  let temp = TempDir::new().expect("tempdir");
  init_project(&temp);

  fs::write(
    temp.path().join("src").join("main.cpp"),
    "#include <iostream>\r\n\r\nint main() {\r\n  std::cout << \"crlf\" << std::endl;\r\n  return 0;\r\n}\r\n",
  )
  .expect("write crlf source");

  let mut build = cargo_bin_cmd!("joy");
  let build_assert = build.current_dir(temp.path()).args(["--json", "build"]).assert().success();
  let build_payload = json_stdout(&build_assert.get_output().stdout);
  assert_eq!(build_payload["command"], "build");
  assert_eq!(build_payload["ok"], true);
  assert!(build_payload["data"]["toolchain"]["compiler_kind"].is_string());

  let mut run = cargo_bin_cmd!("joy");
  let run_assert = run.current_dir(temp.path()).args(["--json", "run"]).assert().success();
  let run_payload = json_stdout(&run_assert.get_output().stdout);
  assert_eq!(run_payload["command"], "run");
  assert_eq!(run_payload["data"]["exit_code"], 0);
  assert_eq!(
    run_payload["data"]["stdout"].as_str().expect("stdout").replace("\r\n", "\n"),
    "crlf\n"
  );
}
