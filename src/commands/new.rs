use serde_json::json;
use std::env;
use std::fs;
use std::path::Path;

use crate::cli::NewArgs;
use crate::commands::{CommandOutput, dir_is_empty, scaffold_files};
use crate::error::JoyError;

pub fn handle(args: NewArgs) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("new", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let root = cwd.join(&args.name);
  let project_name = project_name_from_root(&root);

  if root.exists() {
    let metadata =
      fs::metadata(&root).map_err(|err| JoyError::io("new", "reading metadata", &root, &err))?;
    if !metadata.is_dir() {
      return Err(JoyError::new(
        "new",
        "invalid_target",
        format!("target `{}` exists and is not a directory", root.display()),
        1,
      ));
    }

    let empty = dir_is_empty(&root)?;
    if !empty && !args.force {
      return Err(JoyError::new(
        "new",
        "non_empty_directory",
        format!("target directory `{}` is not empty (use --force)", root.display()),
        1,
      ));
    }
  }

  let summary = scaffold_files("new", &root, &project_name, args.force)?;
  let created_paths: Vec<String> =
    summary.created.iter().map(|path| path.display().to_string()).collect();
  let overwritten_paths: Vec<String> =
    summary.overwritten.iter().map(|path| path.display().to_string()).collect();

  Ok(CommandOutput::new(
    "new",
    format!("Created joy project `{}` at {}", project_name, summary.root.display()),
    json!({
      "project_name": project_name,
      "project_root": summary.root.display().to_string(),
      "created_paths": created_paths,
      "overwritten_paths": overwritten_paths
    }),
  ))
}

fn project_name_from_root(root: &Path) -> String {
  root
    .file_name()
    .map(|name| name.to_string_lossy().into_owned())
    .filter(|name| !name.trim().is_empty())
    .unwrap_or_else(|| root.to_string_lossy().into_owned())
}
