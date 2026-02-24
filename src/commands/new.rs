use crate::cli::NewArgs;
use crate::commands::{CommandOutput, not_implemented};
use crate::error::JoyError;

pub fn handle(_args: NewArgs) -> Result<CommandOutput, JoyError> {
  not_implemented("new")
}
