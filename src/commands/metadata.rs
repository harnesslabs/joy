use serde_json::{Value, json};
use std::env;

use crate::cli::{MetadataArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::manifest::Manifest;
use crate::project_probe;

pub fn handle(_args: MetadataArgs, _runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("metadata", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let probe = project_probe::probe(&cwd);

  if !probe.manifest_path.is_file() {
    return Err(JoyError::new(
      "metadata",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", probe.manifest_path.display()),
      1,
    ));
  }
  let manifest = Manifest::load(&probe.manifest_path)
    .map_err(|err| JoyError::new("metadata", "manifest_parse_error", err.to_string(), 1))?;
  if let Some(parse_error) = &probe.dependency_graph.parse_error {
    return Err(JoyError::new("metadata", "state_graph_error", parse_error.clone(), 1));
  }

  let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
  roots.sort();
  let graph_package_count = probe.dependency_graph_package_count.unwrap_or_default();

  let lockfile_info = if !probe.lockfile.present {
    json!({
      "present": false,
      "path": probe.lockfile.path.display().to_string(),
      "parse_error": Value::Null,
    })
  } else if let Some(parse_error) = &probe.lockfile.parse_error {
    json!({
      "present": true,
      "path": probe.lockfile.path.display().to_string(),
      "parse_error": parse_error,
    })
  } else {
    json!({
      "present": true,
      "path": probe.lockfile.path.display().to_string(),
      "version": probe.lockfile.version,
      "generated_by": probe.lockfile.generated_by.clone(),
      "manifest_hash": probe.lockfile.manifest_hash.clone(),
      "package_count": probe.lockfile.package_count,
      "package_ids": probe.lockfile.package_ids.clone(),
      "parse_error": Value::Null,
    })
  };

  let human = crate::output::HumanMessageBuilder::new("Project metadata")
    .kv("project", probe.project_root.display().to_string())
    .kv("manifest", probe.manifest_path.display().to_string())
    .kv("direct dependencies", roots.len().to_string())
    .kv("graph artifact", probe.dependency_graph.path.display().to_string())
    .kv("graph package count", graph_package_count.to_string())
    .kv("lockfile", probe.lockfile.path.display().to_string())
    .kv("root compile db", probe.root_compile_commands.path.display().to_string())
    .kv("target compile db files", probe.target_compile_commands.len().to_string())
    .build();

  Ok(CommandOutput::new(
    "metadata",
    human,
    json!({
      "project_root": probe.project_root.display().to_string(),
      "manifest_path": probe.manifest_path.display().to_string(),
      "roots": roots,
      "artifacts": {
        "joy_root": probe.joy_root.display().to_string(),
        "state_dir": probe.state_dir.display().to_string(),
        "build_dir": probe.build_dir.display().to_string(),
        "graph_path": probe.dependency_graph.path.display().to_string(),
        "graph_present": probe.dependency_graph.present,
        "compile_commands_path": probe.root_compile_commands.path.display().to_string(),
        "compile_commands_present": probe.root_compile_commands.present,
        "target_compile_commands": probe.target_compile_commands,
      },
      "lockfile": lockfile_info,
      "graph": probe.dependency_graph_json,
    }),
  ))
}
