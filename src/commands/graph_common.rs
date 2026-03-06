use std::collections::BTreeMap;
use std::path::Path;

use crate::error::JoyError;
use crate::lockfile;
use crate::manifest::Manifest;
use crate::resolver;

#[derive(Debug, Clone)]
pub(crate) struct ProvenanceOverlay {
  pub metadata_source: Option<String>,
  pub package_manifest_digest: Option<String>,
  pub declared_deps_source: Option<String>,
}

pub(crate) fn load_fresh_lockfile_provenance_overlay(
  cwd: &Path,
  manifest_path: &Path,
) -> Option<BTreeMap<String, ProvenanceOverlay>> {
  let lockfile_path = cwd.join("joy.lock");
  let lock = lockfile::Lockfile::load(&lockfile_path).ok()?;
  let manifest_hash = lockfile::compute_manifest_hash(manifest_path).ok()?;
  if lock.manifest_hash != manifest_hash {
    return None;
  }

  Some(
    lock
      .packages
      .into_iter()
      .map(|pkg| {
        (
          pkg.id,
          ProvenanceOverlay {
            metadata_source: pkg.metadata_source,
            package_manifest_digest: pkg.package_manifest_digest,
            declared_deps_source: pkg.declared_deps_source,
          },
        )
      })
      .collect(),
  )
}

pub(crate) fn map_resolver_error(command: &'static str, err: resolver::ResolverError) -> JoyError {
  let code = match &err {
    resolver::ResolverError::Fetch { source, .. } if source.is_offline_cache_miss() => {
      "offline_cache_miss"
    },
    resolver::ResolverError::Fetch { source, .. } if source.is_offline_network_disabled() => {
      "offline_network_disabled"
    },
    resolver::ResolverError::Fetch { source, .. } if source.is_invalid_version_requirement() => {
      "invalid_version_requirement"
    },
    resolver::ResolverError::Fetch { source, .. } if source.is_version_not_found() => {
      "version_not_found"
    },
    resolver::ResolverError::RegistryLoad { source }
      if source.is_offline_cache_miss() || source.is_not_configured() =>
    {
      if source.is_offline_cache_miss() { "offline_cache_miss" } else { "registry_not_configured" }
    },
    resolver::ResolverError::RegistryResolve { source, .. }
      if source.is_package_not_found() || source.is_version_not_found() =>
    {
      if source.is_package_not_found() { "registry_package_not_found" } else { "version_not_found" }
    },
    resolver::ResolverError::RegistryResolve { source, .. }
      if source.is_invalid_version_requirement() =>
    {
      "invalid_version_requirement"
    },
    resolver::ResolverError::RegistryLoad { .. }
    | resolver::ResolverError::RegistryResolve { .. } => "registry_load_failed",
    resolver::ResolverError::RegistryAliasUnsupported { .. } => "registry_alias_unsupported",
    resolver::ResolverError::PackageMetadataMismatch { .. } => "package_metadata_mismatch",
    resolver::ResolverError::MissingSourceField { .. } => "invalid_dependency_source",
    resolver::ResolverError::UnsupportedSource { .. } => "source_backend_unsupported",
    _ => "dependency_resolve_failed",
  };
  JoyError::new(command, code, err.to_string(), 1)
}

