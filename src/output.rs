use serde::Serialize;
use serde_json::Value;
use std::io::{self, Write};

use crate::commands::CommandOutput;
use crate::error::JoyError;

/// Output mode selected by CLI flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
  Human,
  Json,
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
    OutputMode::Human => {
      println!("{}", result.human_message);
      Ok(())
    },
    OutputMode::Json => write_json(
      &mut io::stdout(),
      &SuccessEnvelope { ok: true, command: result.command, data: &result.data },
    ),
  }
}

/// Render a command error in human or machine-readable form.
///
/// JSON mode writes to stdout intentionally so callers can treat all command output as a single
/// stream while still relying on process exit codes for success/failure.
pub fn print_error(mode: OutputMode, command: &'static str, err: &JoyError) -> io::Result<()> {
  match mode {
    OutputMode::Human => {
      eprintln!("error[{code}]: {message}", code = err.code, message = err.message);
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

// TODO(phase7): Consolidate command success/error JSON envelope shaping in one module so command
// handlers can return typed payloads instead of raw `serde_json::Value`.
fn error_envelope<'a>(command: &'a str, err: &'a JoyError) -> ErrorEnvelope<'a> {
  ErrorEnvelope {
    ok: false,
    command,
    error: ErrorPayload { code: err.code, message: &err.message },
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::error_envelope;
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
}
