use serde::Serialize;
use serde_json::Value;
use std::io::IsTerminal;
use std::io::{self, Write};

use crate::commands::CommandOutput;
use crate::error::JoyError;

/// Output mode selected by CLI flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
  Human,
  Json,
}

/// Builder for structured human-mode command output.
#[derive(Debug, Default, Clone)]
pub struct HumanMessageBuilder {
  title: String,
  lines: Vec<String>,
  warnings: Vec<String>,
  hints: Vec<String>,
}

impl HumanMessageBuilder {
  pub fn new(title: impl Into<String>) -> Self {
    Self { title: title.into(), ..Self::default() }
  }

  pub fn line(mut self, line: impl Into<String>) -> Self {
    self.lines.push(line.into());
    self
  }

  pub fn kv(mut self, key: &str, value: impl Into<String>) -> Self {
    self.lines.push(format!("- {key}: {}", value.into()));
    self
  }

  pub fn warning(mut self, warning: impl Into<String>) -> Self {
    self.warnings.push(warning.into());
    self
  }

  pub fn hint(mut self, hint: impl Into<String>) -> Self {
    self.hints.push(hint.into());
    self
  }

  pub fn build(self) -> String {
    let mut out = String::new();
    out.push_str(&self.title);
    for line in self.lines {
      out.push('\n');
      out.push_str(&line);
    }
    for warning in self.warnings {
      out.push('\n');
      out.push_str("warning: ");
      out.push_str(&warning);
    }
    for hint in self.hints {
      out.push('\n');
      out.push_str("hint: ");
      out.push_str(&hint);
    }
    out
  }
}

#[derive(Debug, Serialize)]
struct SuccessEnvelope<'a> {
  ok: bool,
  command: &'a str,
  data: &'a Value,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
  ok: bool,
  command: &'a str,
  error: ErrorPayload<'a>,
}

#[derive(Debug, Serialize)]
struct ErrorPayload<'a> {
  code: &'a str,
  message: &'a str,
}

/// Render a successful command result to stdout in the selected mode.
pub fn print_success(mode: OutputMode, result: &CommandOutput) -> io::Result<()> {
  match mode {
    OutputMode::Human => write_line(&mut io::stdout(), &result.human_message),
    OutputMode::Json => {
      let envelope = success_envelope(result);
      write_json(&mut io::stdout(), &envelope)
    },
  }
}

/// Render a command error in human or machine-readable form.
///
/// JSON mode writes to stdout intentionally so callers can treat all command output as a single
/// stream while still relying on process exit codes for success/failure.
pub fn print_error(mode: OutputMode, command: &'static str, err: &JoyError) -> io::Result<()> {
  match mode {
    OutputMode::Human => {
      let mut stderr = io::stderr();
      write_line(
        &mut stderr,
        &format!("error[{code}]: {message}", code = err.code, message = err.message),
      )?;
      if let Some(hint) = human_error_hint(command, err) {
        write_line(&mut stderr, &format!("hint: {hint}"))?;
      }
      Ok(())
    },
    OutputMode::Json => {
      let envelope = error_envelope(command, err);
      write_json(&mut io::stdout(), &envelope)
    },
  }
}

fn write_json<T: Serialize>(writer: &mut impl Write, value: &T) -> io::Result<()> {
  serde_json::to_writer_pretty(&mut *writer, value)?;
  writer.write_all(b"\n")?;
  writer.flush()
}

fn write_line(writer: &mut impl Write, line: &str) -> io::Result<()> {
  writer.write_all(line.as_bytes())?;
  writer.write_all(b"\n")?;
  writer.flush()
}

fn success_envelope<'a>(result: &'a CommandOutput) -> SuccessEnvelope<'a> {
  SuccessEnvelope { ok: true, command: result.command, data: &result.data }
}

fn error_envelope<'a>(command: &'a str, err: &'a JoyError) -> ErrorEnvelope<'a> {
  ErrorEnvelope {
    ok: false,
    command,
    error: ErrorPayload { code: err.code, message: &err.message },
  }
}

