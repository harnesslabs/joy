use crate::cli::RunArgs;
use crate::commands::{CommandOutput, ensure_local_env_if_manifest_present, not_implemented};
use crate::error::JoyError;

pub fn handle(_args: RunArgs) -> Result<CommandOutput, JoyError> {
  ensure_local_env_if_manifest_present("run")?;
  not_implemented("run")
}
