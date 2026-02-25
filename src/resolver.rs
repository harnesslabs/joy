//! Dependency resolution and DAG construction for manifest + recipe dependencies.
//!
//! The current resolver intentionally uses an exact-ref model: direct dependencies provide an
//! explicit ref (or `HEAD`), recipes declare exact revs for transitive dependencies, and git fetch
//! resolution yields concrete commits. Conflicts are reported when one package ID resolves to
//! different commits within the same graph.

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use petgraph::Direction;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use thiserror::Error;

use crate::fetch;
use crate::lockfile;
use crate::manifest::{DependencyRequirementRef, DependencySource, Manifest, ManifestDocument};
use crate::package_id::{PackageId, PackageIdError};
use crate::recipes::RecipeStore;
use crate::registry::{self, RegistryRequirement};

/// A resolved package node in the dependency graph.
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
  pub id: PackageId,
  pub source: DependencySource,
  pub registry: Option<String>,
  pub source_package: Option<String>,
  pub requested_rev: String,
  pub requested_requirement: Option<String>,
  pub resolved_version: Option<String>,
  pub resolved_commit: String,
  pub recipe_slug: Option<String>,
  pub header_only: bool,
  pub direct: bool,
}

/// Resolved dependency DAG with stable lookup helpers.
#[derive(Debug, Clone)]
pub struct ResolvedGraph {
  graph: DiGraph<ResolvedPackage, ()>,
  by_id: BTreeMap<String, NodeIndex>,
}

impl ResolvedGraph {
  /// Lookup a resolved package by canonical package ID.
  pub fn package(&self, id: &str) -> Option<&ResolvedPackage> {
    self.by_id.get(id).and_then(|idx| self.graph.node_weight(*idx))
  }

  /// Iterate all resolved packages in graph insertion order.
  pub fn packages(&self) -> impl Iterator<Item = &ResolvedPackage> {
    self.graph.node_weights()
  }

  /// Return a topological build order with dependencies before dependents.
  pub fn build_order(&self) -> Result<Vec<&ResolvedPackage>, ResolverError> {
    let order = toposort(&self.graph, None).map_err(|cycle| {
      let id = self
        .graph
        .node_weight(cycle.node_id())
        .map(|pkg| pkg.id.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());
      ResolverError::Cycle { package: id }
    })?;
    Ok(order.into_iter().filter_map(|idx| self.graph.node_weight(idx)).collect::<Vec<_>>())
  }

  /// Convenience helper returning the topological order as package IDs.
  pub fn build_order_ids(&self) -> Result<Vec<String>, ResolverError> {
    Ok(self.build_order()?.into_iter().map(|pkg| pkg.id.to_string()).collect())
  }

  /// Return the canonical IDs of the direct dependencies of the given package.
  ///
  /// The resolver stores edges in dependency -> dependent direction, so dependencies are incoming
  /// neighbors of the requested package node.
  pub fn dependency_ids(&self, id: &str) -> Option<Vec<String>> {
    let idx = *self.by_id.get(id)?;
    let mut deps = self
      .graph
      .neighbors_directed(idx, Direction::Incoming)
      .filter_map(|dep_idx| self.graph.node_weight(dep_idx))
      .map(|pkg| pkg.id.to_string())
      .collect::<Vec<_>>();
    deps.sort();
    deps.dedup();
    Some(deps)
  }
}

