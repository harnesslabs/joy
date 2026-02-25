use serde_json::json;
use std::process::Command;

use crate::cli::{RunArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::output::{HumanMessageBuilder, progress_detail};

use super::build;

pub fn handle(args: RunArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let execution = build::build_project(build::BuildOptions {
    release: args.release,
    target: args.target.clone(),
    locked: args.locked || runtime.frozen,
    update_lock: args.update_lock,
    offline: runtime.offline,
    progress: runtime.progress,
  })
  .map_err(remap_build_error_for_run)?;

  if runtime.progress {
    progress_detail("Executing built binary");
  }

  let output = Command::new(&execution.binary_path).args(&args.args).output().map_err(|err| {
    JoyError::new(
      "run",
      "run_spawn_failed",
      format!("failed to execute `{}`: {err}", execution.binary_path.display()),
      1,
    )
  })?;

  let exit_code = output.status.code().unwrap_or_default();
  let stdout_text = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
  if !output.status.success() {
    return Err(JoyError::new(
      "run",
      "run_failed",
      format!(
        "program exited with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout_text.trim_end(),
        stderr_text.trim_end()
      ),
      1,
    ));
  }

  let mut human = String::new();
  if !stdout_text.is_empty() {
    human.push_str(stdout_text.trim_end_matches('\n'));
    human.push('\n');
  }
  if !stderr_text.is_empty() {
    human.push_str(stderr_text.trim_end_matches('\n'));
    human.push('\n');
  }
  human.push_str(
    &HumanMessageBuilder::new("Program finished")
      .kv("binary", execution.binary_path.display().to_string())
      .kv("exit code", exit_code.to_string())
      .build(),
  );

  Ok(CommandOutput::new(
    "run",
    human,
    json!({
      "project_root": execution.project_root.display().to_string(),
      "binary_path": execution.binary_path.display().to_string(),
      "build_file": execution.build_file.display().to_string(),
      "toolchain": {
        "compiler_kind": execution.toolchain.compiler.kind.as_str(),
        "compiler_version": execution.toolchain.compiler.version,
        "compiler_path": execution.toolchain.compiler.path.display().to_string(),
        "ninja_path": execution.toolchain.ninja.path.display().to_string(),
      },
      "profile": match execution.profile { crate::ninja::BuildProfile::Debug => "debug", crate::ninja::BuildProfile::Release => "release" },
      "target": execution.target_name,
      "target_default": execution.target_default,
      "args": args.args,
      "exit_code": exit_code,
      "stdout": stdout_text,
      "stderr": stderr_text,
      "lockfile_path": execution.lockfile_path.display().to_string(),
      "lockfile_updated": execution.lockfile_updated,
    }),
  ))
}

fn remap_build_error_for_run(err: JoyError) -> JoyError {
  JoyError::new("run", err.code, err.message, err.exit_code)
}
