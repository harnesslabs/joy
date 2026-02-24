use serde_json::json;
use std::collections::BTreeSet;
use std::env;

use crate::cli::{RuntimeFlags, TreeArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::manifest::Manifest;
use crate::recipes::RecipeStore;
use crate::resolver;

pub fn handle(_args: TreeArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
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
        "direct": pkg.direct,
        "header_only": pkg.header_only,
        "requested_rev": pkg.requested_rev,
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

fn render_tree_human(resolved: &resolver::ResolvedGraph, roots: &[String]) -> String {
  if roots.is_empty() {
    return "No dependencies".to_string();
  }

  let mut lines = Vec::new();
  let mut stack_guard = BTreeSet::new();
  for root in roots {
    render_tree_node(resolved, root, 0, &mut stack_guard, &mut lines);
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
  lines.push(format!(
    "{indent}- {} ({kind}, rev {}, commit {})",
    pkg.id, pkg.requested_rev, pkg.resolved_commit
  ));

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

fn map_resolver_error(err: resolver::ResolverError) -> JoyError {
  let code = match &err {
    resolver::ResolverError::Fetch { source, .. } if source.is_offline_cache_miss() => {
      "offline_cache_miss"
    },
    resolver::ResolverError::Fetch { source, .. } if source.is_offline_network_disabled() => {
      "offline_network_disabled"
    },
    _ => "dependency_resolve_failed",
  };
  JoyError::new("tree", code, err.to_string(), 1)
}