/// Resolve a manifest using the default fetch-based commit resolver.
pub fn resolve_manifest(
  manifest: &Manifest,
  recipes: &RecipeStore,
) -> Result<ResolvedGraph, ResolverError> {
  let mut registry_store =
    if manifest.dependencies.values().any(|spec| matches!(spec.source, DependencySource::Registry))
    {
      Some(
        registry::RegistryStore::load_default()
          .map_err(|source| ResolverError::RegistryLoad { source })?,
      )
    } else {
      None
    };
  resolve_manifest_with_selector(manifest, recipes, |package, source, request| match source {
    DependencySource::Github => match request {
      ResolveRequest::Rev(requested_rev) => {
        let fetched = fetch::fetch_github(package, requested_rev)
          .map_err(|source| ResolverError::Fetch { package: package.to_string(), source })?;
        Ok(ResolvedSelection {
          registry: None,
          source_package: None,
          requested_rev: fetched.requested_rev,
          requested_requirement: None,
          resolved_version: None,
          resolved_commit: fetched.resolved_commit,
          declared_deps: package_manifest_declared_dependencies(package, &fetched.source_dir)?,
        })
      },
      ResolveRequest::Version(version_req) => {
        let fetched = fetch::fetch_github_semver(package, version_req)
          .map_err(|source| ResolverError::Fetch { package: package.to_string(), source })?;
        Ok(ResolvedSelection {
          registry: None,
          source_package: None,
          requested_rev: fetched.requested_rev,
          requested_requirement: fetched.requested_requirement,
          resolved_version: fetched.resolved_version,
          resolved_commit: fetched.resolved_commit,
          declared_deps: package_manifest_declared_dependencies(package, &fetched.source_dir)?,
        })
      },
    },
    DependencySource::Registry => {
      if registry_store.is_none() {
        registry_store = Some(
          registry::RegistryStore::load_default()
            .map_err(|source| ResolverError::RegistryLoad { source })?,
        );
      }
      let store = registry_store.as_ref().expect("registry store must be loaded");
      let registry_requirement = match request {
        ResolveRequest::Version(req) => RegistryRequirement::Semver(req.as_str()),
        ResolveRequest::Rev(rev) => RegistryRequirement::ExactVersion(rev.as_str()),
      };
      let release = store.resolve(package.as_str(), registry_requirement).map_err(|source| {
        ResolverError::RegistryResolve { package: package.to_string(), source }
      })?;
      let source_package = PackageId::parse(&release.source_package).map_err(|source| {
        ResolverError::InvalidPackageId { package: release.source_package.clone(), source }
      })?;
      if source_package != *package {
        return Err(ResolverError::RegistryAliasUnsupported {
          package: package.to_string(),
          source_package: release.source_package,
        });
      }
      let fetched = fetch::fetch_github(package, &release.source_rev)
        .map_err(|source| ResolverError::Fetch { package: package.to_string(), source })?;
      let declared_deps = registry_release_declared_dependencies_with_fallback(
        package,
        &fetched.source_dir,
        release.manifest.as_ref(),
      )?;
      Ok(ResolvedSelection {
        registry: Some(release.registry),
        source_package: Some(release.source_package),
        requested_rev: fetched.requested_rev,
        requested_requirement: release.requested_requirement,
        resolved_version: Some(release.resolved_version),
        resolved_commit: fetched.resolved_commit,
        declared_deps,
      })
    },
  })
}

/// Resolve a manifest with an injected commit-resolution function.
///
/// This hook keeps unit tests deterministic and allows future resolver extensions to decouple graph
/// construction from transport concerns.
pub fn resolve_manifest_with<F>(
  manifest: &Manifest,
  recipes: &RecipeStore,
  mut resolve_commit: F,
) -> Result<ResolvedGraph, ResolverError>
where
  F: FnMut(&PackageId, &str) -> Result<String, ResolverError>,
{
  resolve_manifest_with_selector(manifest, recipes, |package, _source, request| {
    let requested = match request {
      ResolveRequest::Rev(rev) | ResolveRequest::Version(rev) => rev.as_str(),
    };
    let resolved_commit = resolve_commit(package, requested)?;
    Ok(ResolvedSelection {
      registry: None,
      source_package: None,
      requested_rev: requested.to_string(),
      requested_requirement: None,
      resolved_version: None,
      resolved_commit,
      declared_deps: Vec::new(),
    })
  })
}

