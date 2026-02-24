use serde::Serialize;
use std::env;

use crate::cli::InitArgs;
use crate::commands::{CommandOutput, scaffold_files};
use crate::error::JoyError;

#[derive(Debug, Serialize)]
struct InitResponse {
  project_name: String,
  project_root: String,
  created_paths: Vec<String>,
  overwritten_paths: Vec<String>,
}

pub fn handle(args: InitArgs) -> Result<CommandOutput, JoyError> {
  let root = env::current_dir().map_err(|err| {
    JoyError::new("init", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let project_name = root
    .file_name()
    .and_then(|name| name.to_str())
    .filter(|name| !name.is_empty())
    .unwrap_or("joy-project")
    .to_string();

  let manifest_path = root.join("joy.toml");
  if manifest_path.exists() && !args.force {
    return Err(JoyError::new(
      "init",
      "path_exists",
      format!("refusing to overwrite existing path `{}` (use --force)", manifest_path.display()),
      1,
    ));
  }

  let summary = scaffold_files("init", &root, &project_name, args.force)?;
  let created_paths: Vec<String> =
    summary.created.iter().map(|path| path.display().to_string()).collect();
  let overwritten_paths: Vec<String> =
    summary.overwritten.iter().map(|path| path.display().to_string()).collect();

  CommandOutput::from_data(
    "init",
    format!("Initialized joy project `{project_name}` in {}", summary.root.display()),
    &InitResponse {
      project_name,
      project_root: summary.root.display().to_string(),
      created_paths,
      overwritten_paths,
    },
  )
}
