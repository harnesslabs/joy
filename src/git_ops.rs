use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Output};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitCommandError {
  #[error("failed to run git while {action}: {source}")]
  Spawn {
    action: String,
    #[source]
    source: std::io::Error,
  },
  #[error("git failed while {action} (status {status:?})\\nstdout: {stdout}\\nstderr: {stderr}")]
  Failed { action: String, status: Option<i32>, stdout: String, stderr: String },
}

pub fn run<const N: usize>(
  cwd: Option<&Path>,
  args: [&str; N],
  action: &str,
) -> Result<(), GitCommandError> {
  let output = output(cwd, args.into_iter().map(OsString::from).collect(), action)?;
  ensure_success(action, &output)
}

pub fn run_dynamic(
  cwd: Option<&Path>,
  args: Vec<OsString>,
  action: &str,
) -> Result<(), GitCommandError> {
  let output = output(cwd, args, action)?;
  ensure_success(action, &output)
}

pub fn run_capture<const N: usize>(
  cwd: Option<&Path>,
  args: [&str; N],
  action: &str,
) -> Result<String, GitCommandError> {
  let output = output(cwd, args.into_iter().map(OsString::from).collect(), action)?;
  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(failed_error(action, &output))
  }
}

fn output(
  cwd: Option<&Path>,
  args: Vec<OsString>,
  action: &str,
) -> Result<Output, GitCommandError> {
  let mut cmd = Command::new("git");
  if let Some(dir) = cwd {
    cmd.arg("-C").arg(dir);
  }
  cmd.args(args);
  cmd.output().map_err(|source| GitCommandError::Spawn { action: action.into(), source })
}

fn ensure_success(action: &str, output: &Output) -> Result<(), GitCommandError> {
  if output.status.success() { Ok(()) } else { Err(failed_error(action, output)) }
}

fn failed_error(action: &str, output: &Output) -> GitCommandError {
  GitCommandError::Failed {
    action: action.into(),
    status: output.status.code(),
    stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
    stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
  }
}

#[cfg(test)]
mod tests {
  use std::ffi::OsString;
  use std::process::Command;

  use super::{run, run_capture, run_dynamic};

  #[test]
  fn run_variants_execute_git_version_when_git_is_available() {
    if !git_available() {
      eprintln!("skipping git_ops test: git is not available");
      return;
    }

    run(None, ["--version"], "checking git version").expect("run");
    run_dynamic(None, vec![OsString::from("--version")], "checking git version dynamic")
      .expect("run dynamic");
    let out = run_capture(None, ["--version"], "capturing git version").expect("run capture");
    assert!(out.to_ascii_lowercase().contains("git version"));
  }

  fn git_available() -> bool {
    Command::new("git")
      .arg("--version")
      .output()
      .map(|output| output.status.success())
      .unwrap_or(false)
  }
}