/// Resolve a manifest with an injected request-selection function (exact rev or semver range).
fn resolve_manifest_with_selector<F>(
  manifest: &Manifest,
  recipes: &RecipeStore,
  mut resolve_selection: F,
) -> Result<ResolvedGraph, ResolverError>
where
  F: FnMut(
    &PackageId,
    &DependencySource,
    &ResolveRequest,
  ) -> Result<ResolvedSelection, ResolverError>,
{
  // TODO(phase7): Split graph construction from transitive queue expansion to make semver-range
  // resolution pluggable without rewriting conflict/cycle handling.
  let mut graph = DiGraph::<ResolvedPackage, ()>::new();
  let mut by_id = BTreeMap::<String, NodeIndex>::new();
  let mut queue = VecDeque::<PendingDependency>::new();

  for (raw_id, spec) in &manifest.dependencies {
    let package = PackageId::parse(raw_id)
      .map_err(|source| ResolverError::InvalidPackageId { package: raw_id.clone(), source })?;
    queue.push_back(PendingDependency {
      package,
      source: spec.source.clone(),
      request: match manifest.dependency_requirement(raw_id.as_str()) {
        Some(DependencyRequirementRef::Version(version)) => ResolveRequest::Version(version.into()),
        Some(DependencyRequirementRef::Rev(rev)) => ResolveRequest::Rev(rev.into()),
        None => ResolveRequest::Rev("HEAD".into()),
      },
      dependent: None,
      direct: true,
      requested_by: None,
    });
  }

  while let Some(pending) = queue.pop_front() {
    let selection = resolve_selection(&pending.package, &pending.source, &pending.request)?;
    let requested_rev = selection.requested_rev.clone();
    let resolved_commit = selection.resolved_commit.clone();
    let key = pending.package.to_string();

    let node_idx = if let Some(existing_idx) = by_id.get(&key).copied() {
      let existing = graph.node_weight_mut(existing_idx).expect("existing node");
      if existing.resolved_commit != resolved_commit {
        return Err(ResolverError::VersionConflict(Box::new(VersionConflictError {
          package: key.clone(),
          existing_requested_rev: existing.requested_rev.clone(),
          existing_resolved_commit: existing.resolved_commit.clone(),
          new_requested_rev: requested_rev.clone(),
          new_resolved_commit: resolved_commit,
          requested_by: pending.requested_by.unwrap_or_else(|| "<direct>".to_string()),
        })));
      }
      if pending.direct {
        existing.direct = true;
      }
      if existing.requested_requirement.is_none() && selection.requested_requirement.is_some() {
        existing.requested_requirement = selection.requested_requirement.clone();
      }
      if existing.resolved_version.is_none() && selection.resolved_version.is_some() {
        existing.resolved_version = selection.resolved_version.clone();
      }
      if existing.registry.is_none() && selection.registry.is_some() {
        existing.registry = selection.registry.clone();
      }
      if existing.source_package.is_none() && selection.source_package.is_some() {
        existing.source_package = selection.source_package.clone();
      }
      existing_idx
    } else {
      let recipe = recipes.get(&pending.package);
      let node = ResolvedPackage {
        id: pending.package.clone(),
        source: pending.source.clone(),
        registry: selection.registry.clone(),
        source_package: selection.source_package.clone(),
        requested_rev,
        requested_requirement: selection.requested_requirement.clone(),
        resolved_version: selection.resolved_version.clone(),
        resolved_commit: resolved_commit.clone(),
        recipe_slug: recipe.map(|r| r.slug.clone()),
        header_only: recipe.map(|r| r.is_header_only()).unwrap_or(true),
        direct: pending.direct,
      };
      let idx = graph.add_node(node);
      by_id.insert(key.clone(), idx);

      if let Some(recipe) = recipe {
        for dep in recipe.dep_packages() {
          let dep_id = dep.id().to_string();
          let package = PackageId::parse(&dep_id).map_err(|source| {
            ResolverError::InvalidPackageId { package: dep_id.clone(), source }
          })?;
          let requested_rev = dep.requested_rev().map(ToOwned::to_owned).ok_or_else(|| {
            ResolverError::MissingTransitiveRev { package: key.clone(), dependency: dep_id }
          })?;
          queue.push_back(PendingDependency {
            package,
            source: DependencySource::Github,
            request: ResolveRequest::Rev(requested_rev),
            dependent: Some(idx),
            direct: false,
            requested_by: Some(key.clone()),
          });
        }
      } else {
        for dep in &selection.declared_deps {
          queue.push_back(PendingDependency {
            package: dep.package.clone(),
            source: dep.source.clone(),
            request: dep.request.clone(),
            dependent: Some(idx),
            direct: false,
            requested_by: Some(key.clone()),
          });
        }
      }

      idx
    };

    if let Some(dependent_idx) = pending.dependent {
      if node_idx == dependent_idx {
        return Err(ResolverError::Cycle { package: key });
      }
      if !graph.contains_edge(node_idx, dependent_idx) {
        graph.add_edge(node_idx, dependent_idx, ());
      }
    }
  }

  let resolved = ResolvedGraph { graph, by_id };
  let _ = resolved.build_order()?;
  Ok(resolved)
}

