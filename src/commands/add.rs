use serde_json::json;
use std::env;

use crate::cli::AddArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::global_cache::GlobalCache;
use crate::linking;
use crate::manifest::{DependencySource, DependencySpec, Manifest};
use crate::package_id::PackageId;
use crate::project_env;

pub fn handle(args: AddArgs) -> Result<CommandOutput, JoyError> {
  let package = PackageId::parse(&args.package)
    .map_err(|err| JoyError::new("add", "invalid_package_id", err.to_string(), 1))?;

  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("add", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "add",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }

  let mut manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("add", "manifest_parse_error", err.to_string(), 1))?;

  let rev = args.rev.unwrap_or_else(|| "HEAD".to_string());
  let env_layout = project_env::ensure_layout(&cwd)
    .map_err(|err| JoyError::new("add", "env_setup_failed", err.to_string(), 1))?;

  let cache = GlobalCache::resolve()
    .map_err(|err| JoyError::new("add", "cache_setup_failed", err.to_string(), 1))?;
  let fetched = fetch::fetch_github_with_cache(&package, &rev, &cache)
    .map_err(|err| JoyError::new("add", "fetch_failed", err.to_string(), 1))?;
  let installed = linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
    .map_err(|err| JoyError::new("add", "header_install_failed", err.to_string(), 1))?;

  let changed = manifest.add_dependency(
    package.as_str().to_string(),
    DependencySpec { source: DependencySource::Github, rev: rev.clone() },
  );
  if changed {
    manifest
      .save(&manifest_path)
      .map_err(|err| JoyError::new("add", "manifest_write_error", err.to_string(), 1))?;
  }

  let lockfile_warning = cwd.join("joy.lock").is_file().then_some(
    "joy.lock exists and may be stale; future builds should refresh the lockfile".to_string(),
  );
  let mut human_message = if changed {
    format!("Added dependency `{}` (rev `{rev}`)", args.package)
  } else {
    format!("Dependency `{}` already present with rev `{rev}`", args.package)
  };
  human_message.push('\n');
  human_message.push_str(&format!(
    "Fetched `{}` at {} and installed headers to {}",
    package,
    fetched.resolved_commit,
    installed.link_path.display()
  ));
  if let Some(warning) = &lockfile_warning {
    human_message.push('\n');
    human_message.push_str("warning: ");
    human_message.push_str(warning);
  }

  let created_env_paths: Vec<String> =
    env_layout.created_paths.iter().map(|path| path.display().to_string()).collect();

  Ok(CommandOutput::new(
    "add",
    human_message,
    json!({
      "package": args.package,
      "rev": rev,
      "changed": changed,
      "resolved_commit": fetched.resolved_commit,
      "remote_url": fetched.remote_url,
      "cache_source_dir": fetched.source_dir.display().to_string(),
      "cache_hit": fetched.cache_hit,
      "header_root": installed.header_root.display().to_string(),
      "header_link_path": installed.link_path.display().to_string(),
      "header_link_kind": installed.link_kind,
      "manifest_path": manifest_path.display().to_string(),
      "project_root": cwd.display().to_string(),
      "created_env_paths": created_env_paths,
      "warnings": lockfile_warning.map(|w| vec![w]).unwrap_or_default(),
    }),
  ))
}
