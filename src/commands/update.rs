use serde_json::json;
use std::env;

use super::dependency_common::{map_fetch_error, map_registry_error, parse_dependency_input};
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
use crate::registry::{RegistryRequirement, RegistryStore};

#[derive(Debug)]
struct UpdatedDependency {
  key: String,
  package: Option<String>,
  source: String,
  registry: Option<String>,
  source_package: Option<String>,
  requested_rev: Option<String>,
  requested_requirement: Option<String>,
  resolved_version: Option<String>,
  resolved_commit: Option<String>,
  cache_hit: Option<bool>,
  header_link_path: Option<String>,
  staged_only: bool,
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
  if args.version.is_some() && args.package.is_none() {
    return Err(JoyError::new(
      "update",
      "invalid_update_args",
      "`--version` requires a specific package (`joy update <package> --version <range>`)",
      1,
    ));
  }
  if args.rev.is_some() && args.version.is_some() {
    return Err(JoyError::new(
      "update",
      "invalid_update_args",
      "`--rev` and `--version` are mutually exclusive; choose one dependency requirement style",
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

  let targets = if let Some(raw_target) = args.package.as_deref() {
    let parsed = parse_dependency_input("update", raw_target)?;
    let selector = parsed.reference;
    let key = manifest.resolve_dependency_key(&selector).ok_or_else(|| {
      JoyError::new(
        "update",
        "dependency_not_found",
        format!("dependency `{selector}` is not present in `joy.toml`"),
        1,
      )
    })?;
    vec![key]
  } else {
    manifest.dependencies.keys().cloned().collect::<Vec<_>>()
  };

  let mut manifest_changed = false;
  let mut updated = Vec::new();
  let mut registry_store = None::<RegistryStore>;
  let mut warnings = Vec::<String>::new();

  for key in targets {
    if runtime.progress {
      progress_detail(&format!("Refreshing `{key}`"));
    }
    let spec = manifest.dependencies.get(&key).cloned().ok_or_else(|| {
      JoyError::new(
        "update",
        "dependency_not_found",
        format!("dependency `{key}` is not present in `joy.toml`"),
        1,
      )
    })?;

    match spec.source {
      DependencySource::Github => {
        let package_name = spec.declared_package(&key).to_string();
        let package = PackageId::parse(&package_name)
          .map_err(|err| JoyError::new("update", "invalid_package_id", err.to_string(), 1))?;
        let (desired_spec, fetched) = if let Some(version_req) = args.version.as_ref() {
          (
            DependencySpec {
              source: DependencySource::Github,
              package: spec.package.clone(),
              rev: String::new(),
              version: Some(version_req.clone()),
              registry: None,
              git: None,
              path: None,
              url: None,
              sha256: None,
            },
            fetch::fetch_github_semver_with_cache(&package, version_req, &cache)
              .map_err(|err| map_fetch_error("update", err))?,
          )
        } else if let Some(rev) = args.rev.as_ref() {
          (
            DependencySpec {
              source: DependencySource::Github,
              package: spec.package.clone(),
              rev: rev.clone(),
              version: None,
              registry: None,
              git: None,
              path: None,
              url: None,
              sha256: None,
            },
            fetch::fetch_github_with_cache(&package, rev, &cache)
              .map_err(|err| map_fetch_error("update", err))?,
          )
        } else if let Some(version_req) = spec.version.as_deref() {
          (
            spec.clone(),
            fetch::fetch_github_semver_with_cache(&package, version_req, &cache)
              .map_err(|err| map_fetch_error("update", err))?,
          )
        } else {
          let requested_rev = if spec.rev.trim().is_empty() { "HEAD" } else { spec.rev.as_str() };
          (
            spec.clone(),
            fetch::fetch_github_with_cache(&package, requested_rev, &cache)
              .map_err(|err| map_fetch_error("update", err))?,
          )
        };
        if spec != desired_spec {
          manifest_changed |= manifest.add_dependency(key.clone(), desired_spec);
        }
        let installed =
          linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
            .map_err(|err| JoyError::new("update", "header_install_failed", err.to_string(), 1))?;
        updated.push(UpdatedDependency {
          key: key.clone(),
          package: Some(package_name),
          source: "github".to_string(),
          registry: None,
          source_package: None,
          requested_rev: Some(fetched.requested_rev),
          requested_requirement: fetched.requested_requirement,
          resolved_version: fetched.resolved_version,
          resolved_commit: Some(fetched.resolved_commit),
          cache_hit: Some(fetched.cache_hit),
          header_link_path: Some(installed.link_path.display().to_string()),
          staged_only: false,
        });
      },
      DependencySource::Registry => {
        if args.rev.is_some() {
          return Err(JoyError::new(
            "update",
            "invalid_update_args",
            "registry dependencies currently require `--version <range>`; `--rev` is only supported for github/git dependencies",
            1,
          ));
        }
        let package_name = spec.declared_package(&key).to_string();
        let package = PackageId::parse(&package_name)
          .map_err(|err| JoyError::new("update", "invalid_package_id", err.to_string(), 1))?;
        let version_req =
          args.version.as_ref().cloned().or_else(|| spec.version.clone()).ok_or_else(|| {
            JoyError::new(
              "update",
              "invalid_update_args",
              format!("registry dependency `{key}` must set `version`"),
              1,
            )
          })?;
        let selected_registry = args
          .registry
          .clone()
          .or_else(|| spec.registry.clone())
          .unwrap_or_else(|| "default".to_string());
        let store = if let Some(store) = registry_store.as_ref() {
          store.clone()
        } else {
          let loaded = RegistryStore::load_named_for_project(&selected_registry, Some(&cwd))
            .map_err(|err| map_registry_error("update", err))?;
          registry_store = Some(loaded.clone());
          loaded
        };
        let release = store
          .resolve(package.as_str(), RegistryRequirement::Semver(&version_req))
          .map_err(|err| map_registry_error("update", err))?;
        let mut fetched = fetch::fetch_github_with_cache(&package, &release.source_rev, &cache)
          .map_err(|err| map_fetch_error("update", err))?;
        fetched.requested_requirement = release.requested_requirement.clone();
        fetched.resolved_version = Some(release.resolved_version.clone());
        let desired_spec = DependencySpec {
          source: DependencySource::Registry,
          package: spec.package.clone(),
          rev: String::new(),
          version: Some(version_req),
          registry: Some(selected_registry.clone()),
          git: None,
          path: None,
          url: None,
          sha256: None,
        };
        if spec != desired_spec {
          manifest_changed |= manifest.add_dependency(key.clone(), desired_spec);
        }
        let installed =
          linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
            .map_err(|err| JoyError::new("update", "header_install_failed", err.to_string(), 1))?;
        updated.push(UpdatedDependency {
          key: key.clone(),
          package: Some(package_name),
          source: "registry".to_string(),
          registry: Some(selected_registry),
          source_package: Some(release.source_package),
          requested_rev: Some(fetched.requested_rev),
          requested_requirement: fetched.requested_requirement,
          resolved_version: fetched.resolved_version,
          resolved_commit: Some(fetched.resolved_commit),
          cache_hit: Some(fetched.cache_hit),
          header_link_path: Some(installed.link_path.display().to_string()),
          staged_only: false,
        });
      },
      DependencySource::Git => {
        if args.version.is_some() {
          return Err(JoyError::new(
            "update",
            "invalid_update_args",
            "git dependencies do not support `--version`; use `--rev`",
            1,
          ));
        }
        let mut desired = spec.clone();
        if let Some(rev) = args.rev.as_ref() {
          desired.rev = rev.clone();
        }
        if spec != desired {
          manifest_changed |= manifest.add_dependency(key.clone(), desired.clone());
        }
        let package = resolve_update_package_id(&key, &desired)?;
        let source_git = desired.git.as_deref().ok_or_else(|| {
          JoyError::new(
            "update",
            "invalid_dependency_source",
            format!("dependency `{key}` uses source `git` but missing `git = \"...\"`"),
            1,
          )
        })?;
        let fetched = fetch::fetch_git_with_cache(&package, source_git, &desired.rev, &cache)
          .map_err(|err| map_fetch_error("update", err))?;
        let installed =
          linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
            .map_err(|err| JoyError::new("update", "header_install_failed", err.to_string(), 1))?;
        updated.push(UpdatedDependency {
          key: key.clone(),
          package: Some(package.to_string()),
          source: "git".to_string(),
          registry: None,
          source_package: None,
          requested_rev: Some(fetched.requested_rev),
          requested_requirement: None,
          resolved_version: fetched.resolved_version,
          resolved_commit: Some(fetched.resolved_commit),
          cache_hit: Some(fetched.cache_hit),
          header_link_path: Some(installed.link_path.display().to_string()),
          staged_only: false,
        });
      },
      DependencySource::Path => {
        if args.rev.is_some() || args.version.is_some() || args.sha256.is_some() {
          return Err(JoyError::new(
            "update",
            "invalid_update_args",
            "path dependencies do not support `--rev`, `--version`, or `--sha256` updates",
            1,
          ));
        }
        let package = resolve_update_package_id(&key, &spec)?;
        let source_path = spec.path.as_deref().ok_or_else(|| {
          JoyError::new(
            "update",
            "invalid_dependency_source",
            format!("dependency `{key}` uses source `path` but missing `path = \"...\"`"),
            1,
          )
        })?;
        let fetched = fetch::fetch_path_with_cache(&package, source_path, &cache)
          .map_err(|err| map_fetch_error("update", err))?;
        let installed =
          linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
            .map_err(|err| JoyError::new("update", "header_install_failed", err.to_string(), 1))?;
        updated.push(UpdatedDependency {
          key: key.clone(),
          package: Some(package.to_string()),
          source: "path".to_string(),
          registry: None,
          source_package: None,
          requested_rev: Some(fetched.requested_rev),
          requested_requirement: None,
          resolved_version: fetched.resolved_version,
          resolved_commit: Some(fetched.resolved_commit),
          cache_hit: Some(fetched.cache_hit),
          header_link_path: Some(installed.link_path.display().to_string()),
          staged_only: false,
        });
      },
      DependencySource::Archive => {
        if args.rev.is_some() || args.version.is_some() {
          return Err(JoyError::new(
            "update",
            "invalid_update_args",
            "archive dependencies do not support `--rev` or `--version` updates",
            1,
          ));
        }
        let mut desired = spec.clone();
        if let Some(sha) = args.sha256.as_ref() {
          desired.sha256 = Some(sha.clone());
        }
        if spec != desired {
          manifest_changed |= manifest.add_dependency(key.clone(), desired.clone());
        }
        let package = resolve_update_package_id(&key, &desired)?;
        let source_url = desired.url.as_deref().ok_or_else(|| {
          JoyError::new(
            "update",
            "invalid_dependency_source",
            format!("dependency `{key}` uses source `archive` but missing `url = \"...\"`"),
            1,
          )
        })?;
        let sha256 = desired.sha256.as_deref().ok_or_else(|| {
          JoyError::new(
            "update",
            "invalid_dependency_source",
            format!("dependency `{key}` uses source `archive` but missing `sha256 = \"...\"`"),
            1,
          )
        })?;
        let fetched = fetch::fetch_archive_with_cache(&package, source_url, sha256, &cache)
          .map_err(|err| map_fetch_error("update", err))?;
        let installed =
          linking::install_headers(&env_layout.include_dir, &package, &fetched.source_dir)
            .map_err(|err| JoyError::new("update", "header_install_failed", err.to_string(), 1))?;
        updated.push(UpdatedDependency {
          key: key.clone(),
          package: Some(package.to_string()),
          source: "archive".to_string(),
          registry: None,
          source_package: None,
          requested_rev: Some(fetched.requested_rev),
          requested_requirement: None,
          resolved_version: fetched.resolved_version,
          resolved_commit: Some(fetched.resolved_commit),
          cache_hit: Some(fetched.cache_hit),
          header_link_path: Some(installed.link_path.display().to_string()),
          staged_only: false,
        });
      },
    }
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
    if let Some(path) = &item.header_link_path {
      install_index.header_links.insert(path.clone());
    }
  }
  install_index
    .save(&install_index_path)
    .map_err(|err| JoyError::new("update", "state_index_error", err.to_string(), 1))?;

  updated.sort_by(|a, b| a.key.cmp(&b.key));

  let lockfile_warning = cwd.join("joy.lock").is_file().then_some(
    "joy.lock exists and may be stale after dependency updates; rerun `joy sync --update-lock` or `joy build --update-lock`".to_string(),
  );
  if let Some(warning) = &lockfile_warning {
    warnings.push(warning.clone());
  }

  let mut human_builder = if updated.is_empty() {
    HumanMessageBuilder::new("No dependencies updated")
  } else if manifest_changed {
    HumanMessageBuilder::new(format!("Updated {} dependency entries", updated.len()))
  } else {
    HumanMessageBuilder::new(format!("Refreshed {} dependency entries", updated.len()))
  }
  .kv("updated count", updated.len().to_string())
  .kv("manifest changed", manifest_changed.to_string());
  for warning in &warnings {
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
        "key": item.key,
        "package": item.package,
        "source": item.source,
        "registry": item.registry,
        "source_package": item.source_package,
        "rev": item.requested_rev,
        "requested_requirement": item.requested_requirement,
        "resolved_version": item.resolved_version,
        "resolved_commit": item.resolved_commit,
        "cache_hit": item.cache_hit,
        "header_link_path": item.header_link_path,
        "staged_only": item.staged_only,
      })).collect::<Vec<_>>(),
      "warnings": warnings,
    }),
  ))
}

fn resolve_update_package_id(key: &str, spec: &DependencySpec) -> Result<PackageId, JoyError> {
  let declared = spec.declared_package(key);
  if let Ok(package) = PackageId::parse(declared) {
    return Ok(package);
  }
  if matches!(spec.source, DependencySource::Git)
    && let Some(locator) = spec.git.as_deref()
    && let Some(package) = infer_git_package_id(locator)
  {
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
    .map_err(|err| JoyError::new("update", "invalid_package_id", err.to_string(), 1))
}

fn infer_git_package_id(locator: &str) -> Option<PackageId> {
  let trimmed = locator.trim().trim_end_matches('/').trim_end_matches(".git");
  let base = trimmed
    .strip_prefix("ssh://")
    .or_else(|| trimmed.strip_prefix("https://"))
    .or_else(|| trimmed.strip_prefix("http://"))
    .unwrap_or(trimmed);
  let parts = base.split('/').filter(|p| !p.is_empty()).collect::<Vec<_>>();
  if parts.len() < 2 {
    return None;
  }
  let repo = parts[parts.len() - 1];
  let owner = parts[parts.len() - 2];
  PackageId::parse(&format!("{owner}/{repo}")).ok()
}