#[derive(Debug, Clone)]
struct PendingDependency {
  package: PackageId,
  source: DependencySource,
  request: ResolveRequest,
  dependent: Option<NodeIndex>,
  direct: bool,
  requested_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolveRequest {
  Rev(String),
  Version(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredDependency {
  package: PackageId,
  source: DependencySource,
  request: ResolveRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedSelection {
  registry: Option<String>,
  source_package: Option<String>,
  requested_rev: String,
  requested_requirement: Option<String>,
  resolved_version: Option<String>,
  resolved_commit: String,
  declared_deps: Vec<DeclaredDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PackageManifestMetadata {
  Missing,
  NotPackageManifest,
  Package { digest: Option<String>, declared_deps: Vec<DeclaredDependency> },
}

fn package_manifest_declared_dependencies(
  package: &PackageId,
  source_dir: &Path,
) -> Result<Vec<DeclaredDependency>, ResolverError> {
  match inspect_package_manifest_metadata(package, source_dir)? {
    PackageManifestMetadata::Missing | PackageManifestMetadata::NotPackageManifest => {
      Ok(Vec::new())
    },
    PackageManifestMetadata::Package { declared_deps, .. } => Ok(declared_deps),
  }
}

fn inspect_package_manifest_metadata(
  package: &PackageId,
  source_dir: &Path,
) -> Result<PackageManifestMetadata, ResolverError> {
  let manifest_path = source_dir.join("joy.toml");
  if !manifest_path.exists() {
    return Ok(PackageManifestMetadata::Missing);
  }

  let doc = ManifestDocument::load(&manifest_path).map_err(|source| {
    ResolverError::PackageManifestLoad {
      package: package.to_string(),
      path: manifest_path.clone(),
      source,
    }
  })?;
  let ManifestDocument::Package(pkg_manifest) = doc else {
    return Ok(PackageManifestMetadata::NotPackageManifest);
  };

  let mut declared = Vec::new();
  for (raw_id, spec) in &pkg_manifest.dependencies {
    let dep_package = PackageId::parse(raw_id)
      .map_err(|source| ResolverError::InvalidPackageId { package: raw_id.clone(), source })?;
    let request = match pkg_manifest.dependency_requirement(raw_id.as_str()) {
      Some(DependencyRequirementRef::Version(version)) => ResolveRequest::Version(version.into()),
      Some(DependencyRequirementRef::Rev(rev)) => ResolveRequest::Rev(rev.into()),
      None => ResolveRequest::Rev("HEAD".into()),
    };
    declared.push(DeclaredDependency {
      package: dep_package,
      source: spec.source.clone(),
      request,
    });
  }

  let digest =
    lockfile::compute_manifest_hash(&manifest_path).ok().map(|hash| format!("sha256:{hash}"));
  Ok(PackageManifestMetadata::Package { digest, declared_deps: declared })
}

fn registry_release_declared_dependencies_with_fallback(
  package: &PackageId,
  source_dir: &Path,
  registry_manifest: Option<&registry::RegistryManifestSummary>,
) -> Result<Vec<DeclaredDependency>, ResolverError> {
  match inspect_package_manifest_metadata(package, source_dir)? {
    PackageManifestMetadata::Package { digest, declared_deps } => {
      if let (Some(local_digest), Some(expected_digest)) =
        (digest.as_deref(), registry_manifest.and_then(|m| m.digest.as_deref()))
        && local_digest != expected_digest
      {
        return Err(ResolverError::PackageMetadataMismatch {
          package: package.to_string(),
          expected_digest: expected_digest.to_string(),
          actual_digest: local_digest.to_string(),
        });
      }
      Ok(declared_deps)
    },
    PackageManifestMetadata::Missing | PackageManifestMetadata::NotPackageManifest => {
      match registry_manifest {
        Some(summary) => registry_manifest_declared_dependencies(summary),
        None => Ok(Vec::new()),
      }
    },
  }
}

fn registry_manifest_declared_dependencies(
  summary: &registry::RegistryManifestSummary,
) -> Result<Vec<DeclaredDependency>, ResolverError> {
  let mut declared = Vec::new();
  for dep in &summary.dependencies {
    let package = PackageId::parse(&dep.id)
      .map_err(|source| ResolverError::InvalidPackageId { package: dep.id.clone(), source })?;
    let request = if let Some(version) = dep.version.as_deref() {
      ResolveRequest::Version(version.to_string())
    } else if let Some(rev) = dep.rev.as_deref() {
      ResolveRequest::Rev(if rev.trim().is_empty() { "HEAD".into() } else { rev.to_string() })
    } else {
      ResolveRequest::Rev("HEAD".into())
    };
    declared.push(DeclaredDependency { package, source: dep.source.clone(), request });
  }
  Ok(declared)
}

#[derive(Debug, Error)]
pub enum ResolverError {
  #[error("invalid package id `{package}`: {source}")]
  InvalidPackageId {
    package: String,
    #[source]
    source: PackageIdError,
  },
  #[error("fetch failed for `{package}`: {source}")]
  Fetch {
    package: String,
    #[source]
    source: fetch::FetchError,
  },
  #[error("failed to load registry index: {source}")]
  RegistryLoad {
    #[source]
    source: registry::RegistryError,
  },
  #[error("failed to resolve registry package `{package}`: {source}")]
  RegistryResolve {
    package: String,
    #[source]
    source: registry::RegistryError,
  },
  #[error(
    "registry package `{package}` currently maps to `{source_package}`, but alias package coordinates are not supported yet"
  )]
  RegistryAliasUnsupported { package: String, source_package: String },
  #[error("failed to load package manifest for `{package}` at `{path}`: {source}")]
  PackageManifestLoad {
    package: String,
    path: std::path::PathBuf,
    #[source]
    source: crate::manifest::ManifestError,
  },
  #[error(
    "registry/package metadata mismatch for `{package}`: registry manifest digest `{expected_digest}` != fetched package manifest digest `{actual_digest}`"
  )]
  PackageMetadataMismatch { package: String, expected_digest: String, actual_digest: String },
  #[error("recipe for `{package}` depends on `{dependency}` without an explicit rev")]
  MissingTransitiveRev { package: String, dependency: String },
  #[error(
    "version conflict for `{}`: {} -> {}, {} -> {} (requested by {})",
    .0.package,
    .0.existing_requested_rev,
    .0.existing_resolved_commit,
    .0.new_requested_rev,
    .0.new_resolved_commit,
    .0.requested_by
  )]
  VersionConflict(Box<VersionConflictError>),
  #[error("dependency cycle detected involving `{package}`")]
  Cycle { package: String },
}

