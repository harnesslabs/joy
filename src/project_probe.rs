use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::lockfile;
use crate::manifest::{Manifest, ManifestDocument};

#[derive(Debug, Clone)]
pub(crate) struct ArtifactProbe {
  pub path: PathBuf,
  pub present: bool,
  pub parse_error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LockfileProbe {
  pub present: bool,
  pub path: PathBuf,
  pub fresh: Option<bool>,
  pub package_count: Option<usize>,
  pub parse_error: Option<String>,
  pub version: Option<u32>,
  pub generated_by: Option<String>,
  pub manifest_hash: Option<String>,
  pub package_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyMetadataProbe {
  pub package_count: usize,
  pub metadata_source_counts: BTreeMap<String, u64>,
  pub declared_deps_source_counts: BTreeMap<String, u64>,
  pub package_manifest_count: u64,
  pub registry_manifest_count: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectProbe {
  pub present: bool,
  pub ok: bool,
  pub manifest_parse_error: Option<String>,
  pub project_root: PathBuf,
  pub manifest_path: PathBuf,
  pub manifest_kind: String,
  pub direct_dependency_count: usize,
  pub roots: Vec<String>,
  pub joy_root: PathBuf,
  pub state_dir: PathBuf,
  pub build_dir: PathBuf,
  pub dependency_graph: ArtifactProbe,
  pub dependency_graph_json: Option<Value>,
  pub dependency_graph_package_count: Option<usize>,
  pub root_compile_commands: ArtifactProbe,
  pub target_compile_commands: Vec<String>,
  pub lockfile: LockfileProbe,
  pub dependency_metadata: Option<DependencyMetadataProbe>,
  pub warnings: Vec<String>,
  pub hints: Vec<String>,
}

pub(crate) fn probe(cwd: &Path) -> ProjectProbe {
  let manifest_path = cwd.join("joy.toml");
  let joy_root = cwd.join(".joy");
  let state_dir = joy_root.join("state");
  let build_dir = joy_root.join("build");
  let graph_path = state_dir.join("dependency-graph.json");
  let root_compile_db = cwd.join("compile_commands.json");
  let lockfile_path = cwd.join("joy.lock");

  let mut probe = ProjectProbe {
    present: manifest_path.is_file(),
    ok: true,
    manifest_parse_error: None,
    project_root: cwd.to_path_buf(),
    manifest_path: manifest_path.clone(),
    manifest_kind: "unknown".to_string(),
    direct_dependency_count: 0,
    roots: Vec::new(),
    joy_root: joy_root.clone(),
    state_dir: state_dir.clone(),
    build_dir: build_dir.clone(),
    dependency_graph: ArtifactProbe {
      path: graph_path.clone(),
      present: graph_path.is_file(),
      parse_error: None,
    },
    dependency_graph_json: None,
    dependency_graph_package_count: None,
    root_compile_commands: ArtifactProbe {
      path: root_compile_db.clone(),
      present: root_compile_db.is_file(),
      parse_error: None,
    },
    target_compile_commands: list_target_compile_commands(&build_dir),
    lockfile: LockfileProbe {
      present: lockfile_path.is_file(),
      path: lockfile_path.clone(),
      fresh: None,
      package_count: None,
      parse_error: None,
      version: None,
      generated_by: None,
      manifest_hash: None,
      package_ids: Vec::new(),
    },
    dependency_metadata: None,
    warnings: Vec::new(),
    hints: Vec::new(),
  };

  if !probe.present {
    return probe;
  }

  match ManifestDocument::load(&manifest_path) {
    Ok(doc) => {
      let (kind, count, roots, project_manifest) = match doc {
        ManifestDocument::Project(manifest) => {
          let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
          roots.sort();
          ("project".to_string(), manifest.dependencies.len(), roots, Some(manifest))
        },
        ManifestDocument::Workspace(ws) => {
          ("workspace".to_string(), ws.workspace.members.len(), Vec::new(), None)
        },
        ManifestDocument::Package(pkg) => {
          let mut roots = pkg.dependencies.keys().cloned().collect::<Vec<_>>();
          roots.sort();
          ("package".to_string(), pkg.dependencies.len(), roots, None)
        },
      };
      probe.manifest_kind = kind;
      probe.direct_dependency_count = count;
      probe.roots = roots;

      let manifest_hash = project_manifest
        .as_ref()
        .and_then(|_| lockfile::compute_manifest_hash(&manifest_path).ok());
      inspect_lockfile(&mut probe, manifest_hash.as_deref());
      inspect_graph(&mut probe);
      inspect_compile_db_expectations(&mut probe, project_manifest.as_ref());
    },
    Err(err) => {
      probe.ok = false;
      let parse_error = err.to_string();
      probe.manifest_parse_error = Some(parse_error.clone());
      probe.warnings.push(format!("Project manifest parse failed: {parse_error}"));
      probe.hints.push("Fix `joy.toml` parse/validation errors, then rerun `joy doctor`".into());
      inspect_graph(&mut probe);
      inspect_lockfile(&mut probe, None);
    },
  }

  probe
}

fn inspect_graph(probe: &mut ProjectProbe) {
  if !probe.dependency_graph.present {
    return;
  }
  match fs::read(&probe.dependency_graph.path)
    .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(invalid_json_io))
  {
    Ok(value) => {
      probe.dependency_graph_package_count =
        value.get("packages").and_then(|v| v.as_array()).map(|arr| arr.len());
      probe.dependency_graph_json = Some(value);
    },
    Err(err) => {
      probe.ok = false;
      probe.dependency_graph.parse_error = Some(err.to_string());
      probe.warnings.push("Dependency graph artifact exists but failed to parse as JSON".into());
      probe.hints.push("Rerun `joy sync` to regenerate `.joy/state/dependency-graph.json`".into());
    },
  }
}

fn inspect_lockfile(probe: &mut ProjectProbe, manifest_hash: Option<&str>) {
  if !probe.lockfile.present {
    return;
  }

  match lockfile::Lockfile::load(&probe.lockfile.path) {
    Ok(lock) => {
      probe.lockfile.version = Some(lock.version);
      probe.lockfile.generated_by = Some(lock.generated_by.clone());
      probe.lockfile.manifest_hash = Some(lock.manifest_hash.clone());
      probe.lockfile.package_count = Some(lock.packages.len());
      let mut ids = lock.packages.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
      ids.sort();
      ids.dedup();
      probe.lockfile.package_ids = ids;

      if let Some(expected_hash) = manifest_hash {
        let fresh = expected_hash == lock.manifest_hash;
        probe.lockfile.fresh = Some(fresh);
        if !fresh {
          probe.ok = false;
          probe.warnings.push("joy.lock is stale (manifest hash mismatch)".into());
          probe.hints.push(
            "Run `joy sync --update-lock` to refresh lockfile and graph/editor artifacts".into(),
          );
        }
      }

      let mut metadata_source_counts = BTreeMap::<String, u64>::new();
      let mut declared_deps_source_counts = BTreeMap::<String, u64>::new();
      for pkg in &lock.packages {
        *metadata_source_counts
          .entry(pkg.metadata_source.clone().unwrap_or_else(|| "unknown".into()))
          .or_default() += 1;
        *declared_deps_source_counts
          .entry(pkg.declared_deps_source.clone().unwrap_or_else(|| "unknown".into()))
          .or_default() += 1;
      }
      let package_manifest_count =
        metadata_source_counts.get("package_manifest").copied().unwrap_or_default();
      let registry_manifest_count =
        metadata_source_counts.get("registry_manifest").copied().unwrap_or_default();
      let missing_metadata = metadata_source_counts.get("none").copied().unwrap_or_default();
      if missing_metadata > 0 {
        probe.warnings.push(format!(
          "{missing_metadata} locked package(s) have no package metadata provenance (`metadata_source = none`)"
        ));
        probe.hints.push(
          "Nested dependency expansion may rely on recipes or registry summaries for those packages"
            .into(),
        );
      }
      probe.dependency_metadata = Some(DependencyMetadataProbe {
        package_count: lock.packages.len(),
        metadata_source_counts,
        declared_deps_source_counts,
        package_manifest_count,
        registry_manifest_count,
      });
    },
    Err(err) => {
      probe.ok = false;
      probe.lockfile.parse_error = Some(err.to_string());
      probe.warnings.push(format!("joy.lock parse failed: {err}"));
      probe.hints.push("Regenerate lockfile with `joy sync --update-lock`".into());
    },
  }
}

fn inspect_compile_db_expectations(probe: &mut ProjectProbe, manifest: Option<&Manifest>) {
  if let Some(manifest) = manifest
    && !manifest.dependencies.is_empty()
  {
    if !probe.dependency_graph.present {
      probe
        .warnings
        .push("Dependency graph artifact is missing (`.joy/state/dependency-graph.json`)".into());
      probe.hints.push("Run `joy sync` or `joy build` to materialize dependency state".into());
    }
    if !probe.root_compile_commands.present {
      probe.warnings.push(
        "Root `compile_commands.json` is missing; editors may not resolve dependency includes"
          .into(),
      );
      probe.hints.push(
        "Run `joy sync` or `joy build`; if a toolchain is missing, install a compiler + `ninja` so compile DB generation can run"
          .into(),
      );
    }
  }

  if probe.dependency_graph.parse_error.is_some() {
    probe.ok = false;
  }
}

fn list_target_compile_commands(build_dir: &Path) -> Vec<String> {
  if !build_dir.is_dir() {
    return Vec::new();
  }
  let mut entries = fs::read_dir(build_dir)
    .ok()
    .into_iter()
    .flatten()
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .filter(|path| {
      path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("compile_commands.") && n.ends_with(".json"))
    })
    .map(|path| path.display().to_string())
    .collect::<Vec<_>>();
  entries.sort();
  entries
}

fn invalid_json_io(err: serde_json::Error) -> std::io::Error {
  std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid json: {err}"))
}

#[cfg(test)]
mod tests {
  use super::probe;
  use crate::lockfile::{self, LockedPackage, Lockfile};
  use tempfile::TempDir;

