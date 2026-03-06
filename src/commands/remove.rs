use serde_json::json;
use std::env;

use super::dependency_common::normalize_dependency_arg;
use crate::cli::{RemoveArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fs_ops;
use crate::install_index::InstallIndex;
use crate::manifest::Manifest;
use crate::output::HumanMessageBuilder;
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

  let package_arg = normalize_dependency_arg(&args.package);
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
  let key = manifest.resolve_dependency_key(&package_arg).ok_or_else(|| {
    JoyError::new(
      "remove",
      "dependency_not_found",
      format!("dependency `{}` is not present in `joy.toml`", package_arg),
      1,
    )
  })?;
  let removed = manifest.remove_dependency(&key);
  let Some(removed_spec) = removed else {
    return Err(JoyError::new(
      "remove",
      "dependency_not_found",
      format!("dependency `{}` is not present in `joy.toml`", package_arg),
      1,
    ));
  };

  manifest
    .save(&manifest_path)
    .map_err(|err| JoyError::new("remove", "manifest_write_error", err.to_string(), 1))?;

  let env_layout = project_env::ensure_layout(&cwd)
    .map_err(|err| JoyError::new("remove", "env_setup_failed", err.to_string(), 1))?;
  let resolved_package = removed_spec.declared_package(&key).to_string();
  let header_link_path = resolve_package_id_for_remove(&key, &removed_spec)
    .ok()
    .map(|package| env_layout.include_dir.join("deps").join(package.slug()));
  let header_link_removed = if let Some(path) = header_link_path.as_ref() {
    fs_ops::remove_path_if_exists(path)
      .map_err(|err| JoyError::io("remove", "removing installed headers", path, &err))?
  } else {
    false
  };

  let install_index_path = env_layout.state_dir.join("install-index.json");
  let mut install_index = InstallIndex::load_or_default(&install_index_path)
    .map_err(|err| JoyError::new("remove", "state_index_error", err.to_string(), 1))?;
  if let Some(path) = header_link_path.as_ref() {
    install_index.header_links.remove(&path.display().to_string());
  }
  install_index
    .save(&install_index_path)
    .map_err(|err| JoyError::new("remove", "state_index_error", err.to_string(), 1))?;

  let lockfile_warning = cwd.join("joy.lock").is_file().then_some(
    "joy.lock exists and may be stale after dependency removal; rerun `joy sync --update-lock` or `joy build --update-lock`".to_string(),
  );

  let mut human_builder = HumanMessageBuilder::new(format!("Removed dependency `{}`", key))
    .kv("manifest", manifest_path.display().to_string())
    .kv(
      "header link",
      header_link_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".to_string()),
    )
    .kv("header link removed", header_link_removed.to_string());
  if let Some(warning) = &lockfile_warning {
    human_builder = human_builder.warning(warning.clone());
  }
  let human = human_builder.build();

  let mut data = serde_json::Map::new();
  data.insert(
    "header_link_path".to_string(),
    json!(header_link_path.as_ref().map(|p| p.display().to_string())),
  );
  data.insert("header_link_removed".to_string(), json!(header_link_removed));
  data.insert("manifest_path".to_string(), json!(manifest_path.display().to_string()));
  data.insert("package".to_string(), json!(resolved_package));
  data.insert("project_root".to_string(), json!(cwd.display().to_string()));
  data.insert("registry".to_string(), json!(removed_spec.registry.clone()));
  data.insert("removed".to_string(), json!(true));
  data.insert("source".to_string(), json!(removed_spec.source.as_str()));
  data.insert("source_package".to_string(), json!(removed_spec.package.clone()));
  data.insert("state_index_path".to_string(), json!(install_index_path.display().to_string()));
  data.insert(
    "warnings".to_string(),
    json!(lockfile_warning.clone().map(|w| vec![w]).unwrap_or_default()),
  );

  Ok(CommandOutput::new("remove", human, serde_json::Value::Object(data)))
}

fn resolve_package_id_for_remove(
  key: &str,
  spec: &crate::manifest::DependencySpec,
) -> Result<PackageId, crate::package_id::PackageIdError> {
  if let Ok(package) = PackageId::parse(spec.declared_package(key)) {
    return Ok(package);
  }
  let mut slug = String::new();
  for ch in key.chars() {
    if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
      slug.push(ch);
    } else {
      slug.push('_');
    }
  }
  let slug = slug.trim_matches('_');
  let slug = if slug.is_empty() { "dep" } else { slug };
  PackageId::parse(&format!("local/{slug}"))
}