fn human_error_hint(command: &str, err: &JoyError) -> Option<String> {
  match err.code {
    "manifest_not_found" => Some("Run `joy init` in this directory (or `joy new <name>`).".into()),
    "toolchain_not_found" | "toolchain_probe_failed" => {
      Some("Run `joy doctor` to inspect compiler and ninja availability.".into())
    },
    "lockfile_missing" | "lockfile_stale" | "lockfile_incomplete" | "lockfile_mismatch" => {
      let example = format!("joy {command} --update-lock");
      if err.message.contains("--update-lock") {
        Some(format!("Refresh the lockfile and rerun (for example `{example}`)."))
      } else {
        Some(format!("Refresh the lockfile with `{example}`."))
      }
    },
    "offline_cache_miss" => Some(
      "Warm the cache online first (for example `joy sync`) or rerun without `--offline`.".into(),
    ),
    "offline_network_disabled" => {
      Some("Rerun without `--offline` / `--frozen`, or ensure the cache is already warm.".into())
    },
    "invalid_version_requirement" => {
      Some("Use a valid semver requirement such as `^1`, `~1.2`, or `>=1.2, <2.0`.".into())
    },
    "version_not_found" => Some(
      "Check available tags for the dependency (or relax the version range) and rerun online to refresh the mirror.".into(),
    ),
    "recipe_load_failed" => {
      Some("Run `joy doctor` to validate the bundled recipe store and local environment.".into())
    },
    "dependency_not_found" if matches!(command, "remove" | "update") => {
      Some("Use `joy tree` to inspect current dependencies before editing.".into())
    },
    _ => None,
  }
}

fn write_progress_line(prefix: &str, message: &str) -> io::Result<()> {
  let mut stderr = io::stderr();
  write_line(&mut stderr, &format!("{prefix} {message}"))
}

/// Emit a human-mode stage/status line to stderr.
pub fn progress_stage(message: &str) {
  let _ = write_progress_line("==>", message);
}

/// Emit a human-mode detail line to stderr.
pub fn progress_detail(message: &str) {
  let _ = write_progress_line("  ->", message);
}

/// Emit a human-mode detail line to stderr only when stderr is a TTY.
pub fn progress_detail_tty(message: &str) {
  if io::stderr().is_terminal() {
    let _ = progress_detail_checked(message);
  }
}

fn progress_detail_checked(message: &str) -> io::Result<()> {
  write_progress_line("  ->", message)
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::{HumanMessageBuilder, error_envelope, human_error_hint, success_envelope};
  use crate::commands::CommandOutput;
  use crate::error::JoyError;

  #[test]
  fn json_error_envelope_shape_is_stable() {
    let err = JoyError::new("build", "toolchain_not_found", "No compiler found", 1);
    let value = serde_json::to_value(error_envelope("build", &err)).expect("serialize envelope");

    assert_eq!(
      value,
      json!({
          "ok": false,
          "command": "build",
          "error": {
              "code": "toolchain_not_found",
              "message": "No compiler found"
          }
      })
    );
  }

  #[test]
  fn json_success_envelope_shape_is_stable() {
    let result = CommandOutput::new("recipe-check", "ok", json!({"recipe_count": 9}));
    let value = serde_json::to_value(success_envelope(&result)).expect("serialize envelope");
    assert_eq!(
      value,
      json!({
        "ok": true,
        "command": "recipe-check",
        "data": {
          "recipe_count": 9
        }
      })
    );
  }

  #[test]
  fn human_message_builder_renders_lines_warnings_and_hints() {
    let msg = HumanMessageBuilder::new("Done")
      .kv("project", "demo")
      .line("- mode: debug")
      .warning("joy.lock may be stale")
      .hint("rerun `joy build --update-lock`")
      .build();
    assert_eq!(
      msg,
      "Done\n- project: demo\n- mode: debug\nwarning: joy.lock may be stale\nhint: rerun `joy build --update-lock`"
    );
  }

  #[test]
  fn lockfile_errors_get_human_hint() {
    let err = JoyError::new("build", "lockfile_stale", "joy.lock manifest hash does not match", 1);
    let hint = human_error_hint("build", &err).expect("hint");
    assert!(hint.contains("joy build --update-lock"));
  }
}
