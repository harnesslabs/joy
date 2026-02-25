use serde_json::{Value, json};
use std::env;
use std::fs;

use crate::cli::{MetadataArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::lockfile;
use crate::manifest::Manifest;

pub fn handle(_args: MetadataArgs, _runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("metadata", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "metadata",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }
  let manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("metadata", "manifest_parse_error", err.to_string(), 1))?;

  let joy_root = cwd.join(".joy");
  let state_dir = joy_root.join("state");
  let build_dir = joy_root.join("build");
  let graph_path = state_dir.join("dependency-graph.json");
  let graph = load_json_if_exists(&graph_path)
    .map_err(|err| JoyError::new("metadata", "state_graph_error", err.to_string(), 1))?;

  let lockfile_path = cwd.join("joy.lock");
  let lockfile_info = if lockfile_path.is_file() {
    match lockfile::Lockfile::load(&lockfile_path) {
      Ok(lock) => {
        let mut ids = lock.packages.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        json!({
          "present": true,
          "path": lockfile_path.display().to_string(),
          "version": lock.version,
          "generated_by": lock.generated_by,
          "manifest_hash": lock.manifest_hash,
          "package_count": lock.packages.len(),
          "package_ids": ids,
          "parse_error": Value::Null,
        })
      },
      Err(err) => json!({
        "present": true,
        "path": lockfile_path.display().to_string(),
        "parse_error": err.to_string(),
      }),
    }
  } else {
    json!({
      "present": false,
      "path": lockfile_path.display().to_string(),
      "parse_error": Value::Null,
    })
  };

  let root_compile_db = cwd.join("compile_commands.json");
  let mut target_compile_dbs = Vec::new();
  if build_dir.is_dir() {
    let mut entries = fs::read_dir(&build_dir)
      .map_err(|err| JoyError::io("metadata", "reading build directory", &build_dir, &err))?
      .filter_map(|entry| entry.ok())
      .map(|entry| entry.path())
      .filter(|path| {
        path
          .file_name()
          .and_then(|n| n.to_str())
          .is_some_and(|n| n.starts_with("compile_commands.") && n.ends_with(".json"))
      })
      .map(|path| path.display().to_string())
      .collect::<Vec<_>>();
    entries.sort();
    target_compile_dbs = entries;
  }

  let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
  roots.sort();
  let graph_package_count =
    graph.as_ref().and_then(|v| v.get("packages")).and_then(|v| v.as_array()).map(|arr| arr.len());

  let human = crate::output::HumanMessageBuilder::new("Project metadata")
    .kv("project", cwd.display().to_string())
    .kv("manifest", manifest_path.display().to_string())
    .kv("direct dependencies", roots.len().to_string())
    .kv("graph artifact", graph_path.display().to_string())
    .kv("graph package count", graph_package_count.unwrap_or_default().to_string())
    .kv("lockfile", lockfile_path.display().to_string())
    .kv("root compile db", root_compile_db.display().to_string())
    .kv("target compile db files", target_compile_dbs.len().to_string())
    .build();

  Ok(CommandOutput::new(
    "metadata",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "roots": roots,
      "artifacts": {
        "joy_root": joy_root.display().to_string(),
        "state_dir": state_dir.display().to_string(),
        "build_dir": build_dir.display().to_string(),
        "graph_path": graph_path.display().to_string(),
        "graph_present": graph.is_some(),
        "compile_commands_path": root_compile_db.display().to_string(),
        "compile_commands_present": root_compile_db.is_file(),
        "target_compile_commands": target_compile_dbs,
      },
      "lockfile": lockfile_info,
      "graph": graph,
    }),
  ))
}

fn load_json_if_exists(path: &std::path::Path) -> Result<Option<Value>, std::io::Error> {
  match fs::read(path) {
    Ok(bytes) => {
      let value = serde_json::from_slice(&bytes).map_err(|err| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid json: {err}"))
      })?;
      Ok(Some(value))
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(err) => Err(err),
  }
}
