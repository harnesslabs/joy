use thiserror::Error;

/// Structured command error used by CLI command handlers and renderers.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct JoyError {
  pub command: &'static str,
  pub code: &'static str,
  pub message: String,
  pub exit_code: u8,
}

impl JoyError {
  /// Construct a typed command error with a stable machine-readable code and process exit code.
  pub fn new(
    command: &'static str,
    code: &'static str,
    message: impl Into<String>,
    exit_code: u8,
  ) -> Self {
    Self { command, code, message: message.into(), exit_code }
  }

  /// Helper for contextual filesystem errors.
  pub fn io(
    command: &'static str,
    action: &str,
    path: &std::path::Path,
    err: &std::io::Error,
  ) -> Self {
    Self::new(command, "io_error", format!("{action} `{}` failed: {err}", path.display()), 1)
  }
}
