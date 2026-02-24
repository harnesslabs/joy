use serde_json::json;
use std::env;
use std::fs;
use std::path::Path;

use crate::cli::{RemoveArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::install_index::InstallIndex;
use crate::manifest::Manifest;
use crate::package_id::PackageId;
use crate::project_env;

pub fn handle(args: RemoveArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  if runtime.frozen {
    return Err(JoyError::new(
      "remove",
      "frozen_disallows_remove",
      "`joy remove` mutates the manifest and cannot run with `--frozen`; rerun without `--frozen`",
      1,
    ));
  }

  let package = PackageId::parse(&args.package)
    .map_err(|err| JoyError::new("remove", "invalid_package_id", err.to_string(), 1))?;
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("remove", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "remove",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }

  let mut manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("remove", "manifest_parse_error", err.to_string(), 1))?;
  let removed = manifest.remove_dependency(package.as_str());
  if removed.is_none() {
    return Err(JoyError::new(
      "remove",
      "dependency_not_found",
      format!("dependency `{}` is not present in `joy.toml`", package),
      1,
    ));
  }

  manifest
    .save(&manifest_path)
    .map_err(|err| JoyError::new("remove", "manifest_write_error", err.to_string(), 1))?;

  let env_layout = project_env::ensure_layout(&cwd)
    .map_err(|err| JoyError::new("remove", "env_setup_failed", err.to_string(), 1))?;
  let header_link_path = env_layout.include_dir.join("deps").join(package.slug());
  let header_link_removed = remove_installed_path_if_exists(&header_link_path)
    .map_err(|err| JoyError::io("remove", "removing installed headers", &header_link_path, &err))?;

  let install_index_path = env_layout.state_dir.join("install-index.json");
  let mut install_index = InstallIndex::load_or_default(&install_index_path)
    .map_err(|err| JoyError::new("remove", "state_index_error", err.to_string(), 1))?;
  install_index.header_links.remove(&header_link_path.display().to_string());
  install_index
    .save(&install_index_path)
    .map_err(|err| JoyError::new("remove", "state_index_error", err.to_string(), 1))?;

  let lockfile_warning = cwd.join("joy.lock").is_file().then_some(
    "joy.lock exists and may be stale after dependency removal; rerun `joy sync --update-lock` or `joy build --update-lock`".to_string(),
  );

  let mut human = format!("Removed dependency `{}`", package);
  if header_link_removed {
    human.push('\n');
    human.push_str(&format!("Removed installed headers at {}", header_link_path.display()));
  }
  if let Some(warning) = &lockfile_warning {
    human.push('\n');
    human.push_str("warning: ");
    human.push_str(warning);
  }

  Ok(CommandOutput::new(
    "remove",
    human,
    json!({
      "package": package.as_str(),
      "removed": true,
      "manifest_path": manifest_path.display().to_string(),
      "project_root": cwd.display().to_string(),
      "header_link_path": header_link_path.display().to_string(),
      "header_link_removed": header_link_removed,
      "state_index_path": install_index_path.display().to_string(),
      "warnings": lockfile_warning.map(|w| vec![w]).unwrap_or_default(),
    }),
  ))
}

fn remove_installed_path_if_exists(path: &Path) -> std::io::Result<bool> {
  match fs::symlink_metadata(path) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)
      } else if metadata.is_dir() {
        fs::remove_dir_all(path)
      } else {
        fs::remove_file(path)
      }?;
      Ok(true)
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
    Err(err) => Err(err),
  }
}