/// Detailed payload for version-conflict reporting.
#[derive(Debug)]
pub struct VersionConflictError {
  pub package: String,
  pub existing_requested_rev: String,
  pub existing_resolved_commit: String,
  pub new_requested_rev: String,
  pub new_resolved_commit: String,
  pub requested_by: String,
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;
  use std::fs;

  use tempfile::TempDir;

  use super::{
    DeclaredDependency, ResolveRequest, ResolvedSelection, ResolverError,
    registry_release_declared_dependencies_with_fallback, resolve_manifest_with,
    resolve_manifest_with_selector,
  };
  use crate::manifest::{DependencySource, DependencySpec, Manifest, ProjectSection};
  use crate::package_id::PackageId;
  use crate::recipes::RecipeStore;
  use crate::registry::{RegistryManifestDependency, RegistryManifestSummary};

  #[test]
  fn builds_dag_and_orders_dependencies_before_dependents() {
    let temp = TempDir::new().expect("tempdir");
    write_recipe_store(
      temp.path(),
      &[
        (
          "fmt",
          "fmtlib/fmt",
          r#"
kind = "cmake"
[deps]
packages = [{ id = "madler/zlib", rev = "1.3.1" }]
[headers]
include_roots = ["include"]
[link]
libs = ["fmt"]
"#,
        ),
        (
          "nlohmann_json",
          "nlohmann/json",
          r#"
kind = "header_only"
[headers]
include_roots = ["include"]
"#,
        ),
        (
          "zlib",
          "madler/zlib",
          r#"
kind = "cmake"
[headers]
include_roots = ["include"]
[link]
libs = ["z"]
"#,
        ),
      ],
    );
    let recipes = RecipeStore::load_from_dir(temp.path()).expect("recipes");

    let manifest = manifest_with_deps([("fmtlib/fmt", "11.0.2"), ("nlohmann/json", "HEAD")]);

    let resolved = resolve_manifest_with(&manifest, &recipes, |pkg, rev| {
      Ok(format!("{}::{}", pkg.as_str(), rev))
    })
    .expect("resolve");

    let order = resolved.build_order_ids().expect("toposort");
    let zlib_idx = order.iter().position(|id| id == "madler/zlib").expect("zlib in order");
    let fmt_idx = order.iter().position(|id| id == "fmtlib/fmt").expect("fmt in order");
    assert!(zlib_idx < fmt_idx, "dependency should come before dependent: {order:?}");
    assert!(order.iter().any(|id| id == "nlohmann/json"));

    let fmt = resolved.package("fmtlib/fmt").expect("fmt package");
    assert_eq!(fmt.recipe_slug.as_deref(), Some("fmt"));
    assert!(!fmt.header_only);
    assert_eq!(fmt.requested_requirement, None);
    assert_eq!(fmt.resolved_version, None);
    let json = resolved.package("nlohmann/json").expect("json package");
    assert!(json.header_only);
    assert!(json.direct);
  }

