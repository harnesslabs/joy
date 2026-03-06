use crate::cli::{RuntimeFlags, SyncArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::output::{HumanMessageBuilder, progress_stage};

use super::build;

pub fn handle(args: SyncArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  if runtime.progress {
    progress_stage("Synchronizing dependencies and lockfile");
  }
  let execution = build::sync_project(build::BuildOptions {
    release: args.release,
    target: None,
    locked: args.locked || runtime.frozen,
    update_lock: args.update_lock,
    offline: runtime.offline,
    progress: runtime.progress,
    workspace_root: runtime.workspace_root.clone(),
    workspace_member: runtime.workspace_member.clone(),
  })?;

  let human_message = if let Some(toolchain) = &execution.toolchain {
    HumanMessageBuilder::new("Synchronized dependencies and lockfile")
      .kv("project", execution.project_root.display().to_string())
      .kv(
        "toolchain",
        format!("{} {}", toolchain.compiler.kind.as_str(), toolchain.compiler.version),
      )
      .kv("compiled dependencies built", execution.compiled_dependencies_built.len().to_string())
      .kv("lockfile updated", execution.lockfile_updated.to_string())
      .build()
  } else {
    HumanMessageBuilder::new("Synchronized dependencies and lockfile")
      .kv("project", execution.project_root.display().to_string())
      .kv("compiled dependencies built", execution.compiled_dependencies_built.len().to_string())
      .kv("lockfile updated", execution.lockfile_updated.to_string())
      .build()
  };

  Ok(CommandOutput::new("sync", human_message, execution.json_data()))
}