  #[test]
  fn missing_manifest_reports_not_present_project() {
    let temp = TempDir::new().expect("tempdir");
    let result = probe(temp.path());
    assert!(!result.present);
    assert!(result.ok);
    assert_eq!(result.manifest_kind, "unknown");
    assert!(result.lockfile.parse_error.is_none());
    assert!(result.dependency_metadata.is_none());
  }

  #[test]
  fn lockfile_metadata_counts_are_derived_for_project_manifest() {
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
      manifest_hash: lockfile::compute_manifest_hash(&manifest_path).expect("manifest hash"),
      generated_by: lockfile::generated_by_string(),
      packages: vec![
        LockedPackage {
          id: "demo/one".into(),
          source: "github".into(),
          registry: None,
          source_package: None,
          requested_rev: "HEAD".into(),
          requested_requirement: None,
          resolved_version: None,
          resolved_commit: "abc".into(),
          resolved_ref: None,
          recipe: None,
          metadata_source: Some("recipe".into()),
          package_manifest_digest: None,
          declared_deps_source: Some("recipe".into()),
          header_only: true,
          header_roots: vec!["include".into()],
          deps: Vec::new(),
          abi_hash: String::new(),
          libs: Vec::new(),
          linkage: None,
        },
        LockedPackage {
          id: "demo/two".into(),
          source: "github".into(),
          registry: None,
          source_package: None,
          requested_rev: "HEAD".into(),
          requested_requirement: None,
          resolved_version: None,
          resolved_commit: "def".into(),
          resolved_ref: None,
          recipe: None,
          metadata_source: Some("none".into()),
          package_manifest_digest: None,
          declared_deps_source: Some("none".into()),
          header_only: true,
          header_roots: vec!["include".into()],
          deps: Vec::new(),
          abi_hash: String::new(),
          libs: Vec::new(),
          linkage: None,
        },
      ],
    };
    lock.save(&temp.path().join("joy.lock")).expect("save lockfile");

    let result = probe(temp.path());
    assert!(result.present);
    assert_eq!(result.manifest_kind, "project");
    assert!(result.lockfile.present);
    assert_eq!(result.lockfile.fresh, Some(true));
    assert_eq!(result.lockfile.package_count, Some(2));

    let dep_meta = result.dependency_metadata.expect("dependency metadata");
    assert_eq!(dep_meta.package_count, 2);
    assert_eq!(dep_meta.metadata_source_counts.get("recipe").copied(), Some(1));
    assert_eq!(dep_meta.metadata_source_counts.get("none").copied(), Some(1));
    assert_eq!(dep_meta.declared_deps_source_counts.get("recipe").copied(), Some(1));
    assert_eq!(dep_meta.declared_deps_source_counts.get("none").copied(), Some(1));
  }
}
