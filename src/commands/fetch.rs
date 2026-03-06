use serde_json::json;
use std::env;

use crate::cli::{FetchArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::manifest::Manifest;
use crate::output::HumanMessageBuilder;
use crate::recipes::RecipeStore;
use crate::resolver;

use super::graph_common::map_resolver_error;

pub fn handle(_args: FetchArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("fetch", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "fetch",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }
  let manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("fetch", "manifest_parse_error", err.to_string(), 1))?;
  let recipes = RecipeStore::load_default()
    .map_err(|err| JoyError::new("fetch", "recipe_load_failed", err.to_string(), 1))?;
  let resolved = resolver::resolve_manifest(&manifest, &recipes)
    .map_err(|err| map_resolver_error("fetch", err))?;

  let mut fetched = Vec::new();
  for pkg in resolved.packages() {
    let source_provenance =
      resolved.source_provenance(pkg.id.as_str()).cloned().unwrap_or_default();
    let fetched_result = match pkg.source {
      crate::manifest::DependencySource::Github | crate::manifest::DependencySource::Registry => {
        fetch::fetch_github(&pkg.id, &pkg.requested_rev)
          .map_err(|err| JoyError::new("fetch", "fetch_failed", err.to_string(), 1))?
      },
      crate::manifest::DependencySource::Git => {
        let source_git = source_provenance.source_git.as_deref().ok_or_else(|| {
          JoyError::new(
            "fetch",
            "invalid_dependency_source",
            format!("resolved git dependency `{}` is missing `source_git`", pkg.id),
            1,
          )
        })?;
        fetch::fetch_git(&pkg.id, source_git, &pkg.requested_rev)
          .map_err(|err| JoyError::new("fetch", "fetch_failed", err.to_string(), 1))?
      },
      crate::manifest::DependencySource::Path => {
        let source_path = source_provenance.source_path.as_deref().ok_or_else(|| {
          JoyError::new(
            "fetch",
            "invalid_dependency_source",
            format!("resolved path dependency `{}` is missing `source_path`", pkg.id),
            1,
          )
        })?;
        fetch::fetch_path(&pkg.id, source_path)
          .map_err(|err| JoyError::new("fetch", "fetch_failed", err.to_string(), 1))?
      },
      crate::manifest::DependencySource::Archive => {
        let source_url = source_provenance.source_url.as_deref().ok_or_else(|| {
          JoyError::new(
            "fetch",
            "invalid_dependency_source",
            format!("resolved archive dependency `{}` is missing `source_url`", pkg.id),
            1,
          )
        })?;
        let sha256 = source_provenance.source_checksum_sha256.as_deref().ok_or_else(|| {
          JoyError::new(
            "fetch",
            "invalid_dependency_source",
            format!("resolved archive dependency `{}` is missing `source_checksum_sha256`", pkg.id),
            1,
          )
        })?;
        fetch::fetch_archive(&pkg.id, source_url, sha256)
          .map_err(|err| JoyError::new("fetch", "fetch_failed", err.to_string(), 1))?
      },
    };
    fetched.push(json!({
      "id": pkg.id.to_string(),
      "source": pkg.source.as_str(),
      "registry": pkg.registry,
      "source_package": pkg.source_package,
      "requested_requirement": pkg.requested_requirement,
      "resolved_version": pkg.resolved_version,
      "rev": fetched_result.requested_rev,
      "resolved_commit": fetched_result.resolved_commit,
      "cache_hit": fetched_result.cache_hit,
      "source_dir": fetched_result.source_dir.display().to_string(),
      "source_git": source_provenance.source_git,
      "source_path": source_provenance.source_path,
      "source_url": source_provenance.source_url,
      "source_checksum_sha256": source_provenance.source_checksum_sha256,
    }));
  }

  let human = HumanMessageBuilder::new("Warmed dependency cache")
    .kv("fetched", fetched.len().to_string())
    .build();
  Ok(CommandOutput::new(
    "fetch",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "fetched_count": fetched.len(),
      "fetched": fetched,
      "skipped_count": 0,
      "skipped": [],
    }),
  ))
}
