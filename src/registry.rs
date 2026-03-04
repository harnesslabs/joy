//! Registry index loading and version resolution.
//!
//! Phase 18 introduces an index-backed dependency source path while preserving the existing
//! GitHub shorthand source flow. The first implementation uses a git-backed index mirror cached
//! under `JOY_HOME` and resolves package versions deterministically from a local checkout.

use semver::{Version, VersionReq};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::fetch;
use crate::fs_ops;
use crate::git_ops::{self, GitCommandError};
use crate::global_cache::{GlobalCache, GlobalCacheError};
use crate::manifest::DependencySource;
use crate::output::progress_detail_tty;
use crate::package_id::PackageId;

const DEFAULT_REGISTRY_NAME: &str = "default";
const DEFAULT_PUBLIC_REGISTRY_URL: &str = "https://github.com/harnesslabs/joy-registry.git";

/// Registry-backed version requirement used by resolver and CLI dependency commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryRequirement<'a> {
  Semver(&'a str),
  ExactVersion(&'a str),
}

/// Concrete registry release selected for a package requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRegistryRelease {
  pub registry: String,
  pub package_id: String,
  pub requested_requirement: Option<String>,
  pub resolved_version: String,
  pub source_kind: RegistrySourceKind,
  pub source_package: String,
  pub source_rev: String,
  pub manifest: Option<RegistryManifestSummary>,
}

/// Supported source backends for a registry release entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrySourceKind {
  Github,
}

/// Optional embedded package-manifest summary from registry index v2 releases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryManifestSummary {
  pub digest: Option<String>,
  pub kind: Option<String>,
  pub headers_include_roots: Vec<String>,
  pub dependencies: Vec<RegistryManifestDependency>,
}

/// A single dependency edge declared inside an embedded registry manifest summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryManifestDependency {
  pub id: String,
  pub source: DependencySource,
  pub rev: Option<String>,
  pub version: Option<String>,
}

/// Loaded registry index with deterministic package lookup and version selection.
#[derive(Debug, Clone)]
pub struct RegistryStore {
  name: String,
  packages_by_id: BTreeMap<String, RegistryPackage>,
}

impl RegistryStore {
  /// Load the default registry from a git-backed cache mirror.
  ///
  /// The default registry remote is configured through `JOY_REGISTRY_DEFAULT`.
  pub fn load_default() -> Result<Self, RegistryError> {
    Self::load_named(DEFAULT_REGISTRY_NAME)
  }

  /// Load a named registry from a git-backed cache mirror.
  pub fn load_named(name: &str) -> Result<Self, RegistryError> {
    let cache = GlobalCache::resolve()?;
    load_registry_from_git_cache(name, &cache)
  }

  /// Resolve a package requirement to a concrete release entry.
  pub fn resolve(
    &self,
    package_id: &str,
    requirement: RegistryRequirement<'_>,
  ) -> Result<ResolvedRegistryRelease, RegistryError> {
    let package =
      self.packages_by_id.get(package_id).ok_or_else(|| RegistryError::PackageNotFound {
        registry: self.name.clone(),
        package: package_id.to_string(),
      })?;

    let selected = match requirement {
      RegistryRequirement::Semver(raw_req) => {
        let req = VersionReq::parse(raw_req).map_err(|source| {
          RegistryError::InvalidVersionReq { requirement: raw_req.into(), source }
        })?;
        package
          .releases
          .iter()
          .filter(|release| req.matches(&release.version))
          .max_by(|a, b| a.version.cmp(&b.version).then_with(|| a.raw.version.cmp(&b.raw.version)))
          .ok_or_else(|| RegistryError::VersionNotFound {
            registry: self.name.clone(),
            package: package_id.to_string(),
            requested_requirement: raw_req.to_string(),
          })?
      },
      RegistryRequirement::ExactVersion(raw_version) => {
        let version = Version::parse(raw_version).map_err(|source| {
          RegistryError::InvalidVersionReq { requirement: raw_version.into(), source }
        })?;
        package.releases.iter().find(|release| release.version == version).ok_or_else(|| {
          RegistryError::VersionNotFound {
            registry: self.name.clone(),
            package: package_id.to_string(),
            requested_requirement: raw_version.to_string(),
          }
        })?
      },
    };

    Ok(ResolvedRegistryRelease {
      registry: self.name.clone(),
      package_id: package_id.to_string(),
      requested_requirement: match requirement {
        RegistryRequirement::Semver(req) => Some(req.to_string()),
        RegistryRequirement::ExactVersion(_) => None,
      },
      resolved_version: selected.version.to_string(),
      source_kind: selected.raw.source,
      source_package: selected.raw.package.clone(),
      source_rev: selected.raw.rev.clone(),
      manifest: selected.raw.manifest.clone().map(Into::into),
    })
  }