pub(crate) fn validate_locked_graph_lockfile(
  command: &'static str,
  manifest: &Manifest,
  manifest_path: &Path,
  lockfile_path: &Path,
) -> Result<lockfile::Lockfile, JoyError> {
  if !lockfile_path.is_file() {
    let message = if command == "outdated" {
      format!(
        "`joy outdated` requires `{}`; create or refresh it with `joy sync --update-lock`",
        lockfile_path.display()
      )
    } else {
      format!(
        "`--locked` requires `{}` to exist; create or refresh it with `joy sync --update-lock`",
        lockfile_path.display()
      )
    };
    return Err(JoyError::new(command, "lockfile_missing", message, 1));
  }

  let lock = lockfile::Lockfile::load(lockfile_path)
    .map_err(|err| JoyError::new(command, "lockfile_parse_error", err.to_string(), 1))?;
  let manifest_hash = lockfile::compute_manifest_hash(manifest_path)
    .map_err(|err| JoyError::new(command, "lockfile_hash_failed", err.to_string(), 1))?;
  if lock.manifest_hash != manifest_hash {
    return Err(JoyError::new(
      command,
      "lockfile_stale",
      "joy.lock manifest hash does not match joy.toml; rerun `joy sync --update-lock`".to_string(),
      1,
    ));
  }
  if !manifest.dependencies.is_empty() && lock.packages.is_empty() {
    return Err(JoyError::new(
      command,
      "lockfile_incomplete",
      "joy.lock package metadata is missing for current dependencies; rerun `joy sync --update-lock`"
        .to_string(),
      1,
    ));
  }

  Ok(lock)
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use tempfile::TempDir;

  use super::{load_fresh_lockfile_provenance_overlay, validate_locked_graph_lockfile};
  use crate::lockfile::{self, LockedPackage, Lockfile};
  use crate::manifest::{Manifest, ProjectSection};

  #[test]
  fn locked_validation_reports_missing_lockfile() {
    let temp = TempDir::new().expect("tempdir");
    let manifest = test_manifest();
    let err = validate_locked_graph_lockfile(
      "tree",
      &manifest,
      &temp.path().join("joy.toml"),
      &temp.path().join("joy.lock"),
    )
    .expect_err("missing lockfile should error");
    assert_eq!(err.code, "lockfile_missing");
  }

  #[test]
  fn locked_validation_reports_stale_lockfile() {
    let temp = TempDir::new().expect("tempdir");
    let manifest_path = temp.path().join("joy.toml");
    std::fs::write(
      &manifest_path,
      r#"[project]
name = "demo"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"
"#,
    )
    .expect("write manifest");

    let lock = Lockfile {
      version: Lockfile::VERSION,
      manifest_hash: "not-the-hash".into(),
      generated_by: lockfile::generated_by_string(),
      packages: Vec::new(),
    };
    let lock_path = temp.path().join("joy.lock");
    lock.save(&lock_path).expect("save lockfile");

    let err = validate_locked_graph_lockfile("tree", &test_manifest(), &manifest_path, &lock_path)
      .expect_err("stale lockfile should error");
    assert_eq!(err.code, "lockfile_stale");
  }

  #[test]
  fn provenance_overlay_loads_when_lockfile_is_fresh() {
    let temp = TempDir::new().expect("tempdir");
    let manifest_path = temp.path().join("joy.toml");
    std::fs::write(
      &manifest_path,
      r#"[project]
name = "demo"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"
"#,
    )
    .expect("write manifest");

    let lock = Lockfile {
      version: Lockfile::VERSION,
      manifest_hash: lockfile::compute_manifest_hash(&manifest_path).expect("hash"),
      generated_by: lockfile::generated_by_string(),
      packages: vec![LockedPackage {
        id: "nlohmann/json".into(),
        source: "github".into(),
        source_git: None,
        source_path: None,
        source_url: None,
        source_checksum_sha256: None,
        registry: None,
        source_package: None,
        requested_rev: "HEAD".into(),
        requested_requirement: None,
        resolved_version: None,
        resolved_commit: "abc".into(),
        resolved_ref: None,
        recipe: None,
        metadata_source: Some("recipe".into()),
        package_manifest_digest: Some("sha256:abc".into()),
        declared_deps_source: Some("recipe".into()),
        header_only: true,
        header_roots: vec!["include".into()],
        deps: Vec::new(),
        abi_hash: String::new(),
        libs: Vec::new(),
        linkage: None,
      }],
    };
    lock.save(&temp.path().join("joy.lock")).expect("save lockfile");

    let overlay =
      load_fresh_lockfile_provenance_overlay(temp.path(), &manifest_path).expect("overlay");
    let pkg = overlay.get("nlohmann/json").expect("pkg overlay");
    assert_eq!(pkg.metadata_source.as_deref(), Some("recipe"));
    assert_eq!(pkg.package_manifest_digest.as_deref(), Some("sha256:abc"));
    assert_eq!(pkg.declared_deps_source.as_deref(), Some("recipe"));
  }

  fn test_manifest() -> Manifest {
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
      dependencies: BTreeMap::new(),
    }
  }
}
