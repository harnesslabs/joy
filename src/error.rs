use thiserror::Error;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct JoyError {
  pub command: &'static str,
  pub code: &'static str,
  pub message: String,
  pub exit_code: u8,
}

impl JoyError {
  pub fn new(
    command: &'static str,
    code: &'static str,
    message: impl Into<String>,
    exit_code: u8,
  ) -> Self {
    Self { command, code, message: message.into(), exit_code }
  }

  pub fn not_implemented(command: &'static str) -> Self {
    Self::new(command, "not_implemented", format!("`joy {command}` is not implemented yet"), 2)
  }
}
