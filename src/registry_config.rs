use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::global_cache::{GlobalCache, GlobalCacheError};

const REGISTRY_CONFIG_VERSION: u32 = 1;

/// Registry configuration scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryScope {
  User,
  Project,
}

/// Effective merged registry configuration (project overrides user).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveRegistryConfig {
  pub default: Option<String>,
  pub registries: BTreeMap<String, String>,
}

impl EffectiveRegistryConfig {
  pub fn resolve_url(&self, name: &str) -> Option<&str> {
    self.registries.get(name).map(String::as_str)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RegistryConfigFile {
  #[serde(default = "default_registry_config_version")]
  version: u32,
  #[serde(default)]
  default: Option<String>,
  #[serde(default)]
  registries: BTreeMap<String, RegistryConfigEntry>,
}

impl Default for RegistryConfigFile {
  fn default() -> Self {
    Self { version: REGISTRY_CONFIG_VERSION, default: None, registries: BTreeMap::new() }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RegistryConfigEntry {
  index: String,
}

fn default_registry_config_version() -> u32 {
  REGISTRY_CONFIG_VERSION
}

/// Load the effective registry configuration.
pub fn load_effective(
  project_root: Option<&Path>,
) -> Result<EffectiveRegistryConfig, RegistryConfigError> {
  let user = load_scope(RegistryScope::User, None)?;
  let project = if project_root.is_some() {
    load_scope(RegistryScope::Project, project_root)?
  } else {
    RegistryConfigFile::default()
  };

  let mut registries = BTreeMap::<String, String>::new();
  for (name, entry) in user.registries {
    registries.insert(name, entry.index);
  }
  for (name, entry) in project.registries {
    registries.insert(name, entry.index);
  }

  let default = project.default.or(user.default);
  Ok(EffectiveRegistryConfig { default, registries })
}

/// Add/update a registry mapping in the selected scope.
pub fn set_registry(
  scope: RegistryScope,
  project_root: Option<&Path>,
  name: &str,
  index: &str,
) -> Result<(), RegistryConfigError> {
  let mut cfg = load_scope(scope, project_root)?;
  validate_registry_name(name)?;
  if index.trim().is_empty() {
    return Err(RegistryConfigError::Validation(
      "registry index URL/path must not be empty".into(),
    ));
  }
  cfg.registries.insert(name.to_string(), RegistryConfigEntry { index: index.trim().to_string() });
  save_scope(scope, project_root, &cfg)
}

/// Remove a registry mapping from the selected scope.
pub fn remove_registry(
  scope: RegistryScope,
  project_root: Option<&Path>,
  name: &str,
) -> Result<bool, RegistryConfigError> {
  let mut cfg = load_scope(scope, project_root)?;
  validate_registry_name(name)?;
  let removed = cfg.registries.remove(name).is_some();
  if cfg.default.as_deref() == Some(name) {
    cfg.default = None;
  }
  if removed {
    save_scope(scope, project_root, &cfg)?;
  }
  Ok(removed)
}

/// Set the default registry name for the selected scope.
pub fn set_default_registry(
  scope: RegistryScope,
  project_root: Option<&Path>,
  name: &str,
) -> Result<(), RegistryConfigError> {
  let mut cfg = load_scope(scope, project_root)?;
  validate_registry_name(name)?;
  cfg.default = Some(name.to_string());
  save_scope(scope, project_root, &cfg)
}

fn load_scope(
  scope: RegistryScope,
  project_root: Option<&Path>,
) -> Result<RegistryConfigFile, RegistryConfigError> {
  let path = scope_path(scope, project_root)?;
  match fs::read_to_string(&path) {
    Ok(raw) => {
      let cfg: RegistryConfigFile = toml::from_str(&raw).map_err(|source| {
        RegistryConfigError::Parse { path: path.clone(), source: Box::new(source) }
      })?;
      if cfg.version != REGISTRY_CONFIG_VERSION {
        return Err(RegistryConfigError::UnsupportedVersion(cfg.version));
      }
      Ok(cfg)
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(RegistryConfigFile::default()),
    Err(source) => Err(RegistryConfigError::Io { path, source }),
  }
}

fn save_scope(
  scope: RegistryScope,
  project_root: Option<&Path>,
  cfg: &RegistryConfigFile,
) -> Result<(), RegistryConfigError> {
  if cfg.version != REGISTRY_CONFIG_VERSION {
    return Err(RegistryConfigError::UnsupportedVersion(cfg.version));
  }
  let path = scope_path(scope, project_root)?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|source| RegistryConfigError::Io { path: parent.to_path_buf(), source })?;
  }
  let mut raw = toml::to_string_pretty(cfg).map_err(RegistryConfigError::Serialize)?;
  if !raw.ends_with('\n') {
    raw.push('\n');
  }
  fs::write(&path, raw).map_err(|source| RegistryConfigError::Io { path, source })
}

fn scope_path(
  scope: RegistryScope,
  project_root: Option<&Path>,
) -> Result<PathBuf, RegistryConfigError> {
  match scope {
    RegistryScope::User => {
      let cache = GlobalCache::resolve()?;
      Ok(cache.joy_home.join("config").join("registries.toml"))
    },
    RegistryScope::Project => {
      let Some(root) = project_root else {
        return Err(RegistryConfigError::ProjectRootRequired);
      };
      Ok(root.join(".joy").join("registries.toml"))
    },
  }
}

fn validate_registry_name(name: &str) -> Result<(), RegistryConfigError> {
  if name.trim().is_empty() {
    return Err(RegistryConfigError::Validation("registry name must not be empty".into()));
  }
  if !name.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')) {
    return Err(RegistryConfigError::Validation(format!(
      "invalid registry name `{name}`; allowed characters are [A-Za-z0-9_.-]"
    )));
  }
  Ok(())
}

#[derive(Debug, Error)]
pub enum RegistryConfigError {
  #[error(transparent)]
  GlobalCache(#[from] GlobalCacheError),
  #[error("project-scoped registry config requires a project root")]
  ProjectRootRequired,
  #[error("filesystem error for `{path}`: {source}")]
  Io {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to parse registry config `{path}`: {source}")]
  Parse {
    path: PathBuf,
    #[source]
    source: Box<toml::de::Error>,
  },
  #[error("failed to serialize registry config: {0}")]
  Serialize(toml::ser::Error),
  #[error("unsupported registry config version `{0}`")]
  UnsupportedVersion(u32),
  #[error("invalid registry config: {0}")]
  Validation(String),
}

#[cfg(test)]
mod tests {
  use std::sync::{Mutex, OnceLock};

  use tempfile::TempDir;

  use super::{
    EffectiveRegistryConfig, RegistryScope, load_effective, remove_registry, set_default_registry,
    set_registry,
  };
  use crate::global_cache::GlobalCache;

  fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
  }

  #[test]
  fn effective_config_merges_user_and_project_with_project_override() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = TempDir::new().expect("tempdir");
    let joy_home = temp.path().join(".joy-home");
    unsafe { std::env::set_var("JOY_HOME", &joy_home) };

    let project_root = temp.path().join("proj");
    std::fs::create_dir_all(&project_root).expect("project root");

    set_registry(RegistryScope::User, None, "default", "https://example.com/default.git")
      .expect("set user default");
    set_default_registry(RegistryScope::User, None, "default").expect("set user default name");
    set_registry(
      RegistryScope::Project,
      Some(&project_root),
      "default",
      "https://example.com/project-default.git",
    )
    .expect("set project default");

    let effective = load_effective(Some(&project_root)).expect("load effective");
    assert_eq!(effective.default.as_deref(), Some("default"));
    assert_eq!(effective.resolve_url("default"), Some("https://example.com/project-default.git"));

    let _ = GlobalCache::resolve().expect("global cache path still valid");
  }

  #[test]
  fn remove_registry_unsets_default_when_matching_name() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = TempDir::new().expect("tempdir");
    unsafe { std::env::set_var("JOY_HOME", temp.path().join(".joy-home")) };

    set_registry(RegistryScope::User, None, "default", "https://example.com/default.git")
      .expect("set registry");
    set_default_registry(RegistryScope::User, None, "default").expect("set default");
    assert!(remove_registry(RegistryScope::User, None, "default").expect("remove"));

    let effective = load_effective(None).expect("load effective");
    assert_eq!(effective, EffectiveRegistryConfig::default());
  }
}