  #[cfg(test)]
  fn load_from_dir(name: &str, checkout_dir: &Path) -> Result<Self, RegistryError> {
    load_registry_from_checkout(name, checkout_dir)
  }
}

#[derive(Debug, Clone)]
struct RegistryPackage {
  releases: Vec<RegistryRelease>,
}

#[derive(Debug, Clone)]
struct RegistryRelease {
  version: Version,
  raw: RegistryReleaseEntry,
}

fn load_registry_from_git_cache(
  name: &str,
  cache: &GlobalCache,
) -> Result<RegistryStore, RegistryError> {
  cache.ensure_layout()?;
  let remote_url = registry_remote_url(name)?;
  let mirror_dir = cache.git_root.join("registry").join(format!("{name}.git"));
  let checkout_parent = cache.src_root.join("registry-index").join(name);

  let runtime = fetch::runtime_options();
  if runtime.offline && !mirror_dir.exists() {
    return Err(RegistryError::OfflineIndexMiss { registry: name.to_string(), mirror_dir });
  }

  ensure_registry_mirror(&remote_url, &mirror_dir, runtime.progress)?;
  let commit = resolve_registry_head_commit(&mirror_dir, runtime.offline)?;
  let checkout_dir = checkout_parent.join(&commit);
  if !checkout_dir.exists() {
    materialize_registry_checkout(
      &mirror_dir,
      &checkout_dir,
      &commit,
      cache.tmp_dir(),
      runtime.progress,
    )?;
  }
  load_registry_from_checkout(name, &checkout_dir)
}

