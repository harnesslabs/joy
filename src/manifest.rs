use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Parsed `joy.toml` manifest.
///
/// The schema is intentionally small in the current roadmap: a single project section and a map of
/// direct dependencies keyed by canonical `owner/repo` IDs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
  pub project: ProjectSection,
  #[serde(default)]
  pub dependencies: BTreeMap<String, DependencySpec>,
}

/// Project-level build configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSection {
  pub name: String,
  pub version: String,
  pub cpp_standard: String,
  pub entry: String,
}

/// Dependency request recorded in `joy.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencySpec {
  pub source: DependencySource,
  pub rev: String,
}

/// Supported dependency source backends for direct manifest entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencySource {
  Github,
}

impl Manifest {
  /// Load, parse, and validate a manifest from disk.
  pub fn load(path: &Path) -> Result<Self, ManifestError> {
    let raw = fs::read_to_string(path)
      .map_err(|source| ManifestError::ReadIo { path: path.to_path_buf(), source })?;
    let manifest: Self = toml::from_str(&raw).map_err(|source| ManifestError::Parse {
      path: path.to_path_buf(),
      source: Box::new(source),
    })?;
    manifest.validate()?;
    Ok(manifest)
  }

  /// Validate and write a manifest back to disk.
  ///
  /// Formatting normalization is expected because `toml` serialization is used.
  pub fn save(&self, path: &Path) -> Result<(), ManifestError> {
    self.validate()?;
    let mut raw = toml::to_string_pretty(self).map_err(ManifestError::Serialize)?;
    if !raw.ends_with('\n') {
      raw.push('\n');
    }
    fs::write(path, raw)
      .map_err(|source| ManifestError::WriteIo { path: path.to_path_buf(), source })
  }

  /// Insert or replace a dependency entry.
  ///
  /// Returns `true` if the manifest changed and `false` if the existing entry was identical.
  pub fn add_dependency(&mut self, package: String, spec: DependencySpec) -> bool {
    match self.dependencies.get(&package) {
      Some(existing) if existing == &spec => false,
      _ => {
        self.dependencies.insert(package, spec);
        true
      },
    }
  }

  fn validate(&self) -> Result<(), ManifestError> {
    if self.project.name.trim().is_empty() {
      return Err(ManifestError::Validation("project.name must not be empty".into()));
    }
    if self.project.entry.trim().is_empty() {
      return Err(ManifestError::Validation("project.entry must not be empty".into()));
    }
    Ok(())
  }
}

/// Errors produced when loading, validating, or writing `joy.toml`.
#[derive(Debug, Error)]
pub enum ManifestError {
  #[error("failed to read manifest `{path}`: {source}")]
  ReadIo {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to write manifest `{path}`: {source}")]
  WriteIo {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to parse manifest `{path}`: {source}")]
  Parse {
    path: PathBuf,
    #[source]
    source: Box<toml::de::Error>,
  },
  #[error("failed to serialize manifest: {0}")]
  Serialize(toml::ser::Error),
  #[error("invalid manifest: {0}")]
  Validation(String),
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::{DependencySource, DependencySpec, Manifest, ProjectSection};

  #[test]
  fn manifest_roundtrip_serialization() {
    let mut manifest = Manifest {
      project: ProjectSection {
        name: "demo".into(),
        version: "0.1.0".into(),
        cpp_standard: "c++20".into(),
        entry: "src/main.cpp".into(),
      },
      dependencies: Default::default(),
    };
    manifest.dependencies.insert(
      "nlohmann/json".into(),
      DependencySpec { source: DependencySource::Github, rev: "HEAD".into() },
    );

    let raw = toml::to_string_pretty(&manifest).expect("serialize");
    let reparsed: Manifest = toml::from_str(&raw).expect("parse");

    assert_eq!(reparsed, manifest);
  }

  #[test]
  fn write_io_error_mentions_write_action() {
    let err = super::ManifestError::WriteIo {
      path: PathBuf::from("joy.toml"),
      source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    };
    assert!(err.to_string().contains("failed to write manifest"));
  }
}