  #[test]
  fn resolves_direct_semver_dependency_and_records_selected_version_metadata() {
    let temp = TempDir::new().expect("tempdir");
    write_recipe_store(
      temp.path(),
      &[(
        "fmt",
        "fmtlib/fmt",
        r#"
kind = "header_only"
[headers]
include_roots = ["include"]
"#,
      )],
    );
    let recipes = RecipeStore::load_from_dir(temp.path()).expect("recipes");
    let mut manifest = manifest_with_deps([]);
    manifest.dependencies.insert(
      "fmtlib/fmt".into(),
      DependencySpec {
        source: DependencySource::Github,
        rev: String::new(),
        version: Some("^11".into()),
      },
    );

    let resolved =
      resolve_manifest_with_selector(&manifest, &recipes, |pkg, _source, request| match request {
        ResolveRequest::Version(req) if pkg.as_str() == "fmtlib/fmt" => Ok(ResolvedSelection {
          registry: None,
          source_package: None,
          requested_rev: "v11.1.2".into(),
          requested_requirement: Some(req.clone()),
          resolved_version: Some("11.1.2".into()),
          resolved_commit: "commit-fmt-11-1-2".into(),
          declared_deps: Vec::new(),
        }),
        other => panic!("unexpected request: {other:?}"),
      })
      .expect("resolve semver");

    let fmt = resolved.package("fmtlib/fmt").expect("fmt");
    assert_eq!(fmt.requested_rev, "v11.1.2");
    assert_eq!(fmt.requested_requirement.as_deref(), Some("^11"));
    assert_eq!(fmt.resolved_version.as_deref(), Some("11.1.2"));
    assert_eq!(fmt.resolved_commit, "commit-fmt-11-1-2");
  }

  #[test]
  fn reports_conflict_when_same_package_resolves_to_different_commits() {
    let temp = TempDir::new().expect("tempdir");
    write_recipe_store(
      temp.path(),
      &[
        (
          "pkg_a",
          "org/pkg-a",
          r#"
[deps]
packages = [{ id = "org/common", rev = "v1" }]
"#,
        ),
        (
          "pkg_b",
          "org/pkg-b",
          r#"
[deps]
packages = [{ id = "org/common", rev = "v2" }]
"#,
        ),
        ("common", "org/common", ""),
      ],
    );
    let recipes = RecipeStore::load_from_dir(temp.path()).expect("recipes");
    let manifest = manifest_with_deps([("org/pkg-a", "HEAD"), ("org/pkg-b", "HEAD")]);

    let err = resolve_manifest_with(&manifest, &recipes, |pkg, rev| match (pkg.as_str(), rev) {
      ("org/common", "v1") => Ok("commit-111".into()),
      ("org/common", "v2") => Ok("commit-222".into()),
      _ => Ok(format!("{}::{rev}", pkg.as_str())),
    })
    .expect_err("conflict expected");

    match err {
      ResolverError::VersionConflict(details) => assert_eq!(details.package, "org/common"),
      other => panic!("unexpected error: {other}"),
    }
  }