fn load_registry_from_checkout(
  name: &str,
  checkout_dir: &Path,
) -> Result<RegistryStore, RegistryError> {
  let index_path = checkout_dir.join("index.toml");
  let raw = fs::read_to_string(&index_path).map_err(|source| RegistryError::Io {
    action: "reading registry index".into(),
    path: index_path.clone(),
    source,
  })?;
  let parsed: RegistryIndexFile = toml::from_str(&raw).map_err(|source| RegistryError::Parse {
    path: index_path.clone(),
    source: Box::new(source),
  })?;
  if parsed.version != 1 && parsed.version != 2 {
    return Err(RegistryError::UnsupportedIndexVersion(parsed.version));
  }

  let mut packages_by_id = BTreeMap::new();
  for pkg in parsed.packages {
    let canonical = PackageId::parse(&pkg.id).map_err(|_| {
      RegistryError::Validation(format!(
        "registry package `{}` must currently use canonical `owner/repo` form",
        pkg.id
      ))
    })?;
    let _ = canonical;
    if pkg.versions.is_empty() {
      return Err(RegistryError::Validation(format!(
        "registry package `{}` must define at least one version entry",
        pkg.id
      )));
    }
    if packages_by_id.contains_key(&pkg.id) {
      return Err(RegistryError::Validation(format!("duplicate registry package `{}`", pkg.id)));
    }

    let mut releases = Vec::new();
    let mut seen_versions = std::collections::BTreeSet::new();
    for release in pkg.versions {
      let version = Version::parse(&release.version).map_err(|source| {
        RegistryError::InvalidVersionReq { requirement: release.version.clone(), source }
      })?;
      if !seen_versions.insert(version.clone()) {
        return Err(RegistryError::Validation(format!(
          "duplicate registry release version `{}` for `{}`",
          version, pkg.id
        )));
      }
      match release.source {
        RegistrySourceKind::Github => {
          let _ = PackageId::parse(&release.package).map_err(|_| {
            RegistryError::Validation(format!(
              "registry release `{}` for `{}` has invalid github package `{}`",
              release.version, pkg.id, release.package
            ))
          })?;
          // Phase 18 initial cut keeps manifest IDs aligned with fetch/build package IDs.
          if release.package != pkg.id {
            return Err(RegistryError::Validation(format!(
              "registry release `{}` for `{}` maps to `{}`; alias packages are not supported yet in this phase cut",
              release.version, pkg.id, release.package
            )));
          }
          if release.rev.trim().is_empty() {
            return Err(RegistryError::Validation(format!(
              "registry release `{}` for `{}` must set a non-empty `rev`",
              release.version, pkg.id
            )));
          }
        },
      }
      if let Some(manifest) = &release.manifest {
        if let Some(digest) = &manifest.digest
          && digest.trim().is_empty()
        {
          return Err(RegistryError::Validation(format!(
            "registry release `{}` for `{}` has empty manifest digest",
            release.version, pkg.id
          )));
        }
        if let Some(kind) = &manifest.kind
          && kind.trim().is_empty()
        {
          return Err(RegistryError::Validation(format!(
            "registry release `{}` for `{}` has empty manifest kind",
            release.version, pkg.id
          )));
        }
        if manifest.headers_include_roots.iter().any(|root| root.trim().is_empty()) {
          return Err(RegistryError::Validation(format!(
            "registry release `{}` for `{}` has empty manifest headers_include_roots entry",
            release.version, pkg.id
          )));
        }
        for dep in &manifest.dependencies {
          let _ = PackageId::parse(&dep.id).map_err(|_| {
            RegistryError::Validation(format!(
              "registry release `{}` for `{}` has invalid manifest dependency id `{}`",
              release.version, pkg.id, dep.id
            ))
          })?;
          let has_rev = dep.rev.as_deref().is_some_and(|rev| !rev.trim().is_empty());
          let has_version = dep.version.as_deref().is_some_and(|v| !v.trim().is_empty());
          if has_rev == has_version {
            return Err(RegistryError::Validation(format!(
              "registry release `{}` for `{}` manifest dependency `{}` must set exactly one of `rev` or `version`",
              release.version, pkg.id, dep.id
            )));
          }
          if matches!(dep.source, DependencySource::Registry) && has_rev {
            return Err(RegistryError::Validation(format!(
              "registry release `{}` for `{}` manifest dependency `{}` uses source `registry` and must set `version`",
              release.version, pkg.id, dep.id
            )));
          }
        }
      }
      releases.push(RegistryRelease { version, raw: release });
    }

    packages_by_id.insert(pkg.id, RegistryPackage { releases });
  }

  Ok(RegistryStore { name: name.to_string(), packages_by_id })
}

fn registry_remote_url(name: &str) -> Result<String, RegistryError> {
  if name != DEFAULT_REGISTRY_NAME {
    return Err(RegistryError::RegistryNotConfigured {
      registry: name.to_string(),
      reason: "only the default registry is currently supported".into(),
    });
  }
  Ok(std::env::var("JOY_REGISTRY_DEFAULT").unwrap_or_else(|_| DEFAULT_PUBLIC_REGISTRY_URL.into()))
}

fn ensure_registry_mirror(
  remote_url: &str,
  mirror_dir: &Path,
  progress: bool,
) -> Result<(), RegistryError> {
  if let Some(parent) = mirror_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
      action: "creating registry mirror parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }

  if mirror_dir.exists() {
    if progress {
      progress_detail_tty(&format!(
        "Refreshing cached registry index mirror from `{remote_url}` ({})",
        mirror_dir.display()
      ));
    }
    git_ops::run(
      Some(mirror_dir),
      ["fetch", "--all", "--tags", "--prune"],
      "fetching registry mirror",
    )
    .map_err(map_git_error)?;
    return Ok(());
  }

  if progress {
    progress_detail_tty(&format!(
      "Cloning registry index mirror from `{remote_url}` into {}",
      mirror_dir.display()
    ));
  }
  git_ops::run_dynamic(
    None,
    vec![
      "clone".into(),
      "--mirror".into(),
      remote_url.into(),
      mirror_dir.as_os_str().to_os_string(),
    ],
    "cloning registry mirror",
  )
  .map_err(map_git_error)
}

