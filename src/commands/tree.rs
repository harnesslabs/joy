use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::env;

use super::graph_common::{
  ProvenanceOverlay, load_fresh_lockfile_provenance_overlay, map_resolver_error,
  validate_locked_graph_lockfile,
};
use crate::cli::{RuntimeFlags, TreeArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::lockfile;
use crate::manifest::Manifest;
use crate::output::HumanMessageBuilder;
use crate::recipes::RecipeStore;
use crate::resolver;

pub fn handle(args: TreeArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("tree", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "tree",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }

  let manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("tree", "manifest_parse_error", err.to_string(), 1))?;
  if args.locked {
    return handle_locked_tree(&cwd, &manifest_path, &manifest);
  }
  let provenance_overlay = load_fresh_lockfile_provenance_overlay(&cwd, &manifest_path);
  let recipes = RecipeStore::load_default()
    .map_err(|err| JoyError::new("tree", "recipe_load_failed", err.to_string(), 1))?;
  let resolved = resolver::resolve_manifest(&manifest, &recipes)
    .map_err(|err| map_resolver_error("tree", err))?;

  let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
  roots.sort();

  let mut packages = resolved
    .packages()
    .map(|pkg| {
      let deps = resolved.dependency_ids(pkg.id.as_str()).unwrap_or_default();
      let overlay = provenance_overlay.as_ref().and_then(|m| m.get(pkg.id.as_str()));
      let metadata_source = pkg
        .recipe_slug
        .as_ref()
        .map(|_| "recipe".to_string())
        .or_else(|| overlay.and_then(|p| p.metadata_source.clone()));
      let declared_deps_source = pkg
        .recipe_slug
        .as_ref()
        .map(|_| "recipe".to_string())
        .or_else(|| overlay.and_then(|p| p.declared_deps_source.clone()));
      let package_manifest_digest = overlay.and_then(|p| p.package_manifest_digest.clone());
      json!({
        "id": pkg.id.to_string(),
        "source": match pkg.source {
          crate::manifest::DependencySource::Github => "github",
          crate::manifest::DependencySource::Registry => "registry",
        },
        "registry": pkg.registry,
        "source_package": pkg.source_package,
        "direct": pkg.direct,
        "header_only": pkg.header_only,
        "requested_rev": pkg.requested_rev,
        "requested_requirement": pkg.requested_requirement,
        "resolved_version": pkg.resolved_version,
        "resolved_commit": pkg.resolved_commit,
        "recipe": pkg.recipe_slug,
        "metadata_source": metadata_source,
        "package_manifest_digest": package_manifest_digest,
        "declared_deps_source": declared_deps_source,
        "deps": deps,
      })
    })
    .collect::<Vec<_>>();
  packages.sort_by(|a, b| {
    a.get("id").and_then(|v| v.as_str()).cmp(&b.get("id").and_then(|v| v.as_str()))
  });

  let human = render_tree_human(&resolved, &roots, provenance_overlay.as_ref());

  Ok(CommandOutput::new(
    "tree",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "roots": roots,
      "packages": packages,
    }),
  ))
}

fn handle_locked_tree(
  cwd: &std::path::Path,
  manifest_path: &std::path::Path,
  manifest: &Manifest,
) -> Result<CommandOutput, JoyError> {
  let lockfile_path = cwd.join("joy.lock");
  let lock = validate_locked_graph_lockfile("tree", manifest, manifest_path, &lockfile_path)?;

  let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
  roots.sort();
  let by_id = lock.packages.iter().map(|pkg| (pkg.id.clone(), pkg)).collect::<BTreeMap<_, _>>();

  let mut packages = lock
    .packages
    .iter()
    .map(|pkg| {
      json!({
        "id": pkg.id,
        "source": pkg.source,
        "registry": pkg.registry,
        "source_package": pkg.source_package,
        "direct": manifest.dependencies.contains_key(&pkg.id),
        "header_only": pkg.header_only,
        "requested_rev": pkg.requested_rev,
        "requested_requirement": pkg.requested_requirement,
        "resolved_version": pkg.resolved_version,
        "resolved_commit": pkg.resolved_commit,
        "recipe": pkg.recipe,
        "metadata_source": pkg.metadata_source,
        "package_manifest_digest": pkg.package_manifest_digest,
        "declared_deps_source": pkg.declared_deps_source,
        "deps": pkg.deps,
      })
    })
    .collect::<Vec<_>>();
  packages.sort_by(|a, b| {
    a.get("id").and_then(|v| v.as_str()).cmp(&b.get("id").and_then(|v| v.as_str()))
  });

  let human = render_locked_tree_human(&by_id, &roots);
  Ok(CommandOutput::new(
    "tree",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "roots": roots,
      "packages": packages,
    }),
  ))
}