  #[test]
  fn reports_cycle_from_recipe_dependencies() {
    let temp = TempDir::new().expect("tempdir");
    write_recipe_store(
      temp.path(),
      &[
        (
          "a",
          "cycle/a",
          r#"
[deps]
packages = [{ id = "cycle/b", rev = "HEAD" }]
"#,
        ),
        (
          "b",
          "cycle/b",
          r#"
[deps]
packages = [{ id = "cycle/a", rev = "HEAD" }]
"#,
        ),
      ],
    );
    let recipes = RecipeStore::load_from_dir(temp.path()).expect("recipes");
    let manifest = manifest_with_deps([("cycle/a", "HEAD")]);

    let err =
      resolve_manifest_with(&manifest, &recipes, |pkg, rev| Ok(format!("{}::{rev}", pkg.as_str())))
        .expect_err("cycle expected");

    match err {
      ResolverError::Cycle { package } => assert!(package.starts_with("cycle/")),
      other => panic!("unexpected error: {other}"),
    }
  }

  #[test]
  fn expands_transitive_dependencies_from_declared_package_manifest_edges() {
    let temp = TempDir::new().expect("tempdir");
    write_recipe_store(temp.path(), &[]);
    let recipes = RecipeStore::load_from_dir(temp.path()).expect("recipes");
    let manifest = manifest_with_deps([("harnesslabs/igneous", "HEAD")]);

    let resolved = resolve_manifest_with_selector(&manifest, &recipes, |pkg, _source, request| {
      match (pkg.as_str(), request) {
        ("harnesslabs/igneous", ResolveRequest::Rev(rev)) if rev == "HEAD" => {
          Ok(ResolvedSelection {
            registry: None,
            source_package: None,
            requested_rev: "HEAD".into(),
            requested_requirement: None,
            resolved_version: None,
            resolved_commit: "commit-igneous".into(),
            declared_deps: vec![DeclaredDependency {
              package: PackageId::parse("xsimd/xsimd").expect("package id"),
              source: DependencySource::Github,
              request: ResolveRequest::Rev("HEAD".into()),
            }],
          })
        },
        ("xsimd/xsimd", ResolveRequest::Rev(rev)) if rev == "HEAD" => Ok(ResolvedSelection {
          registry: None,
          source_package: None,
          requested_rev: "HEAD".into(),
          requested_requirement: None,
          resolved_version: None,
          resolved_commit: "commit-xsimd".into(),
          declared_deps: Vec::new(),
        }),
        other => panic!("unexpected request: {other:?}"),
      }
    })
    .expect("resolve");

    let order = resolved.build_order_ids().expect("toposort");
    let xsimd_idx = order.iter().position(|id| id == "xsimd/xsimd").expect("xsimd in order");
    let igneous_idx =
      order.iter().position(|id| id == "harnesslabs/igneous").expect("igneous in order");
    assert!(xsimd_idx < igneous_idx, "transitive dep should precede parent: {order:?}");
    assert_eq!(
      resolved.dependency_ids("harnesslabs/igneous").expect("igneous deps"),
      vec!["xsimd/xsimd".to_string()]
    );
  }

