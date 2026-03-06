use serde_json::json;
use std::env;

use crate::cli::InfoArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::output::HumanMessageBuilder;
use crate::registry::RegistryStore;
use crate::registry_config;

pub fn handle(args: InfoArgs) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("info", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let effective = registry_config::load_effective(Some(&cwd))
    .map_err(|err| JoyError::new("info", "registry_config_error", err.to_string(), 1))?;
  let registry_name =
    args.registry.clone().or(effective.default).unwrap_or_else(|| "default".to_string());
  let store = RegistryStore::load_named_for_project(&registry_name, Some(&cwd))
    .map_err(|err| JoyError::new("info", "registry_load_failed", err.to_string(), 1))?;

  let versions = store.package_versions(&args.package).ok_or_else(|| {
    JoyError::new(
      "info",
      "registry_package_not_found",
      format!("package `{}` not found in registry `{registry_name}`", args.package),
      1,
    )
  })?;
  let latest = versions.first().cloned();
  let human = HumanMessageBuilder::new(format!("Registry info for `{}`", args.package))
    .kv("registry", registry_name.clone())
    .kv("latest", latest.clone().unwrap_or_else(|| "<none>".to_string()))
    .kv("versions", versions.len().to_string())
    .build();
  Ok(CommandOutput::new(
    "info",
    human,
    json!({
      "registry": registry_name,
      "package": args.package,
      "latest_version": latest,
      "versions": versions,
    }),
  ))
}
