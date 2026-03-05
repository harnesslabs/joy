use serde_json::json;
use std::collections::BTreeMap;
use std::env;

use crate::cli::{FetchArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::manifest::{DependencySource, Manifest};
use crate::output::HumanMessageBuilder;
use crate::package_id::PackageId;
use crate::registry::{RegistryRequirement, RegistryStore};
use crate::registry_config;

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
  let effective = registry_config::load_effective(Some(&cwd))
    .map_err(|err| JoyError::new("fetch", "registry_config_error", err.to_string(), 1))?;
  let default_registry = effective.default.unwrap_or_else(|| "default".to_string());

  let mut registry_stores = BTreeMap::<String, RegistryStore>::new();
  let mut fetched = Vec::new();
  let mut skipped = Vec::new();

  for (key, spec) in &manifest.dependencies {
    let declared = spec.declared_package(key);
    match spec.source {
      DependencySource::Github => {
        let package = PackageId::parse(declared).map_err(|err| {
          JoyError::new("fetch", "invalid_package_id", format!("dependency `{key}`: {err}"), 1)
        })?;
        let fetched_result = if let Some(version_req) = spec.version.as_deref() {
          fetch::fetch_github_semver(&package, version_req)
            .map_err(|err| JoyError::new("fetch", "fetch_failed", err.to_string(), 1))?
        } else {
          let requested = if spec.rev.trim().is_empty() { "HEAD" } else { spec.rev.as_str() };
          fetch::fetch_github(&package, requested)
            .map_err(|err| JoyError::new("fetch", "fetch_failed", err.to_string(), 1))?
        };
        fetched.push(json!({
          "key": key,
          "package": declared,
          "source": "github",
          "rev": fetched_result.requested_rev,
          "resolved_version": fetched_result.resolved_version,
          "resolved_commit": fetched_result.resolved_commit,
          "cache_hit": fetched_result.cache_hit,
          "source_dir": fetched_result.source_dir.display().to_string(),
        }));
      },
      DependencySource::Registry => {
        let package = PackageId::parse(declared).map_err(|err| {
          JoyError::new("fetch", "invalid_package_id", format!("dependency `{key}`: {err}"), 1)
        })?;
        let Some(version_req) = spec.version.as_deref() else {
          return Err(JoyError::new(
            "fetch",
            "invalid_manifest",
            format!("registry dependency `{key}` must declare `version`"),
            1,
          ));
        };
        let registry_name = spec.registry.clone().unwrap_or_else(|| default_registry.clone());
        let store = if let Some(store) = registry_stores.get(&registry_name) {
          store.clone()
        } else {
          let loaded = RegistryStore::load_named_for_project(&registry_name, Some(&cwd))
            .map_err(|err| JoyError::new("fetch", "registry_load_failed", err.to_string(), 1))?;
          registry_stores.insert(registry_name.clone(), loaded.clone());
          loaded
        };
        let release = store
          .resolve(package.as_str(), RegistryRequirement::Semver(version_req))
          .map_err(|err| JoyError::new("fetch", "registry_load_failed", err.to_string(), 1))?;
        let fetched_result = fetch::fetch_github(&package, &release.source_rev)
          .map_err(|err| JoyError::new("fetch", "fetch_failed", err.to_string(), 1))?;
        fetched.push(json!({
          "key": key,
          "package": declared,
          "source": "registry",
          "registry": registry_name,
          "requested_requirement": release.requested_requirement,
          "resolved_version": release.resolved_version,
          "rev": fetched_result.requested_rev,
          "resolved_commit": fetched_result.resolved_commit,
          "cache_hit": fetched_result.cache_hit,
          "source_dir": fetched_result.source_dir.display().to_string(),
        }));
      },
      DependencySource::Git | DependencySource::Path | DependencySource::Archive => {
        skipped.push(json!({
          "key": key,
          "source": spec.source.as_str(),
          "reason": "fetch for this source backend is not implemented yet",
        }));
      },
    }
  }

  let mut human = HumanMessageBuilder::new("Warmed dependency cache")
    .kv("fetched", fetched.len().to_string())
    .kv("skipped", skipped.len().to_string());
  if !skipped.is_empty() {
    human = human.warning(
      "Some dependencies were skipped because fetch support is not implemented for their source",
    );
  }
  Ok(CommandOutput::new(
    "fetch",
    human.build(),
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "fetched_count": fetched.len(),
      "skipped_count": skipped.len(),
      "fetched": fetched,
      "skipped": skipped,
    }),
  ))
}
