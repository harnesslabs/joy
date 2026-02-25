use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::env;

use crate::cli::{RuntimeFlags, WhyArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::lockfile;
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

  let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
  roots.sort();
  if args.locked {
    return handle_locked(&cwd, &manifest_path, &manifest, &args.package, &roots);
  }

  let recipes = RecipeStore::load_default()
    .map_err(|err| JoyError::new("why", "recipe_load_failed", err.to_string(), 1))?;
  let resolved = resolver::resolve_manifest(&manifest, &recipes).map_err(map_resolver_error)?;
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
  let human = render_why_human(&args.package, &paths);
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
        "source": match pkg.source {
          crate::manifest::DependencySource::Github => "github",
          crate::manifest::DependencySource::Registry => "registry",
        },
        "registry": pkg.registry,
        "requested_rev": pkg.requested_rev,
        "requested_requirement": pkg.requested_requirement,
        "resolved_version": pkg.resolved_version,
        "resolved_commit": pkg.resolved_commit,
        "header_only": pkg.header_only,
        "recipe": pkg.recipe_slug,
      }
    }),
  ))
}

fn handle_locked(
  cwd: &std::path::Path,
  manifest_path: &std::path::Path,
  _manifest: &Manifest,
  target: &str,
  roots: &[String],
) -> Result<CommandOutput, JoyError> {
  let lockfile_path = cwd.join("joy.lock");
  if !lockfile_path.is_file() {
    return Err(JoyError::new(
      "why",
      "lockfile_missing",
      format!(
        "`--locked` requires `{}` to exist; create or refresh it with `joy sync --update-lock`",
        lockfile_path.display()
      ),
      1,
    ));
  }
  let lock = lockfile::Lockfile::load(&lockfile_path)
    .map_err(|err| JoyError::new("why", "lockfile_parse_error", err.to_string(), 1))?;
  let manifest_hash = lockfile::compute_manifest_hash(manifest_path)
    .map_err(|err| JoyError::new("why", "lockfile_hash_failed", err.to_string(), 1))?;
  if lock.manifest_hash != manifest_hash {
    return Err(JoyError::new(
      "why",
      "lockfile_stale",
      "joy.lock manifest hash does not match joy.toml; rerun `joy sync --update-lock`".to_string(),
      1,
    ));
  }

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
  let human = render_why_human(target, &paths);
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
      }
    }),
  ))
}

fn render_why_human(target: &str, paths: &[Vec<String>]) -> String {
  if paths.is_empty() {
    return crate::output::HumanMessageBuilder::new(format!(
      "No dependency path found for `{target}`"
    ))
    .hint("Try `joy tree` to inspect the current graph")
    .build();
  }

  let mut builder = crate::output::HumanMessageBuilder::new(format!("Why `{target}` is present"));
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
  JoyError::new("why", code, err.to_string(), 1)
}
