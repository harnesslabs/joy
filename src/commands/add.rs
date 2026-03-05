use serde_json::json;
use std::env;

use crate::cli::{AddArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::fs_ops;
use crate::global_cache::GlobalCache;
use crate::install_index::InstallIndex;
use crate::linking::{self, HeaderInstall};
use crate::manifest::{DependencySource, DependencySpec, Manifest};
use crate::output::{HumanMessageBuilder, progress_detail, progress_stage};
use crate::package_id::PackageId;
use crate::project_env;
use crate::registry::{RegistryRequirement, RegistryStore};

use super::build;
use super::dependency_common::{
  infer_dependency_key, map_fetch_error, map_registry_error, parse_dependency_input,
};

pub fn handle(args: AddArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  if runtime.frozen {
    return Err(JoyError::new(
      "add",
      "frozen_disallows_add",
      "`joy add` mutates the manifest and cannot run with `--frozen`; rerun without `--frozen`",
      1,
    ));
  }
  if args.rev.is_some() && args.version.is_some() {
    return Err(JoyError::new(
      "add",
      "invalid_add_args",
      "`--rev` and `--version` are mutually exclusive; choose one dependency requirement style",
      1,
    ));
  }

  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });

  if runtime.progress {
    progress_stage(&format!("Resolving dependency `{}`", args.package));
  }

  let parsed_input = parse_dependency_input("add", &args.package)?;
  let dependency_key = if let Some(explicit) = args.as_name.clone() {
    explicit
  } else if matches!(parsed_input.source, DependencySource::Github | DependencySource::Registry) {
    parsed_input.reference.clone()
  } else {
    infer_dependency_key(&parsed_input.source, &parsed_input.reference).ok_or_else(|| {
      JoyError::new(
        "add",
        "invalid_add_args",
        "could not infer dependency key; provide one with `--as <name>`",
        1,
      )
    })?
  };

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

  let mut warnings = Vec::<String>::new();
  let mut fetched = None::<fetch::FetchResult>;
  let mut installed = None::<HeaderInstall>;
  let mut registry_name = None::<String>;
  let mut source_package = None::<String>;
  let mut sync_attempted = false;
  let mut created_env_paths = Vec::<String>::new();
  let mut state_index_path = None::<String>;

  let manifest_spec = match parsed_input.source {
    DependencySource::Github => {
      let package = PackageId::parse(&parsed_input.reference)
        .map_err(|err| JoyError::new("add", "invalid_package_id", err.to_string(), 1))?;
      let cache = GlobalCache::resolve()
        .map_err(|err| JoyError::new("add", "cache_setup_failed", err.to_string(), 1))?;
      let fetched_result = if let Some(version_req) = args.version.as_deref() {
        fetch::fetch_github_semver_with_cache(&package, version_req, &cache)
          .map_err(|err| map_fetch_error("add", err))?
      } else {
        let rev = args.rev.as_deref().unwrap_or("HEAD");
        fetch::fetch_github_with_cache(&package, rev, &cache)
          .map_err(|err| map_fetch_error("add", err))?
      };
      let env_layout = project_env::ensure_layout(&cwd)
        .map_err(|err| JoyError::new("add", "env_setup_failed", err.to_string(), 1))?;
      created_env_paths =
        env_layout.created_paths.iter().map(|path| path.display().to_string()).collect();
      if runtime.progress {
        progress_detail(&format!("Installing headers for `{}`", package.as_str()));
      }
      let installed_headers =
        linking::install_headers(&env_layout.include_dir, &package, &fetched_result.source_dir)
          .map_err(|err| JoyError::new("add", "header_install_failed", err.to_string(), 1))?;
      let install_index_path = env_layout.state_dir.join("install-index.json");
      state_index_path = Some(install_index_path.display().to_string());
      let mut install_index = InstallIndex::load_or_default(&install_index_path)
        .map_err(|err| JoyError::new("add", "state_index_error", err.to_string(), 1))?;
      install_index.header_links.insert(installed_headers.link_path.display().to_string());
      install_index
        .save(&install_index_path)
        .map_err(|err| JoyError::new("add", "state_index_error", err.to_string(), 1))?;
      fetched = Some(fetched_result);
      installed = Some(installed_headers);
      DependencySpec {
        source: DependencySource::Github,
        package: (dependency_key != parsed_input.reference)
          .then_some(parsed_input.reference.clone()),
        rev: if args.version.is_some() {
          String::new()
        } else {
          args.rev.clone().unwrap_or_else(|| "HEAD".to_string())
        },
        version: args.version.clone(),
        registry: None,
        git: None,
        path: None,
        url: None,
        sha256: None,
      }
    },
    DependencySource::Registry => {
      let package = PackageId::parse(&parsed_input.reference)
        .map_err(|err| JoyError::new("add", "invalid_package_id", err.to_string(), 1))?;
      let version_req = args.version.as_deref().ok_or_else(|| {
        JoyError::new(
          "add",
          "invalid_add_args",
          "registry dependencies require `--version <range>`",
          1,
        )
      })?;
      let selected_registry = args.registry.clone().unwrap_or_else(|| "default".to_string());
      let store = RegistryStore::load_named_for_project(&selected_registry, Some(&cwd))
        .map_err(|err| map_registry_error("add", err))?;
      let release = store
        .resolve(package.as_str(), RegistryRequirement::Semver(version_req))
        .map_err(|err| map_registry_error("add", err))?;
      let cache = GlobalCache::resolve()
        .map_err(|err| JoyError::new("add", "cache_setup_failed", err.to_string(), 1))?;
      let mut fetched_result =
        fetch::fetch_github_with_cache(&package, &release.source_rev, &cache)
          .map_err(|err| map_fetch_error("add", err))?;
      fetched_result.requested_requirement = release.requested_requirement.clone();
      fetched_result.resolved_version = Some(release.resolved_version.clone());
      let env_layout = project_env::ensure_layout(&cwd)
        .map_err(|err| JoyError::new("add", "env_setup_failed", err.to_string(), 1))?;
      created_env_paths =
        env_layout.created_paths.iter().map(|path| path.display().to_string()).collect();
      if runtime.progress {
        progress_detail(&format!("Installing headers for `{}`", package.as_str()));
      }
      let installed_headers =
        linking::install_headers(&env_layout.include_dir, &package, &fetched_result.source_dir)
          .map_err(|err| JoyError::new("add", "header_install_failed", err.to_string(), 1))?;
      let install_index_path = env_layout.state_dir.join("install-index.json");
      state_index_path = Some(install_index_path.display().to_string());
      let mut install_index = InstallIndex::load_or_default(&install_index_path)
        .map_err(|err| JoyError::new("add", "state_index_error", err.to_string(), 1))?;
      install_index.header_links.insert(installed_headers.link_path.display().to_string());
      install_index
        .save(&install_index_path)
        .map_err(|err| JoyError::new("add", "state_index_error", err.to_string(), 1))?;
      fetched = Some(fetched_result);
      installed = Some(installed_headers);
      registry_name = Some(selected_registry.clone());
      source_package = Some(release.source_package.clone());
      DependencySpec {
        source: DependencySource::Registry,
        package: (dependency_key != parsed_input.reference)
          .then_some(parsed_input.reference.clone()),
        rev: String::new(),
        version: Some(version_req.to_string()),
        registry: Some(selected_registry),
        git: None,
        path: None,
        url: None,
        sha256: None,
      }
    },
    DependencySource::Git => {
      let rev = args.rev.clone().ok_or_else(|| {
        JoyError::new(
          "add",
          "invalid_add_args",
          "git dependencies require `--rev <commit-or-tag>`",
          1,
        )
      })?;
      if args.version.is_some() {
        return Err(JoyError::new(
          "add",
          "invalid_add_args",
          "git dependencies do not support `--version`; use `--rev`",
          1,
        ));
      }
      warnings.push(
        "git dependencies are recorded in `joy.toml`, but build/sync resolver support is not complete yet"
          .to_string(),
      );
      DependencySpec {
        source: DependencySource::Git,
        package: None,
        rev,
        version: None,
        registry: None,
        git: Some(parsed_input.reference.clone()),
        path: None,
        url: None,
        sha256: None,
      }
    },
    DependencySource::Path => {
      if args.rev.is_some() || args.version.is_some() {
        return Err(JoyError::new(
          "add",
          "invalid_add_args",
          "path dependencies cannot set `--rev` or `--version`",
          1,
        ));
      }
      warnings.push(
        "path dependencies are recorded in `joy.toml`, but build/sync resolver support is not complete yet"
          .to_string(),
      );
      DependencySpec {
        source: DependencySource::Path,
        package: None,
        rev: String::new(),
        version: None,
        registry: None,
        git: None,
        path: Some(parsed_input.reference.clone()),
        url: None,
        sha256: None,
      }
    },
    DependencySource::Archive => {
      if args.rev.is_some() || args.version.is_some() {
        return Err(JoyError::new(
          "add",
          "invalid_add_args",
          "archive dependencies cannot set `--rev` or `--version`",
          1,
        ));
      }
      let sha256 = args.sha256.clone().ok_or_else(|| {
        JoyError::new(
          "add",
          "invalid_add_args",
          "archive dependencies require `--sha256 <checksum>`",
          1,
        )
      })?;
      warnings.push(
        "archive dependencies are recorded in `joy.toml`, but build/sync resolver support is not complete yet"
          .to_string(),
      );
      DependencySpec {
        source: DependencySource::Archive,
        package: None,
        rev: String::new(),
        version: None,
        registry: None,
        git: None,
        path: None,
        url: Some(parsed_input.reference.clone()),
        sha256: Some(sha256),
      }
    },
  };

  let changed = manifest.add_dependency(dependency_key.clone(), manifest_spec.clone());
  if changed && let Err(err) = manifest.save(&manifest_path) {
    if let Some(installed_headers) = &installed {
      let _ = fs_ops::remove_path_if_exists(&installed_headers.link_path);
    }
    return Err(JoyError::new("add", "manifest_write_error", err.to_string(), 1));
  }

  if !args.no_sync
    && matches!(manifest_spec.source, DependencySource::Github | DependencySource::Registry)
  {
    sync_attempted = true;
    let sync_result = build::sync_project(build::BuildOptions {
      release: false,
      target: None,
      locked: false,
      update_lock: true,
      offline: runtime.offline,
      progress: runtime.progress,
    });
    if let Err(err) = sync_result {
      return Err(JoyError::new(
        "add",
        "add_sync_failed",
        format!(
          "dependency `{}` was added to `joy.toml`, but sync-lite failed: {}\nrerun `joy sync --update-lock`{}",
          dependency_key,
          err.message,
          if runtime.offline { " (or rerun online to refresh cache state)" } else { "" }
        ),
        err.exit_code,
      ));
    }
  } else if !args.no_sync {
    warnings.push("sync-lite was skipped for this dependency source backend".to_string());
  }

  if args.no_sync && cwd.join("joy.lock").is_file() {
    warnings.push(
      "joy.lock exists and may be stale; refresh it with `joy sync --update-lock`".to_string(),
    );
  }

  let mut human_builder = if changed {
    HumanMessageBuilder::new(format!("Added dependency `{}`", dependency_key))
  } else {
    HumanMessageBuilder::new(format!("Dependency `{}` already present", dependency_key))
  };
  human_builder = human_builder.kv("source", manifest_spec.source.as_str().to_string());
  if let Some(package) = manifest_spec.package.as_deref() {
    human_builder = human_builder.kv("package", package.to_string());
  }
  if let Some(fetched_result) = fetched.as_ref() {
    if let Some(req) = fetched_result.requested_requirement.as_deref() {
      human_builder = human_builder.kv("requested version", req.to_string());
    } else {
      human_builder = human_builder.kv("requested rev", fetched_result.requested_rev.clone());
    }
    human_builder = human_builder.kv("resolved commit", fetched_result.resolved_commit.clone());
  } else if !manifest_spec.rev.is_empty() {
    human_builder = human_builder.kv("requested rev", manifest_spec.rev.clone());
  }
  if let Some(installed_headers) = installed.as_ref() {
    human_builder =
      human_builder.kv("headers installed", installed_headers.link_path.display().to_string());
  }
  for warning in &warnings {
    human_builder = human_builder.warning(warning.clone());
  }
  let human_message = human_builder.build();
  let package_value = manifest_spec.package.clone().unwrap_or_else(|| dependency_key.clone());
  let fetched_requested_rev = fetched.as_ref().map(|f| f.requested_rev.clone());
  let fetched_requested_requirement =
    fetched.as_ref().and_then(|f| f.requested_requirement.clone());
  let fetched_resolved_version = fetched.as_ref().and_then(|f| f.resolved_version.clone());
  let fetched_resolved_commit = fetched.as_ref().map(|f| f.resolved_commit.clone());
  let fetched_remote_url = fetched.as_ref().map(|f| f.remote_url.clone());
  let fetched_cache_source_dir = fetched.as_ref().map(|f| f.source_dir.display().to_string());
  let fetched_cache_hit = fetched.as_ref().map(|f| f.cache_hit);
  let installed_header_root = installed.as_ref().map(|h| h.header_root.display().to_string());
  let installed_header_link_path = installed.as_ref().map(|h| h.link_path.display().to_string());
  let installed_header_link_kind = installed.as_ref().map(|h| h.link_kind.to_string());

  let is_legacy_source =
    matches!(manifest_spec.source, DependencySource::Github | DependencySource::Registry);
  let mut data = serde_json::Map::new();
  data.insert("cache_hit".to_string(), json!(fetched_cache_hit));
  data.insert("cache_source_dir".to_string(), json!(fetched_cache_source_dir));
  data.insert("changed".to_string(), json!(changed));
  data.insert("created_env_paths".to_string(), json!(created_env_paths));
  data.insert("header_link_kind".to_string(), json!(installed_header_link_kind));
  data.insert("header_link_path".to_string(), json!(installed_header_link_path));
  data.insert("header_root".to_string(), json!(installed_header_root));
  data.insert("manifest_path".to_string(), json!(manifest_path.display().to_string()));
  data.insert("package".to_string(), json!(package_value));
  data.insert("project_root".to_string(), json!(cwd.display().to_string()));
  data.insert(
    "registry".to_string(),
    json!(manifest_spec.registry.clone().or(registry_name.clone())),
  );
  data.insert("remote_url".to_string(), json!(fetched_remote_url));
  data.insert("requested_requirement".to_string(), json!(fetched_requested_requirement));
  data.insert("resolved_commit".to_string(), json!(fetched_resolved_commit));
  data.insert("resolved_version".to_string(), json!(fetched_resolved_version));
  data.insert(
    "rev".to_string(),
    json!(fetched_requested_rev.or_else(|| Some(manifest_spec.rev.clone()))),
  );
  data.insert("source".to_string(), json!(manifest_spec.source.as_str()));
  data.insert("source_package".to_string(), json!(source_package));
  data.insert("state_index_path".to_string(), json!(state_index_path));
  data.insert("warnings".to_string(), json!(warnings));

  if !is_legacy_source {
    data.insert("key".to_string(), json!(dependency_key));
    data.insert("version".to_string(), json!(manifest_spec.version.clone()));
    data.insert("git".to_string(), json!(manifest_spec.git.clone()));
    data.insert("path".to_string(), json!(manifest_spec.path.clone()));
    data.insert("url".to_string(), json!(manifest_spec.url.clone()));
    data.insert("sha256".to_string(), json!(manifest_spec.sha256.clone()));
    data.insert("sync_attempted".to_string(), json!(sync_attempted));
    data.insert(
      "fetched".to_string(),
      json!(fetched.as_ref().map(|f| json!({
        "requested_rev": f.requested_rev,
        "requested_requirement": f.requested_requirement,
        "resolved_version": f.resolved_version,
        "resolved_commit": f.resolved_commit,
        "remote_url": f.remote_url,
        "source_dir": f.source_dir.display().to_string(),
        "cache_hit": f.cache_hit,
      }))),
    );
    data.insert(
      "installed".to_string(),
      json!(installed.as_ref().map(|h| json!({
        "header_root": h.header_root.display().to_string(),
        "header_link_path": h.link_path.display().to_string(),
        "header_link_kind": h.link_kind,
      }))),
    );
  }

  Ok(CommandOutput::new("add", human_message, serde_json::Value::Object(data)))
}
