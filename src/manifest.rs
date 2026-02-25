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

/// Parsed workspace root `joy.toml` manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceManifest {
  pub workspace: WorkspaceSection,
}

/// Workspace-level configuration for multi-project routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceSection {
  #[serde(default)]
  pub members: Vec<String>,
  #[serde(default)]
  pub default_member: Option<String>,
}

/// Top-level manifest document, either a project manifest or a workspace root manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ManifestDocument {
  Project(Manifest),
  Workspace(WorkspaceManifest),
}

/// Project-level build configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSection {
  pub name: String,
  pub version: String,
  pub cpp_standard: String,
  pub entry: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub extra_sources: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub include_dirs: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub targets: Vec<ProjectTarget>,
}

/// Additional named binary targets for a project manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectTarget {
  pub name: String,
  pub entry: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub extra_sources: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub include_dirs: Vec<String>,
}

/// Dependency request recorded in `joy.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencySpec {
  pub source: DependencySource,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub rev: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
}

/// Supported dependency source backends for direct manifest entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencySource {
  Github,
  Registry,
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

  /// Remove a dependency entry by canonical package ID.
  ///
  /// Returns the removed spec if present.
  pub fn remove_dependency(&mut self, package: &str) -> Option<DependencySpec> {
    self.dependencies.remove(package)
  }

  fn validate(&self) -> Result<(), ManifestError> {
    if self.project.name.trim().is_empty() {
      return Err(ManifestError::Validation("project.name must not be empty".into()));
    }
    if self.project.entry.trim().is_empty() {
      return Err(ManifestError::Validation("project.entry must not be empty".into()));
    }
    if self.project.extra_sources.iter().any(|path| path.trim().is_empty()) {
      return Err(ManifestError::Validation(
        "project.extra_sources entries must not be empty".into(),
      ));
    }
    if self.project.include_dirs.iter().any(|path| path.trim().is_empty()) {
      return Err(ManifestError::Validation(
        "project.include_dirs entries must not be empty".into(),
      ));
    }
    for (id, spec) in &self.dependencies {
      let has_rev = !spec.rev.trim().is_empty();
      let has_version = spec.version.as_deref().is_some_and(|v| !v.trim().is_empty());
      if has_rev && has_version {
        return Err(ManifestError::Validation(format!(
          "dependency `{id}` cannot set both `rev` and `version`"
        )));
      }
      if !has_rev && !has_version {
        return Err(ManifestError::Validation(format!(
          "dependency `{id}` must set either `rev` or `version`"
        )));
      }
      if matches!(spec.source, DependencySource::Registry) && has_rev {
        return Err(ManifestError::Validation(format!(
          "dependency `{id}` uses source `registry` and must set `version` instead of `rev`"
        )));
      }
    }
    let mut target_names = std::collections::BTreeSet::new();
    for target in &self.project.targets {
      if target.name.trim().is_empty() {
        return Err(ManifestError::Validation("project.targets[].name must not be empty".into()));
      }
      if !target_names.insert(target.name.clone()) {
        return Err(ManifestError::Validation(format!(
          "duplicate project target `{}`",
          target.name
        )));
      }
      if target.entry.trim().is_empty() {
        return Err(ManifestError::Validation(format!(
          "project.targets[`{}`].entry must not be empty",
          target.name
        )));
      }
      if target.extra_sources.iter().any(|path| path.trim().is_empty()) {
        return Err(ManifestError::Validation(format!(
          "project.targets[`{}`].extra_sources entries must not be empty",
          target.name
        )));
      }
      if target.include_dirs.iter().any(|path| path.trim().is_empty()) {
        return Err(ManifestError::Validation(format!(
          "project.targets[`{}`].include_dirs entries must not be empty",
          target.name
        )));
      }
    }
    Ok(())
  }
}

