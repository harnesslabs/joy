use crate::cli::BuildArgs;
use crate::commands::{CommandOutput, not_implemented};
use crate::error::JoyError;

pub fn handle(_args: BuildArgs) -> Result<CommandOutput, JoyError> {
  not_implemented("build")
}
