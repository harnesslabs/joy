use crate::cli::RunArgs;
use crate::commands::{CommandOutput, not_implemented};
use crate::error::JoyError;

pub fn handle(_args: RunArgs) -> Result<CommandOutput, JoyError> {
  not_implemented("run")
}