  #[test]
  fn curated_recipe_dependencies_take_precedence_over_declared_package_manifest_edges() {
    let temp = TempDir::new().expect("tempdir");
    write_recipe_store(
      temp.path(),
      &[(
        "igneous",
        "harnesslabs/igneous",
        r#"
[deps]
packages = [{ id = "curated/transitive", rev = "v1" }]
"#,
      )],
    );
    let recipes = RecipeStore::load_from_dir(temp.path()).expect("recipes");
    let manifest = manifest_with_deps([("harnesslabs/igneous", "HEAD")]);

    let resolved = resolve_manifest_with_selector(&manifest, &recipes, |pkg, _source, request| {
      match (pkg.as_str(), request) {
        ("harnesslabs/igneous", ResolveRequest::Rev(rev)) if rev == "HEAD" => {
          Ok(ResolvedSelection {
            registry: None,
            source_package: None,
            requested_rev: "HEAD".into(),
            requested_requirement: None,
            resolved_version: None,
            resolved_commit: "commit-igneous".into(),
            declared_deps: vec![DeclaredDependency {
              package: PackageId::parse("ignored/from-manifest").expect("package id"),
              source: DependencySource::Github,
              request: ResolveRequest::Rev("HEAD".into()),
            }],
          })
        },
        ("curated/transitive", ResolveRequest::Rev(rev)) if rev == "v1" => Ok(ResolvedSelection {
          registry: None,
          source_package: None,
          requested_rev: "v1".into(),
          requested_requirement: None,
          resolved_version: None,
          resolved_commit: "commit-curated-v1".into(),
          declared_deps: Vec::new(),
        }),
        other => panic!("unexpected request: {other:?}"),
      }
    })
    .expect("resolve");

    assert!(resolved.package("curated/transitive").is_some());
    assert!(resolved.package("ignored/from-manifest").is_none());
    assert_eq!(
      resolved.dependency_ids("harnesslabs/igneous").expect("igneous deps"),
      vec!["curated/transitive".to_string()]
    );
  }

  #[test]
  fn registry_manifest_digest_mismatch_is_reported_when_package_manifest_exists() {
    let temp = TempDir::new().expect("tempdir");
    std::fs::write(
      temp.path().join("joy.toml"),
      r#"[package]
id = "harnesslabs/igneous"
version = "0.3.0"
kind = "header_only"

[headers]
include_roots = ["include"]

[dependencies]
"xsimd/xsimd" = { source = "github", rev = "HEAD" }
"#,
    )
    .expect("write package manifest");

    let package = PackageId::parse("harnesslabs/igneous").expect("package id");
    let err = registry_release_declared_dependencies_with_fallback(
      &package,
      temp.path(),
      Some(&RegistryManifestSummary {
        digest: Some("sha256:not-the-real-digest".into()),
        kind: Some("header_only".into()),
        headers_include_roots: vec!["include".into()],
        dependencies: vec![RegistryManifestDependency {
          id: "xsimd/xsimd".into(),
          source: DependencySource::Github,
          rev: Some("HEAD".into()),
          version: None,
        }],
      }),
    )
    .expect_err("mismatch expected");

    match err {
      ResolverError::PackageMetadataMismatch { package, .. } => {
        assert_eq!(package, "harnesslabs/igneous")
      },
      other => panic!("unexpected error: {other}"),
    }
  }

  fn manifest_with_deps<const N: usize>(deps: [(&str, &str); N]) -> Manifest {
    let mut dependencies = BTreeMap::new();
    for (id, rev) in deps {
      dependencies.insert(
        id.to_string(),
        DependencySpec { source: DependencySource::Github, rev: rev.to_string(), version: None },
      );
    }
    Manifest {
      project: ProjectSection {
        name: "demo".into(),
        version: "0.1.0".into(),
        cpp_standard: "c++20".into(),
        entry: "src/main.cpp".into(),
        extra_sources: Vec::new(),
        include_dirs: Vec::new(),
        targets: Vec::new(),
      },
      dependencies,
    }
  }

  fn write_recipe_store(root: &std::path::Path, entries: &[(&str, &str, &str)]) {
    fs::create_dir_all(root.join("packages")).expect("packages dir");

    let mut index = String::from("version = 1\n\n");
    for (slug, id, _) in entries {
      index.push_str("[[packages]]\n");
      index.push_str(&format!("id = \"{id}\"\n"));
      index.push_str(&format!("slug = \"{slug}\"\n\n"));
    }
    fs::write(root.join("index.toml"), index).expect("write index");

    for (slug, id, body) in entries {
      let mut recipe = format!("id = \"{id}\"\nslug = \"{slug}\"\nsource = \"github\"\n");
      if !body.trim().is_empty() {
        if !body.trim_start().starts_with('\n') {
          recipe.push('\n');
        }
        recipe.push_str(body.trim_start_matches('\n'));
        if !recipe.ends_with('\n') {
          recipe.push('\n');
        }
      }
      fs::write(root.join("packages").join(format!("{slug}.toml")), recipe).expect("write recipe");
    }
  }
}
