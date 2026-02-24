use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
  pub project: ProjectSection,
  #[serde(default)]
  pub dependencies: BTreeMap<String, DependencySpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSection {
  pub name: String,
  pub version: String,
  pub cpp_standard: String,
  pub entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencySpec {
  pub source: DependencySource,
  pub rev: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencySource {
  Github,
}

impl Manifest {
  pub fn load(path: &Path) -> Result<Self, ManifestError> {
    let raw = fs::read_to_string(path)
      .map_err(|source| ManifestError::Io { path: path.to_path_buf(), source })?;
    let manifest: Self = toml::from_str(&raw)
      .map_err(|source| ManifestError::Parse { path: path.to_path_buf(), source })?;
    manifest.validate()?;
    Ok(manifest)
  }

  pub fn save(&self, path: &Path) -> Result<(), ManifestError> {
    self.validate()?;
    let mut raw = toml::to_string_pretty(self).map_err(ManifestError::Serialize)?;
    if !raw.ends_with('\n') {
      raw.push('\n');
    }
    fs::write(path, raw).map_err(|source| ManifestError::Io { path: path.to_path_buf(), source })
  }

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

#[derive(Debug, Error)]
pub enum ManifestError {
  #[error("failed to read manifest `{path}`: {source}")]
  Io {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to parse manifest `{path}`: {source}")]
  Parse {
    path: PathBuf,
    #[source]
    source: toml::de::Error,
  },
  #[error("failed to serialize manifest: {0}")]
  Serialize(toml::ser::Error),
  #[error("invalid manifest: {0}")]
  Validation(String),
}

#[cfg(test)]
mod tests {
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
}
