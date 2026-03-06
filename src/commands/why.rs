use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::env;

use super::graph_common::{
  load_fresh_lockfile_provenance_overlay, map_resolver_error, validate_locked_graph_lockfile,
};
use crate::cli::{RuntimeFlags, WhyArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::manifest::Manifest;
use crate::recipes::RecipeStore;
use crate::resolver;

pub fn handle(args: WhyArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("why", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "why",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }
  let manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("why", "manifest_parse_error", err.to_string(), 1))?;
  let provenance_overlay = load_fresh_lockfile_provenance_overlay(&cwd, &manifest_path);

  let mut roots = manifest
    .dependencies
    .iter()
    .map(|(key, spec)| spec.declared_package(key).to_string())
    .collect::<Vec<_>>();
  roots.sort();
  if args.locked {
    return handle_locked(&cwd, &manifest_path, &manifest, &args.package, &roots);
  }

  let recipes = RecipeStore::load_default()
    .map_err(|err| JoyError::new("why", "recipe_load_failed", err.to_string(), 1))?;
  let resolved = resolver::resolve_manifest(&manifest, &recipes)
    .map_err(|err| map_resolver_error("why", err))?;
  if resolved.package(&args.package).is_none() {
    return Err(JoyError::new(
      "why",
      "dependency_not_found",
      format!("dependency `{}` is not present in the resolved graph", args.package),
      1,
    ));
  }
  let paths =
    find_paths(&roots, &args.package, |id| resolved.dependency_ids(id).unwrap_or_default());

  let pkg = resolved.package(&args.package).expect("checked");
  let provenance = pkg.recipe_slug.as_ref().map(|_| "recipe".to_string()).or_else(|| {
    provenance_overlay
      .as_ref()
      .and_then(|m| m.get(args.package.as_str()))
      .and_then(|p| p.metadata_source.clone())
  });
  let package_manifest_digest = provenance_overlay
    .as_ref()
    .and_then(|m| m.get(args.package.as_str()))
    .and_then(|p| p.package_manifest_digest.clone());
  let declared_deps_source = pkg.recipe_slug.as_ref().map(|_| "recipe".to_string()).or_else(|| {
    provenance_overlay
      .as_ref()
      .and_then(|m| m.get(args.package.as_str()))
      .and_then(|p| p.declared_deps_source.clone())
  });
  let human = render_why_human(&args.package, &paths, provenance.as_deref());
  Ok(CommandOutput::new(
    "why",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "package": args.package,
      "locked": false,
      "roots": roots,
      "paths": paths,
      "package_info": {
        "id": pkg.id.to_string(),
        "source": pkg.source.as_str(),
        "registry": pkg.registry,
        "requested_rev": pkg.requested_rev,
        "requested_requirement": pkg.requested_requirement,
        "resolved_version": pkg.resolved_version,
        "resolved_commit": pkg.resolved_commit,
        "header_only": pkg.header_only,
        "recipe": pkg.recipe_slug,
        "metadata_source": provenance,
        "package_manifest_digest": package_manifest_digest,
        "declared_deps_source": declared_deps_source,
      }
    }),
  ))
}

fn handle_locked(
  cwd: &std::path::Path,
  manifest_path: &std::path::Path,
  manifest: &Manifest,
  target: &str,
  roots: &[String],
) -> Result<CommandOutput, JoyError> {
  let lockfile_path = cwd.join("joy.lock");
  let lock = validate_locked_graph_lockfile("why", manifest, manifest_path, &lockfile_path)?;

  let by_id = lock.packages.iter().map(|pkg| (pkg.id.clone(), pkg)).collect::<BTreeMap<_, _>>();
  let Some(pkg) = by_id.get(target).copied() else {
    return Err(JoyError::new(
      "why",
      "dependency_not_found",
      format!("dependency `{target}` is not present in `joy.lock` package graph"),
      1,
    ));
  };

  let paths =
    find_paths(roots, target, |id| by_id.get(id).map(|p| p.deps.clone()).unwrap_or_default());
  let human = render_why_human(target, &paths, pkg.metadata_source.as_deref());
  Ok(CommandOutput::new(
    "why",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "package": target,
      "locked": true,
      "roots": roots,
      "paths": paths,
      "package_info": {
        "id": pkg.id,
        "source": pkg.source,
        "registry": pkg.registry,
        "requested_rev": pkg.requested_rev,
        "requested_requirement": pkg.requested_requirement,
        "resolved_version": pkg.resolved_version,
        "resolved_commit": pkg.resolved_commit,
        "header_only": pkg.header_only,
        "recipe": pkg.recipe,
        "metadata_source": pkg.metadata_source,
        "package_manifest_digest": pkg.package_manifest_digest,
        "declared_deps_source": pkg.declared_deps_source,
      }
    }),
  ))
}

fn render_why_human(target: &str, paths: &[Vec<String>], metadata_source: Option<&str>) -> String {
  if paths.is_empty() {
    return crate::output::HumanMessageBuilder::new(format!(
      "No dependency path found for `{target}`"
    ))
    .hint("Try `joy tree` to inspect the current graph")
    .build();
  }

  let mut builder = crate::output::HumanMessageBuilder::new(format!("Why `{target}` is present"));
  if let Some(source) = metadata_source {
    builder = builder.kv("metadata", source.to_string());
  }
  for path in paths {
    builder = builder.line(format!("- {}", path.join(" -> ")));
  }
  builder.build()
}

fn find_paths<F>(roots: &[String], target: &str, deps_of: F) -> Vec<Vec<String>>
where
  F: Fn(&str) -> Vec<String>,
{
  let mut out = Vec::new();
  for root in roots {
    let mut stack = vec![root.clone()];
    let mut visiting = BTreeSet::new();
    dfs_collect_paths(root, target, &deps_of, &mut visiting, &mut stack, &mut out);
  }
  out.sort();
  out.dedup();
  out
}

fn dfs_collect_paths<F>(
  current: &str,
  target: &str,
  deps_of: &F,
  visiting: &mut BTreeSet<String>,
  stack: &mut Vec<String>,
  out: &mut Vec<Vec<String>>,
) where
  F: Fn(&str) -> Vec<String>,
{
  if current == target {
    out.push(stack.clone());
    return;
  }
  if !visiting.insert(current.to_string()) {
    return;
  }
  let mut deps = deps_of(current);
  deps.sort();
  deps.dedup();
  for dep in deps {
    stack.push(dep.clone());
    dfs_collect_paths(&dep, target, deps_of, visiting, stack, out);
    stack.pop();
  }
  visiting.remove(current);
}
