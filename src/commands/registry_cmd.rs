use serde_json::json;
use std::env;

use crate::cli::{
  RegistryAddArgs, RegistryArgs, RegistryListArgs, RegistryRemoveArgs, RegistrySetDefaultArgs,
  RegistrySubcommand,
};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::output::HumanMessageBuilder;
use crate::registry_config;
use crate::registry_config::RegistryScope;

pub fn handle(args: RegistryArgs) -> Result<CommandOutput, JoyError> {
  match args.command {
    RegistrySubcommand::List(list_args) => handle_list(list_args),
    RegistrySubcommand::Add(add_args) => handle_add(add_args),
    RegistrySubcommand::Remove(remove_args) => handle_remove(remove_args),
    RegistrySubcommand::SetDefault(default_args) => handle_set_default(default_args),
  }
}

fn handle_list(args: RegistryListArgs) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("registry", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let effective = registry_config::load_effective(args.project.then_some(cwd.as_path()))
    .map_err(|err| JoyError::new("registry", "registry_config_error", err.to_string(), 1))?;
  let mut names = effective.registries.keys().cloned().collect::<Vec<_>>();
  names.sort();
  let mut human = HumanMessageBuilder::new("Configured registries");
  human = human.kv("default", effective.default.clone().unwrap_or_else(|| "<unset>".to_string()));
  human = human.kv("count", names.len().to_string());
  for name in &names {
    if let Some(url) = effective.registries.get(name) {
      human = human.line(format!("- {name}: {url}"));
    }
  }
  Ok(CommandOutput::new(
    "registry",
    human.build(),
    json!({
      "scope": if args.project { "project+user" } else { "user" },
      "default": effective.default,
      "registries": names.iter().map(|name| json!({
        "name": name,
        "index": effective.registries.get(name).cloned().unwrap_or_default(),
      })).collect::<Vec<_>>(),
    }),
  ))
}

fn handle_add(args: RegistryAddArgs) -> Result<CommandOutput, JoyError> {
  let (scope, project_root) = command_scope(args.project)?;
  registry_config::set_registry(scope, project_root.as_deref(), &args.name, &args.index)
    .map_err(|err| JoyError::new("registry", "registry_config_error", err.to_string(), 1))?;
  if args.default {
    registry_config::set_default_registry(scope, project_root.as_deref(), &args.name)
      .map_err(|err| JoyError::new("registry", "registry_config_error", err.to_string(), 1))?;
  }
  let human = HumanMessageBuilder::new(format!("Configured registry `{}`", args.name))
    .kv("index", args.index.clone())
    .kv("scope", scope_name(scope).to_string())
    .kv("default", args.default.to_string())
    .build();
  Ok(CommandOutput::new(
    "registry",
    human,
    json!({
      "action": "add",
      "name": args.name,
      "index": args.index,
      "scope": scope_name(scope),
      "default_set": args.default,
    }),
  ))
}

fn handle_remove(args: RegistryRemoveArgs) -> Result<CommandOutput, JoyError> {
  let (scope, project_root) = command_scope(args.project)?;
  let removed = registry_config::remove_registry(scope, project_root.as_deref(), &args.name)
    .map_err(|err| JoyError::new("registry", "registry_config_error", err.to_string(), 1))?;
  let human = HumanMessageBuilder::new(format!("Removed registry `{}`", args.name))
    .kv("scope", scope_name(scope).to_string())
    .kv("removed", removed.to_string())
    .build();
  Ok(CommandOutput::new(
    "registry",
    human,
    json!({
      "action": "remove",
      "name": args.name,
      "scope": scope_name(scope),
      "removed": removed,
    }),
  ))
}

fn handle_set_default(args: RegistrySetDefaultArgs) -> Result<CommandOutput, JoyError> {
  let (scope, project_root) = command_scope(args.project)?;
  registry_config::set_default_registry(scope, project_root.as_deref(), &args.name)
    .map_err(|err| JoyError::new("registry", "registry_config_error", err.to_string(), 1))?;
  let human = HumanMessageBuilder::new(format!("Set default registry to `{}`", args.name))
    .kv("scope", scope_name(scope).to_string())
    .build();
  Ok(CommandOutput::new(
    "registry",
    human,
    json!({
      "action": "set-default",
      "name": args.name,
      "scope": scope_name(scope),
    }),
  ))
}

fn command_scope(project: bool) -> Result<(RegistryScope, Option<std::path::PathBuf>), JoyError> {
  if project {
    let cwd = env::current_dir().map_err(|err| {
      JoyError::new("registry", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
    })?;
    Ok((RegistryScope::Project, Some(cwd)))
  } else {
    Ok((RegistryScope::User, None))
  }
}

fn scope_name(scope: RegistryScope) -> &'static str {
  match scope {
    RegistryScope::User => "user",
    RegistryScope::Project => "project",
  }
}
