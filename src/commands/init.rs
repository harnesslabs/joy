use crate::cli::InitArgs;
use crate::commands::{CommandOutput, not_implemented};
use crate::error::JoyError;

pub fn handle(_args: InitArgs) -> Result<CommandOutput, JoyError> {
  not_implemented("init")
}
