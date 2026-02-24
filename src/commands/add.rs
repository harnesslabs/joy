use serde_json::json;
use std::env;
use std::fs;
use std::path::Path;

use crate::cli::{AddArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::global_cache::GlobalCache;
use crate::install_index::InstallIndex;
use crate::linking;
use crate::manifest::{DependencySource, DependencySpec, Manifest};
use crate::output::{HumanMessageBuilder, progress_detail, progress_stage};
use crate::package_id::PackageId;
use crate::project_env;

pub fn handle(args: AddArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  if runtime.frozen {
    return Err(JoyError::new(
      "add",
      "frozen_disallows_add",
      "`joy add` mutates the manifest and cannot run with `--frozen`; rerun without `--frozen`",
      1,
    ));
  }

  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });

  if runtime.progress {
    progress_stage(&format!("Resolving and fetching `{}`", args.package));
  }

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
    .map_err(|err| map_fetch_error("add", err))?;
  if runtime.progress {
    progress_detail(&format!("Installing headers for `{package}`"));
  }
  let installed = linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
    .map_err(|err| JoyError::new("add", "header_install_failed", err.to_string(), 1))?;

  let changed = manifest.add_dependency(
    package.as_str().to_string(),
    DependencySpec { source: DependencySource::Github, rev: rev.clone() },
  );
  if changed && let Err(err) = manifest.save(&manifest_path) {
    let rollback_err = remove_installed_header_path(&installed.link_path);
    let mut message = err.to_string();
    if let Err(clean_err) = rollback_err {
      message.push_str(&format!(
        "\nrollback failed: could not remove installed headers at `{}`: {clean_err}",
        installed.link_path.display()
      ));
    }
    return Err(JoyError::new("add", "manifest_write_error", message, 1));
  }

  let lockfile_warning = cwd.join("joy.lock").is_file().then_some(
    "joy.lock exists and may be stale; future builds should refresh the lockfile".to_string(),
  );

  let install_index_path = env_layout.state_dir.join("install-index.json");
  let mut install_index = InstallIndex::load_or_default(&install_index_path)
    .map_err(|err| JoyError::new("add", "state_index_error", err.to_string(), 1))?;
  install_index.header_links.insert(installed.link_path.display().to_string());
  install_index
    .save(&install_index_path)
    .map_err(|err| JoyError::new("add", "state_index_error", err.to_string(), 1))?;

  let mut human_builder = if changed {
    HumanMessageBuilder::new(format!("Added dependency `{}`", args.package))
  } else {
    HumanMessageBuilder::new(format!("Dependency `{}` already present", args.package))
  }
  .kv("requested rev", format!("`{rev}`"))
  .kv("resolved commit", fetched.resolved_commit.clone())
  .kv("headers installed", installed.link_path.display().to_string());
  if let Some(warning) = &lockfile_warning {
    human_builder = human_builder.warning(warning.clone());
  }
  let human_message = human_builder.build();

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
      "state_index_path": install_index_path.display().to_string(),
      "warnings": lockfile_warning.map(|w| vec![w]).unwrap_or_default(),
    }),
  ))
}

fn map_fetch_error(command: &'static str, err: fetch::FetchError) -> JoyError {
  let code = if err.is_offline_cache_miss() {
    "offline_cache_miss"
  } else if err.is_offline_network_disabled() {
    "offline_network_disabled"
  } else {
    "fetch_failed"
  };
  JoyError::new(command, code, err.to_string(), 1)
}

fn remove_installed_header_path(path: &Path) -> std::io::Result<()> {
  match fs::symlink_metadata(path) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).or_else(|err| {
          if matches!(
            err.kind(),
            std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::IsADirectory
          ) {
            fs::remove_dir(path)
          } else {
            Err(err)
          }
        })
      } else if metadata.is_dir() {
        fs::remove_dir_all(path)
      } else {
        fs::remove_file(path)
      }
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(err) => Err(err),
  }
}
