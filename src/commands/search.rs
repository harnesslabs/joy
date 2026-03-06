use serde_json::json;
use std::env;

use crate::cli::SearchArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::output::HumanMessageBuilder;
use crate::registry::RegistryStore;
use crate::registry_config;

pub fn handle(args: SearchArgs) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("search", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let effective = registry_config::load_effective(Some(&cwd))
    .map_err(|err| JoyError::new("search", "registry_config_error", err.to_string(), 1))?;
  let registry_name =
    args.registry.clone().or(effective.default).unwrap_or_else(|| "default".to_string());
  let store = RegistryStore::load_named_for_project(&registry_name, Some(&cwd))
    .map_err(|err| JoyError::new("search", "registry_load_failed", err.to_string(), 1))?;
  let query = args.query.to_ascii_lowercase();
  let mut matches = store
    .package_ids()
    .filter(|id| id.to_ascii_lowercase().contains(&query))
    .map(|id| {
      let latest = store.package_versions(id).and_then(|versions| versions.first().cloned());
      (id.to_string(), latest)
    })
    .collect::<Vec<_>>();
  matches.sort_by(|a, b| a.0.cmp(&b.0));
  if matches.len() > args.limit {
    matches.truncate(args.limit);
  }

  let mut human = HumanMessageBuilder::new(format!("Registry search results for `{}`", args.query))
    .kv("registry", registry_name.clone())
    .kv("matches", matches.len().to_string());
  for (id, latest) in &matches {
    human = human.line(format!(
      "- {}{}",
      id,
      latest.as_deref().map(|v| format!(" ({v})")).unwrap_or_default()
    ));
  }

  Ok(CommandOutput::new(
    "search",
    human.build(),
    json!({
      "query": args.query,
      "registry": registry_name,
      "count": matches.len(),
      "packages": matches.into_iter().map(|(id, latest)| json!({
        "id": id,
        "latest_version": latest,
      })).collect::<Vec<_>>(),
    }),
  ))
}
