use serde_json::json;
use std::env;

use crate::cli::{RuntimeFlags, UpdateArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::global_cache::GlobalCache;
use crate::install_index::InstallIndex;
use crate::linking;
use crate::manifest::{DependencySource, DependencySpec, Manifest};
use crate::output::{HumanMessageBuilder, progress_detail};
use crate::package_id::PackageId;
use crate::project_env;

#[derive(Debug)]
struct UpdatedDependency {
  package: String,
  requested_rev: String,
  resolved_commit: String,
  cache_hit: bool,
  header_link_path: String,
}

pub fn handle(args: UpdateArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  if runtime.frozen {
    return Err(JoyError::new(
      "update",
      "frozen_disallows_update",
      "`joy update` may mutate the manifest and cannot run with `--frozen`; rerun without `--frozen`",
      1,
    ));
  }
  if args.rev.is_some() && args.package.is_none() {
    return Err(JoyError::new(
      "update",
      "invalid_update_args",
      "`--rev` requires a specific package (`joy update <package> --rev <rev>`)",
      1,
    ));
  }

  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });

  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("update", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "update",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }

  let mut manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("update", "manifest_parse_error", err.to_string(), 1))?;
  let env_layout = project_env::ensure_layout(&cwd)
    .map_err(|err| JoyError::new("update", "env_setup_failed", err.to_string(), 1))?;
  let cache = GlobalCache::resolve()
    .map_err(|err| JoyError::new("update", "cache_setup_failed", err.to_string(), 1))?;

  if manifest.dependencies.is_empty() {
    return Ok(CommandOutput::new(
      "update",
      "No dependencies to update",
      json!({
        "project_root": cwd.display().to_string(),
        "manifest_path": manifest_path.display().to_string(),
        "updated": [],
        "updated_count": 0,
        "manifest_changed": false,
        "warnings": [],
      }),
    ));
  }

  let targets = if let Some(package) = args.package.as_ref() {
    if !manifest.dependencies.contains_key(package) {
      return Err(JoyError::new(
        "update",
        "dependency_not_found",
        format!("dependency `{package}` is not present in `joy.toml`"),
        1,
      ));
    }
    vec![package.clone()]
  } else {
    manifest.dependencies.keys().cloned().collect::<Vec<_>>()
  };

  let mut manifest_changed = false;
  let mut updated = Vec::new();

  for package_raw in targets {
    if runtime.progress {
      progress_detail(&format!("Refreshing `{package_raw}`"));
    }
    let package = PackageId::parse(&package_raw)
      .map_err(|err| JoyError::new("update", "invalid_package_id", err.to_string(), 1))?;
    let spec = manifest.dependencies.get(package.as_str()).cloned().ok_or_else(|| {
      JoyError::new(
        "update",
        "dependency_not_found",
        format!("dependency `{}` is not present in `joy.toml`", package.as_str()),
        1,
      )
    })?;
    if !matches!(spec.source, DependencySource::Github) {
      return Err(JoyError::new(
        "update",
        "unsupported_dependency_source",
        format!("unsupported dependency source for `{}`", package),
        1,
      ));
    }

    let requested_rev = if let Some(rev) = args.rev.as_ref() {
      rev.clone()
    } else if spec.rev.trim().is_empty() {
      "HEAD".to_string()
    } else {
      spec.rev.clone()
    };

    let fetched = fetch::fetch_github_with_cache(&package, &requested_rev, &cache)
      .map_err(|err| map_fetch_error("update", err))?;
    let installed =
      linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
        .map_err(|err| JoyError::new("update", "header_install_failed", err.to_string(), 1))?;

    if spec.rev != requested_rev {
      manifest_changed |= manifest.add_dependency(
        package.as_str().to_string(),
        DependencySpec { source: DependencySource::Github, rev: requested_rev.clone() },
      );
    }

    updated.push(UpdatedDependency {
      package: package.as_str().to_string(),
      requested_rev,
      resolved_commit: fetched.resolved_commit,
      cache_hit: fetched.cache_hit,
      header_link_path: installed.link_path.display().to_string(),
    });
  }

  if manifest_changed {
    manifest
      .save(&manifest_path)
      .map_err(|err| JoyError::new("update", "manifest_write_error", err.to_string(), 1))?;
  }

  let install_index_path = env_layout.state_dir.join("install-index.json");
  let mut install_index = InstallIndex::load_or_default(&install_index_path)
    .map_err(|err| JoyError::new("update", "state_index_error", err.to_string(), 1))?;
  for item in &updated {
    install_index.header_links.insert(item.header_link_path.clone());
  }
  install_index
    .save(&install_index_path)
    .map_err(|err| JoyError::new("update", "state_index_error", err.to_string(), 1))?;

  updated.sort_by(|a, b| a.package.cmp(&b.package));

  let lockfile_warning = cwd.join("joy.lock").is_file().then_some(
    "joy.lock exists and may be stale after dependency updates; rerun `joy sync --update-lock` or `joy build --update-lock`".to_string(),
  );

  let mut human_builder = if updated.is_empty() {
    HumanMessageBuilder::new("No dependencies updated")
  } else if manifest_changed {
    HumanMessageBuilder::new(format!("Updated {} dependency entries", updated.len()))
  } else {
    HumanMessageBuilder::new(format!("Refreshed {} dependency header installs", updated.len()))
  }
  .kv("updated count", updated.len().to_string())
  .kv("manifest changed", manifest_changed.to_string());
  if let Some(warning) = &lockfile_warning {
    human_builder = human_builder.warning(warning.clone());
  }
  let human = human_builder.build();

  Ok(CommandOutput::new(
    "update",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "state_index_path": install_index_path.display().to_string(),
      "manifest_changed": manifest_changed,
      "updated_count": updated.len(),
      "updated": updated.iter().map(|item| json!({
        "package": item.package,
        "rev": item.requested_rev,
        "resolved_commit": item.resolved_commit,
        "cache_hit": item.cache_hit,
        "header_link_path": item.header_link_path,
      })).collect::<Vec<_>>(),
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