impl WorkspaceManifest {
  /// Load, parse, and validate a workspace root manifest from disk.
  pub fn load(path: &Path) -> Result<Self, ManifestError> {
    let doc = ManifestDocument::load(path)?;
    match doc {
      ManifestDocument::Workspace(ws) => Ok(ws),
      ManifestDocument::Project(_) => Err(ManifestError::Validation(
        "expected a workspace root manifest with `[workspace]`".into(),
      )),
    }
  }

  fn validate(&self) -> Result<(), ManifestError> {
    if self.workspace.members.is_empty() {
      return Err(ManifestError::Validation(
        "workspace.members must contain at least one member".into(),
      ));
    }
    if self.workspace.members.iter().any(|m| m.trim().is_empty()) {
      return Err(ManifestError::Validation("workspace.members entries must not be empty".into()));
    }
    let mut seen = std::collections::BTreeSet::new();
    for member in &self.workspace.members {
      if !seen.insert(member) {
        return Err(ManifestError::Validation(format!("duplicate workspace member `{member}`")));
      }
    }
    if let Some(default) = &self.workspace.default_member
      && !self.workspace.members.iter().any(|m| m == default)
    {
      return Err(ManifestError::Validation(format!(
        "workspace.default_member `{default}` must be listed in workspace.members"
      )));
    }
    Ok(())
  }
}

impl ManifestDocument {
  /// Load a manifest document and determine whether it is a project or workspace manifest.
  pub fn load(path: &Path) -> Result<Self, ManifestError> {
    let raw = fs::read_to_string(path)
      .map_err(|source| ManifestError::ReadIo { path: path.to_path_buf(), source })?;
    let doc: Self = toml::from_str(&raw).map_err(|source| ManifestError::Parse {
      path: path.to_path_buf(),
      source: Box::new(source),
    })?;
    match &doc {
      Self::Project(project) => project.validate()?,
      Self::Workspace(workspace) => workspace.validate()?,
    }
    Ok(doc)
  }
}

/// Selected target configuration merged with project defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedTarget {
  pub name: String,
  pub entry: String,
  pub extra_sources: Vec<String>,
  pub include_dirs: Vec<String>,
  pub is_default: bool,
}

impl Manifest {
  /// Return the default target derived from the top-level `project` fields.
  pub fn default_target(&self) -> SelectedTarget {
    SelectedTarget {
      name: self.project.name.clone(),
      entry: self.project.entry.clone(),
      extra_sources: self.project.extra_sources.clone(),
      include_dirs: self.project.include_dirs.clone(),
      is_default: true,
    }
  }

  /// Resolve a named target, or the default target when `None`.
  pub fn select_target(&self, name: Option<&str>) -> Result<SelectedTarget, ManifestError> {
    let Some(name) = name else {
      return Ok(self.default_target());
    };
    let Some(target) = self.project.targets.iter().find(|t| t.name == name) else {
      return Err(ManifestError::Validation(format!(
        "unknown project target `{name}`; define it under `[[project.targets]]`"
      )));
    };
    Ok(SelectedTarget {
      name: target.name.clone(),
      entry: target.entry.clone(),
      extra_sources: target.extra_sources.clone(),
      include_dirs: target.include_dirs.clone(),
      is_default: false,
    })
  }

  /// Return the dependency request as an exact rev (when present) or semver range.
  pub fn dependency_requirement<'a>(
    &'a self,
    package: &str,
  ) -> Option<DependencyRequirementRef<'a>> {
    let spec = self.dependencies.get(package)?;
    if let Some(version) = spec.version.as_deref()
      && !version.trim().is_empty()
    {
      return Some(DependencyRequirementRef::Version(version));
    }
    Some(DependencyRequirementRef::Rev(if spec.rev.trim().is_empty() { "HEAD" } else { &spec.rev }))
  }
}