fn render_tree_human(
  resolved: &resolver::ResolvedGraph,
  roots: &[String],
  provenance_overlay: Option<&BTreeMap<String, ProvenanceOverlay>>,
) -> String {
  if roots.is_empty() {
    return HumanMessageBuilder::new("No dependencies")
      .hint("Add one with `joy add <owner/repo>`")
      .build();
  }

  let mut lines = Vec::new();
  let mut stack_guard = BTreeSet::new();
  for root in roots {
    render_tree_node(resolved, root, 0, provenance_overlay, &mut stack_guard, &mut lines);
  }
  lines.join("\n")
}

fn render_locked_tree_human(
  by_id: &BTreeMap<String, &lockfile::LockedPackage>,
  roots: &[String],
) -> String {
  if roots.is_empty() {
    return HumanMessageBuilder::new("No dependencies")
      .hint("Add one with `joy add <owner/repo>`")
      .build();
  }

  let mut lines = Vec::new();
  let mut stack_guard = BTreeSet::new();
  for root in roots {
    render_locked_tree_node(by_id, root, 0, &mut stack_guard, &mut lines);
  }
  lines.join("\n")
}

fn render_tree_node(
  resolved: &resolver::ResolvedGraph,
  id: &str,
  depth: usize,
  provenance_overlay: Option<&BTreeMap<String, ProvenanceOverlay>>,
  stack_guard: &mut BTreeSet<String>,
  lines: &mut Vec<String>,
) {
  let Some(pkg) = resolved.package(id) else {
    return;
  };
  let indent = "  ".repeat(depth);
  let kind = if pkg.header_only { "header-only" } else { "compiled" };
  let source_suffix = match (&pkg.source, pkg.registry.as_deref()) {
    (crate::manifest::DependencySource::Registry, Some(registry)) => {
      format!(", registry {registry}")
    },
    (crate::manifest::DependencySource::Registry, None) => ", registry".to_string(),
    _ => String::new(),
  };
  let metadata_source = pkg.recipe_slug.as_deref().map(|_| "recipe").or_else(|| {
    provenance_overlay.and_then(|m| m.get(id)).and_then(|p| p.metadata_source.as_deref())
  });
  let metadata_suffix =
    metadata_source.map(|source| format!(", metadata {source}")).unwrap_or_default();
  if let Some(req) = pkg.requested_requirement.as_deref() {
    if let Some(version) = pkg.resolved_version.as_deref() {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}{metadata_suffix}, req {req}, version {version}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    } else {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}{metadata_suffix}, req {req}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    }
  } else {
    lines.push(format!(
      "{indent}- {} ({kind}{source_suffix}{metadata_suffix}, rev {}, commit {})",
      pkg.id, pkg.requested_rev, pkg.resolved_commit
    ));
  }

  if !stack_guard.insert(id.to_string()) {
    lines.push(format!("{indent}  - <cycle prevented>"));
    return;
  }

  if let Some(deps) = resolved.dependency_ids(id) {
    for dep in deps {
      render_tree_node(resolved, &dep, depth + 1, provenance_overlay, stack_guard, lines);
    }
  }
  stack_guard.remove(id);
}

fn render_locked_tree_node(
  by_id: &BTreeMap<String, &lockfile::LockedPackage>,
  id: &str,
  depth: usize,
  stack_guard: &mut BTreeSet<String>,
  lines: &mut Vec<String>,
) {
  let Some(pkg) = by_id.get(id).copied() else {
    return;
  };
  let indent = "  ".repeat(depth);
  let kind = if pkg.header_only { "header-only" } else { "compiled" };
  let source_suffix = match (pkg.source.as_str(), pkg.registry.as_deref()) {
    ("registry", Some(registry)) => format!(", registry {registry}"),
    ("registry", None) => ", registry".to_string(),
    _ => String::new(),
  };
  let metadata_suffix =
    pkg.metadata_source.as_deref().map(|source| format!(", metadata {source}")).unwrap_or_default();
  if let Some(req) = pkg.requested_requirement.as_deref() {
    if let Some(version) = pkg.resolved_version.as_deref() {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}{metadata_suffix}, req {req}, version {version}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    } else {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}{metadata_suffix}, req {req}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    }
  } else {
    lines.push(format!(
      "{indent}- {} ({kind}{source_suffix}{metadata_suffix}, rev {}, commit {})",
      pkg.id, pkg.requested_rev, pkg.resolved_commit
    ));
  }

  if !stack_guard.insert(id.to_string()) {
    lines.push(format!("{indent}  - <cycle prevented>"));
    return;
  }
  let mut deps = pkg.deps.clone();
  deps.sort();
  deps.dedup();
  for dep in deps {
    render_locked_tree_node(by_id, &dep, depth + 1, stack_guard, lines);
  }
  stack_guard.remove(id);
}
