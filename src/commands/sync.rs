use crate::cli::{RuntimeFlags, SyncArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;

use super::build;

pub fn handle(args: SyncArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let execution = build::sync_project(build::BuildOptions {
    release: args.release,
    locked: args.locked || runtime.frozen,
    update_lock: args.update_lock,
    offline: runtime.offline,
  })?;

  let human_message = if let Some(toolchain) = &execution.toolchain {
    format!(
      "Synchronized dependencies and lockfile for `{}` using {} {}",
      execution.project_root.display(),
      toolchain.compiler.kind.as_str(),
      toolchain.compiler.version
    )
  } else {
    format!("Synchronized dependencies and lockfile for `{}`", execution.project_root.display())
  };

  Ok(CommandOutput::new("sync", human_message, execution.json_data()))
}
