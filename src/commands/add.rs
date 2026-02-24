use crate::cli::AddArgs;
use crate::commands::{CommandOutput, not_implemented};
use crate::error::JoyError;

pub fn handle(_args: AddArgs) -> Result<CommandOutput, JoyError> {
  not_implemented("add")
}
