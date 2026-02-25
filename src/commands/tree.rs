use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::env;

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
  let recipes = RecipeStore::load_default()
    .map_err(|err| JoyError::new("tree", "recipe_load_failed", err.to_string(), 1))?;
  let resolved = resolver::resolve_manifest(&manifest, &recipes).map_err(map_resolver_error)?;

  let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
  roots.sort();

  let mut packages = resolved
    .packages()
    .map(|pkg| {
      let deps = resolved.dependency_ids(pkg.id.as_str()).unwrap_or_default();
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
        "deps": deps,
      })
    })
    .collect::<Vec<_>>();
  packages.sort_by(|a, b| {
    a.get("id").and_then(|v| v.as_str()).cmp(&b.get("id").and_then(|v| v.as_str()))
  });

  let human = render_tree_human(&resolved, &roots);

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
  if !lockfile_path.is_file() {
    return Err(JoyError::new(
      "tree",
      "lockfile_missing",
      format!(
        "`--locked` requires `{}` to exist; create or refresh it with `joy sync --update-lock`",
        lockfile_path.display()
      ),
      1,
    ));
  }
  let lock = lockfile::Lockfile::load(&lockfile_path)
    .map_err(|err| JoyError::new("tree", "lockfile_parse_error", err.to_string(), 1))?;
  let manifest_hash = lockfile::compute_manifest_hash(manifest_path)
    .map_err(|err| JoyError::new("tree", "lockfile_hash_failed", err.to_string(), 1))?;
  if lock.manifest_hash != manifest_hash {
    return Err(JoyError::new(
      "tree",
      "lockfile_stale",
      "joy.lock manifest hash does not match joy.toml; rerun `joy sync --update-lock`".to_string(),
      1,
    ));
  }
  if !manifest.dependencies.is_empty() && lock.packages.is_empty() {
    return Err(JoyError::new(
      "tree",
      "lockfile_incomplete",
      "joy.lock package metadata is missing for current dependencies; rerun `joy sync --update-lock`"
        .to_string(),
      1,
    ));
  }

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

fn render_tree_human(resolved: &resolver::ResolvedGraph, roots: &[String]) -> String {
  if roots.is_empty() {
    return HumanMessageBuilder::new("No dependencies")
      .hint("Add one with `joy add <owner/repo>`")
      .build();
  }

  let mut lines = Vec::new();
  let mut stack_guard = BTreeSet::new();
  for root in roots {
    render_tree_node(resolved, root, 0, &mut stack_guard, &mut lines);
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
  if let Some(req) = pkg.requested_requirement.as_deref() {
    if let Some(version) = pkg.resolved_version.as_deref() {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}, req {req}, version {version}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    } else {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}, req {req}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    }
  } else {
    lines.push(format!(
      "{indent}- {} ({kind}{source_suffix}, rev {}, commit {})",
      pkg.id, pkg.requested_rev, pkg.resolved_commit
    ));
  }

  if !stack_guard.insert(id.to_string()) {
    lines.push(format!("{indent}  - <cycle prevented>"));
    return;
  }

  if let Some(deps) = resolved.dependency_ids(id) {
    for dep in deps {
      render_tree_node(resolved, &dep, depth + 1, stack_guard, lines);
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
  if let Some(req) = pkg.requested_requirement.as_deref() {
    if let Some(version) = pkg.resolved_version.as_deref() {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}, req {req}, version {version}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    } else {
      lines.push(format!(
        "{indent}- {} ({kind}{source_suffix}, req {req}, tag {}, commit {})",
        pkg.id, pkg.requested_rev, pkg.resolved_commit
      ));
    }
  } else {
    lines.push(format!(
      "{indent}- {} ({kind}{source_suffix}, rev {}, commit {})",
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

fn map_resolver_error(err: resolver::ResolverError) -> JoyError {
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
    _ => "dependency_resolve_failed",
  };
  JoyError::new("tree", code, err.to_string(), 1)
}