/// Borrowed dependency requirement view used by resolver/commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyRequirementRef<'a> {
  Rev(&'a str),
  Version(&'a str),
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
  use std::collections::BTreeMap;
  use std::path::PathBuf;
  use tempfile::TempDir;

  use super::{
    DependencyRequirementRef, DependencySource, DependencySpec, Manifest, ManifestDocument,
    ProjectSection, ProjectTarget, WorkspaceManifest,
  };

  #[test]
  fn manifest_roundtrip_serialization() {
    let mut manifest = Manifest {
      project: ProjectSection {
        name: "demo".into(),
        version: "0.1.0".into(),
        cpp_standard: "c++20".into(),
        entry: "src/main.cpp".into(),
        extra_sources: vec!["src/lib.cpp".into()],
        include_dirs: vec!["include".into()],
        targets: vec![ProjectTarget {
          name: "tool".into(),
          entry: "src/tool.cpp".into(),
          extra_sources: vec![],
          include_dirs: vec![],
        }],
      },
      dependencies: Default::default(),
    };
    manifest.dependencies.insert(
      "nlohmann/json".into(),
      DependencySpec { source: DependencySource::Github, rev: "HEAD".into(), version: None },
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

  #[test]
  fn parses_manifest_without_multifile_fields_using_defaults() {
    let manifest: Manifest = toml::from_str(
      r#"
[project]
name = "demo"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"

[dependencies]
"#,
    )
    .expect("parse manifest");

    assert!(manifest.project.extra_sources.is_empty());
    assert!(manifest.project.include_dirs.is_empty());
    assert!(manifest.project.targets.is_empty());
  }

  #[test]
  fn parses_workspace_manifest_document() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("joy.toml");
    std::fs::write(
      &path,
      r#"
[workspace]
members = ["apps/a", "apps/b"]
default_member = "apps/a"
"#,
    )
    .expect("write manifest");

    let doc = ManifestDocument::load(&path).expect("load doc");
    match doc {
      ManifestDocument::Workspace(WorkspaceManifest { workspace }) => {
        assert_eq!(workspace.members.len(), 2);
        assert_eq!(workspace.default_member.as_deref(), Some("apps/a"));
      },
      other => panic!("expected workspace doc, got {other:?}"),
    }
  }

  #[test]
  fn selects_named_project_target() {
    let manifest = Manifest {
      project: ProjectSection {
        name: "demo".into(),
        version: "0.1.0".into(),
        cpp_standard: "c++20".into(),
        entry: "src/main.cpp".into(),
        extra_sources: vec![],
        include_dirs: vec!["include".into()],
        targets: vec![ProjectTarget {
          name: "tool".into(),
          entry: "src/tool.cpp".into(),
          extra_sources: vec!["src/shared.cpp".into()],
          include_dirs: vec!["tools/include".into()],
        }],
      },
      dependencies: Default::default(),
    };

    let default = manifest.select_target(None).expect("default target");
    assert!(default.is_default);
    assert_eq!(default.name, "demo");
    let tool = manifest.select_target(Some("tool")).expect("tool target");
    assert!(!tool.is_default);
    assert_eq!(tool.entry, "src/tool.cpp");
    assert_eq!(tool.extra_sources, vec!["src/shared.cpp"]);
  }

  #[test]
  fn dependency_requirement_prefers_semver_when_present() {
    let manifest = Manifest {
      project: ProjectSection {
        name: "demo".into(),
        version: "0.1.0".into(),
        cpp_standard: "c++20".into(),
        entry: "src/main.cpp".into(),
        extra_sources: vec![],
        include_dirs: vec![],
        targets: vec![],
      },
      dependencies: BTreeMap::from([
        (
          "fmtlib/fmt".into(),
          DependencySpec {
            source: DependencySource::Github,
            rev: String::new(),
            version: Some("^11".into()),
          },
        ),
        (
          "nlohmann/json".into(),
          DependencySpec { source: DependencySource::Github, rev: "HEAD".into(), version: None },
        ),
      ]),
    };
    assert_eq!(
      manifest.dependency_requirement("fmtlib/fmt"),
      Some(DependencyRequirementRef::Version("^11"))
    );
    assert_eq!(
      manifest.dependency_requirement("nlohmann/json"),
      Some(DependencyRequirementRef::Rev("HEAD"))
    );
  }
}