fn resolve_registry_head_commit(mirror_dir: &Path, offline: bool) -> Result<String, RegistryError> {
  match git_ops::run_capture(Some(mirror_dir), ["rev-parse", "HEAD"], "resolving registry HEAD")
    .map_err(map_git_error)
  {
    Ok(out) => Ok(out.trim().to_string()),
    Err(err) if offline => Err(RegistryError::OfflineIndexHeadUnavailable {
      mirror_dir: mirror_dir.to_path_buf(),
      source: Box::new(err),
    }),
    Err(err) => Err(err),
  }
}

fn materialize_registry_checkout(
  mirror_dir: &Path,
  dest_dir: &Path,
  commit: &str,
  tmp_root: &Path,
  progress: bool,
) -> Result<(), RegistryError> {
  if let Some(parent) = dest_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
      action: "creating registry checkout parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }
  fs::create_dir_all(tmp_root).map_err(|source| RegistryError::Io {
    action: "creating registry cache tmp dir".into(),
    path: tmp_root.to_path_buf(),
    source,
  })?;

  let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
  let tmp_dir = tmp_root.join(format!(
    "registry-index-{}-{}-{}-tmp",
    std::process::id(),
    nonce,
    dest_dir.file_name().and_then(|n| n.to_str()).unwrap_or("checkout")
  ));
  if tmp_dir.exists() {
    fs_ops::remove_path_if_exists(&tmp_dir).map_err(|source| RegistryError::Io {
      action: "cleaning temp registry checkout".into(),
      path: tmp_dir.clone(),
      source,
    })?;
  }

  let result = (|| {
    if progress {
      progress_detail_tty(&format!(
        "Materializing registry index checkout `{commit}` into cache ({})",
        dest_dir.display()
      ));
    }
    git_ops::run_dynamic(
      None,
      vec![
        "clone".into(),
        "--no-checkout".into(),
        mirror_dir.as_os_str().to_os_string(),
        tmp_dir.as_os_str().to_os_string(),
      ],
      "cloning registry checkout",
    )
    .map_err(map_git_error)?;
    git_ops::run(
      Some(&tmp_dir),
      ["checkout", "--detach", commit],
      "checking out registry index commit",
    )
    .map_err(map_git_error)?;
    if dest_dir.exists() {
      fs_ops::remove_path_if_exists(dest_dir).map_err(|source| RegistryError::Io {
        action: "removing stale registry checkout".into(),
        path: dest_dir.to_path_buf(),
        source,
      })?;
    }
    fs::rename(&tmp_dir, dest_dir).map_err(|source| RegistryError::Io {
      action: "moving registry checkout into cache".into(),
      path: dest_dir.to_path_buf(),
      source,
    })?;
    Ok::<(), RegistryError>(())
  })();

  if result.is_err() && tmp_dir.exists() {
    let _ = fs_ops::remove_path_if_exists(&tmp_dir);
  }
  result
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryIndexFile {
  version: u32,
  #[serde(default)]
  packages: Vec<RegistryPackageEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryPackageEntry {
  id: String,
  #[serde(default)]
  versions: Vec<RegistryReleaseEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryReleaseEntry {
  version: String,
  source: RegistrySourceKind,
  package: String,
  rev: String,
  #[serde(default)]
  manifest: Option<RegistryManifestSummaryEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryManifestSummaryEntry {
  #[serde(default)]
  digest: Option<String>,
  #[serde(default)]
  kind: Option<String>,
  #[serde(default)]
  headers_include_roots: Vec<String>,
  #[serde(default)]
  dependencies: Vec<RegistryManifestDependencyEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryManifestDependencyEntry {
  id: String,
  source: DependencySource,
  #[serde(default)]
  rev: Option<String>,
  #[serde(default)]
  version: Option<String>,
}

impl From<RegistryManifestSummaryEntry> for RegistryManifestSummary {
  fn from(value: RegistryManifestSummaryEntry) -> Self {
    Self {
      digest: value.digest,
      kind: value.kind,
      headers_include_roots: value.headers_include_roots,
      dependencies: value.dependencies.into_iter().map(Into::into).collect(),
    }
  }
}

impl From<RegistryManifestDependencyEntry> for RegistryManifestDependency {
  fn from(value: RegistryManifestDependencyEntry) -> Self {
    Self { id: value.id, source: value.source, rev: value.rev, version: value.version }
  }
}

impl<'de> Deserialize<'de> for RegistrySourceKind {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let raw = String::deserialize(deserializer)?;
    match raw.as_str() {
      "github" => Ok(Self::Github),
      other => Err(serde::de::Error::custom(format!(
        "unsupported registry source `{other}` (expected `github`)"
      ))),
    }
  }
}

fn map_git_error(err: GitCommandError) -> RegistryError {
  match err {
    GitCommandError::Spawn { action, source } => RegistryError::SpawnGit { action, source },
    GitCommandError::Failed { action, status, stdout, stderr } => {
      RegistryError::GitFailed { action, status, stdout, stderr }
    },
  }
}

/// Errors produced while loading or resolving the registry index.
#[derive(Debug, Error)]
pub enum RegistryError {
  #[error(transparent)]
  GlobalCache(#[from] GlobalCacheError),
  #[error("registry `{registry}` is not configured: {reason}")]
  RegistryNotConfigured { registry: String, reason: String },
  #[error("filesystem error while {action} at `{path}`: {source}")]
  Io {
    action: String,
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to parse registry index `{path}`: {source}")]
  Parse {
    path: PathBuf,
    #[source]
    source: Box<toml::de::Error>,
  },
  #[error("invalid registry index: {0}")]
  Validation(String),
  #[error("unsupported registry index version `{0}`")]
  UnsupportedIndexVersion(u32),
  #[error("failed to run git while {action}: {source}")]
  SpawnGit {
    action: String,
    #[source]
    source: std::io::Error,
  },
  #[error("git failed while {action} (status {status:?})\nstdout: {stdout}\nstderr: {stderr}")]
  GitFailed { action: String, status: Option<i32>, stdout: String, stderr: String },
  #[error("offline mode requires a cached registry mirror for `{registry}` (missing `{}`)", .mirror_dir.display())]
  OfflineIndexMiss { registry: String, mirror_dir: PathBuf },
  #[error(
    "offline mode could not resolve registry HEAD from cached mirror `{}`; refresh online first",
    .mirror_dir.display()
  )]
  OfflineIndexHeadUnavailable {
    mirror_dir: PathBuf,
    #[source]
    source: Box<RegistryError>,
  },
  #[error("registry package `{package}` not found in registry `{registry}`")]
  PackageNotFound { registry: String, package: String },
  #[error("invalid semver requirement `{requirement}`: {source}")]
  InvalidVersionReq {
    requirement: String,
    #[source]
    source: semver::Error,
  },
  #[error(
    "no registry version matching `{requested_requirement}` for `{package}` in registry `{registry}`"
  )]
  VersionNotFound { registry: String, package: String, requested_requirement: String },
}

impl RegistryError {
  pub fn is_offline_cache_miss(&self) -> bool {
    matches!(self, Self::OfflineIndexMiss { .. } | Self::OfflineIndexHeadUnavailable { .. })
  }

  pub fn is_invalid_version_requirement(&self) -> bool {
    matches!(self, Self::InvalidVersionReq { .. })
  }

  pub fn is_version_not_found(&self) -> bool {
    matches!(self, Self::VersionNotFound { .. })
  }

  pub fn is_package_not_found(&self) -> bool {
    matches!(self, Self::PackageNotFound { .. })
  }

  pub fn is_not_configured(&self) -> bool {
    matches!(self, Self::RegistryNotConfigured { .. })
  }
}

#[cfg(test)]
mod tests {
  use std::fs::File;
  use std::io::Write;
  use std::path::Path;

  use tempfile::TempDir;

  use super::{RegistryRequirement, RegistrySourceKind, RegistryStore};

  #[test]
  fn resolves_highest_matching_semver_release_from_registry_index() {
    let temp = TempDir::new().expect("tempdir");
    write_index(
      temp.path(),
      r#"version = 1

[[packages]]
id = "fmtlib/fmt"

[[packages.versions]]
version = "10.2.1"
source = "github"
package = "fmtlib/fmt"
rev = "v10.2.1"

[[packages.versions]]
version = "11.0.0"
source = "github"
package = "fmtlib/fmt"
rev = "v11.0.0"

[[packages.versions]]
version = "11.1.2"
source = "github"
package = "fmtlib/fmt"
rev = "v11.1.2"
"#,
    );

    let registry = RegistryStore::load_from_dir("default", temp.path()).expect("load registry");
    let resolved =
      registry.resolve("fmtlib/fmt", RegistryRequirement::Semver("^11")).expect("resolve");
    assert_eq!(resolved.registry, "default");
    assert_eq!(resolved.package_id, "fmtlib/fmt");
    assert_eq!(resolved.requested_requirement.as_deref(), Some("^11"));
    assert_eq!(resolved.resolved_version, "11.1.2");
    assert_eq!(resolved.source_kind, RegistrySourceKind::Github);
    assert_eq!(resolved.source_package, "fmtlib/fmt");
    assert_eq!(resolved.source_rev, "v11.1.2");
    assert!(resolved.manifest.is_none());
  }

  #[test]
  fn parses_registry_index_v2_embedded_manifest_summary() {
    let temp = TempDir::new().expect("tempdir");
    write_index(
      temp.path(),
      r#"version = 2

[[packages]]
id = "harnesslabs/igneous"

[[packages.versions]]
version = "0.3.0"
source = "github"
package = "harnesslabs/igneous"
rev = "v0.3.0"

[packages.versions.manifest]
digest = "sha256:deadbeef"
kind = "header_only"
headers_include_roots = ["include"]

[[packages.versions.manifest.dependencies]]
id = "xsimd/xsimd"
source = "registry"
version = "^13"
"#,
    );

    let registry = RegistryStore::load_from_dir("default", temp.path()).expect("load registry");
    let resolved = registry
      .resolve("harnesslabs/igneous", RegistryRequirement::ExactVersion("0.3.0"))
      .expect("resolve");

    let manifest = resolved.manifest.expect("embedded manifest summary");
    assert_eq!(manifest.digest.as_deref(), Some("sha256:deadbeef"));
    assert_eq!(manifest.kind.as_deref(), Some("header_only"));
    assert_eq!(manifest.headers_include_roots, vec!["include"]);
    assert_eq!(manifest.dependencies.len(), 1);
    assert_eq!(manifest.dependencies[0].id, "xsimd/xsimd");
    assert_eq!(manifest.dependencies[0].source, crate::manifest::DependencySource::Registry);
    assert_eq!(manifest.dependencies[0].version.as_deref(), Some("^13"));
    assert_eq!(manifest.dependencies[0].rev, None);
  }

  #[test]
  fn rejects_invalid_registry_v2_manifest_dependency_requirement_shape() {
    let temp = TempDir::new().expect("tempdir");
    write_index(
      temp.path(),
      r#"version = 2

[[packages]]
id = "harnesslabs/igneous"

[[packages.versions]]
version = "0.3.0"
source = "github"
package = "harnesslabs/igneous"
rev = "v0.3.0"

[packages.versions.manifest]
kind = "header_only"

[[packages.versions.manifest.dependencies]]
id = "xsimd/xsimd"
source = "registry"
rev = "v1"
version = "^1"
"#,
    );

    let err =
      RegistryStore::load_from_dir("default", temp.path()).expect_err("expected validation");
    assert!(err.to_string().contains("must set exactly one of `rev` or `version`"));
  }

  #[test]
  fn rejects_alias_mapping_in_phase18_initial_cut() {
    let temp = TempDir::new().expect("tempdir");
    write_index(
      temp.path(),
      r#"version = 1

[[packages]]
id = "fmtlib/fmt-registry"

[[packages.versions]]
version = "11.0.0"
source = "github"
package = "fmtlib/fmt"
rev = "v11.0.0"
"#,
    );

    let err =
      RegistryStore::load_from_dir("default", temp.path()).expect_err("expected validation");
    assert!(err.to_string().contains("alias packages are not supported yet"));
  }

  fn write_index(root: &Path, contents: &str) {
    let mut file = File::create(root.join("index.toml")).expect("create index");
    file.write_all(contents.as_bytes()).expect("write index");
    file.flush().expect("flush");
  }
}
