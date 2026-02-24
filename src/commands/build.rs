use crate::cli::BuildArgs;
use crate::commands::{CommandOutput, ensure_local_env_if_manifest_present, not_implemented};
use crate::error::JoyError;

pub fn handle(_args: BuildArgs) -> Result<CommandOutput, JoyError> {
  ensure_local_env_if_manifest_present("build")?;
  not_implemented("build")
}
