//! `joy build` implementation and shared project build pipeline used by `joy run`.

use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::{BuildArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::install_index::InstallIndex;
use crate::lockfile;
use crate::manifest::{Manifest, SelectedTarget};
use crate::ninja::{BuildProfile, NinjaBuildSpec};
use crate::output::{HumanMessageBuilder, progress_detail, progress_stage};
use crate::recipes::{Linkage as RecipeLinkage, RecipeStore};
use crate::{abi, cmake, fetch, global_cache, linking, ninja, project_env, resolver, toolchain};

#[derive(Debug, Clone)]
pub(crate) struct BuildExecution {
  pub project_root: PathBuf,
  pub manifest_path: PathBuf,
  pub build_file: PathBuf,
  pub binary_path: PathBuf,
  pub source_file: PathBuf,
  pub compiled_sources: Vec<PathBuf>,
  pub target_name: String,
  pub target_default: bool,
  pub include_dirs: Vec<PathBuf>,
  pub link_dirs: Vec<PathBuf>,
  pub link_libs: Vec<String>,
  pub toolchain: toolchain::Toolchain,
  pub profile: BuildProfile,
  pub ninja_status: i32,
  pub ninja_stdout: String,
  pub ninja_stderr: String,
  pub compiled_dependencies_built: Vec<String>,
  pub lockfile_path: PathBuf,
  pub lockfile_updated: bool,
}

impl BuildExecution {
  fn profile_name(&self) -> &'static str {
    match self.profile {
      BuildProfile::Debug => "debug",
      BuildProfile::Release => "release",
    }
  }

  fn json_data(&self) -> serde_json::Value {
    json!({
      "project_root": self.project_root.display().to_string(),
      "manifest_path": self.manifest_path.display().to_string(),
      "build_file": self.build_file.display().to_string(),
      "binary_path": self.binary_path.display().to_string(),
      "source_file": self.source_file.display().to_string(),
      "compiled_sources": self.compiled_sources.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
      "target": self.target_name,
      "target_default": self.target_default,
      "profile": self.profile_name(),
      "include_dirs": self.include_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
      "link_dirs": self.link_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
      "link_libs": self.link_libs.clone(),
      "compiled_dependencies_built": self.compiled_dependencies_built.clone(),
      "toolchain": {
        "compiler_kind": self.toolchain.compiler.kind.as_str(),
        "compiler_version": self.toolchain.compiler.version,
        "compiler_path": self.toolchain.compiler.path.display().to_string(),
        "ninja_path": self.toolchain.ninja.path.display().to_string(),
      },
      "ninja_status": self.ninja_status,
      "ninja_stdout": self.ninja_stdout,
      "ninja_stderr": self.ninja_stderr,
      "lockfile_path": self.lockfile_path.display().to_string(),
      "lockfile_updated": self.lockfile_updated,
    })
  }
}

#[derive(Debug, Clone)]
pub(crate) struct SyncExecution {
  pub project_root: PathBuf,
  pub manifest_path: PathBuf,
  pub include_dirs: Vec<PathBuf>,
  pub link_dirs: Vec<PathBuf>,
  pub link_libs: Vec<String>,
  pub toolchain: Option<toolchain::Toolchain>,
  pub profile: BuildProfile,
  pub compiled_dependencies_built: Vec<String>,
  pub lockfile_path: PathBuf,
  pub lockfile_updated: bool,
}

impl SyncExecution {
  fn profile_name(&self) -> &'static str {
    match self.profile {
      BuildProfile::Debug => "debug",
      BuildProfile::Release => "release",
    }
  }

  pub(crate) fn json_data(&self) -> serde_json::Value {
    let toolchain = self.toolchain.as_ref().map(|toolchain| {
      json!({
        "compiler_kind": toolchain.compiler.kind.as_str(),
        "compiler_version": toolchain.compiler.version,
        "compiler_path": toolchain.compiler.path.display().to_string(),
        "ninja_path": toolchain.ninja.path.display().to_string(),
      })
    });
    json!({
      "project_root": self.project_root.display().to_string(),
      "manifest_path": self.manifest_path.display().to_string(),
      "profile": self.profile_name(),
      "include_dirs": self.include_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
      "link_dirs": self.link_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
      "link_libs": self.link_libs.clone(),
      "compiled_dependencies_built": self.compiled_dependencies_built.clone(),
      "toolchain": toolchain,
      "lockfile_path": self.lockfile_path.display().to_string(),
      "lockfile_updated": self.lockfile_updated,
    })
  }
}

#[derive(Debug, Clone)]
pub(crate) struct BuildOptions {
  pub release: bool,
  pub target: Option<String>,
  pub locked: bool,
  pub update_lock: bool,
  pub offline: bool,
  pub progress: bool,
}

/// Handle `joy build` by executing the local build pipeline and returning a CLI output payload.
pub fn handle(args: BuildArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  if runtime.progress {
    progress_stage("Starting build");
  }
  let execution = build_project(BuildOptions {
    release: args.release,
    target: args.target,
    locked: args.locked || runtime.frozen,
    update_lock: args.update_lock,
    offline: runtime.offline,
    progress: runtime.progress,
  })?;

  Ok(CommandOutput::new(
    "build",
    HumanMessageBuilder::new("Build finished")
      .kv("binary", execution.binary_path.display().to_string())
      .kv("target", execution.target_name.clone())
      .kv(
        "toolchain",
        format!(
          "{} {}",
          execution.toolchain.compiler.kind.as_str(),
          execution.toolchain.compiler.version
        ),
      )
      .kv("profile", execution.profile_name())
      .kv("compiled dependencies built", execution.compiled_dependencies_built.len().to_string())
      .kv("lockfile updated", execution.lockfile_updated.to_string())
      .build(),
    execution.json_data(),
  ))
}

/// Build the current project and return the execution metadata reused by `joy run`.
pub(crate) fn build_project(options: BuildOptions) -> Result<BuildExecution, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: options.offline,
    progress: options.progress,
  });
  let project_root = env::current_dir().map_err(|err| {
    JoyError::new("build", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = project_root.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "build",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }

  let manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("build", "manifest_parse_error", err.to_string(), 1))?;
  let selected_target = manifest
    .select_target(options.target.as_deref())
    .map_err(|err| JoyError::new("build", "invalid_target", err.to_string(), 1))?;
  let manifest_hash = lockfile::compute_manifest_hash(&manifest_path)
    .map_err(|err| JoyError::new("build", "lockfile_hash_failed", err.to_string(), 1))?;
  let lockfile_path = project_root.join("joy.lock");
  let env_layout = project_env::ensure_layout(&project_root)
    .map_err(|err| JoyError::new("build", "env_setup_failed", err.to_string(), 1))?;
  let lock_plan = evaluate_lockfile_plan("build", &lockfile_path, &manifest_hash, &options)?;

  let toolchain = toolchain::discover().map_err(|err| map_toolchain_error("build", err))?;
  let profile = BuildProfile::from_release_flag(options.release);
  let source_file = project_root.join(&selected_target.entry);
  let source_files = collect_project_source_files(&project_root, &selected_target)?;

  let obj_dir = env_layout.build_dir.join("obj");
  fs::create_dir_all(&obj_dir)
    .map_err(|err| JoyError::io("build", "creating object directory", &obj_dir, &err))?;
  let compile_units = source_files
    .iter()
    .map(|src| ninja::NinjaCompileUnit {
      source_file: relative_or_absolute(&project_root, src),
      object_file: relative_or_absolute(
        &project_root,
        &obj_dir.join(object_file_name_for_source(&project_root, &selected_target.name, src)),
      ),
    })
    .collect::<Vec<_>>();
  let binary_path = env_layout.bin_dir.join(binary_name(&selected_target.name));
  let build_file = if selected_target.is_default {
    env_layout.build_dir.join("build.ninja")
  } else {
    env_layout
      .build_dir
      .join(format!("build-{}.ninja", sanitize_target_name(&selected_target.name)))
  };
  let mut include_dirs = collect_include_dirs(&env_layout.include_dir).map_err(|err| {
    JoyError::io("build", "reading include directories", &env_layout.include_dir, &err)
  })?;
  let user_include_dirs = collect_user_include_dirs(&project_root, &manifest)?;
  let target_include_dirs = collect_target_include_dirs(&project_root, &selected_target)?;
  include_dirs.extend(user_include_dirs);
  include_dirs.extend(target_include_dirs);
  include_dirs.sort();
  include_dirs.dedup();
  let native_link =
    prepare_compiled_dependencies(&manifest, &env_layout.lib_dir, &toolchain, profile)?;
  validate_locked_package_metadata_if_needed("build", &lock_plan, &native_link.lockfile_packages)?;
  refresh_install_index(&env_layout, &manifest, &native_link)?;

  if options.progress {
    progress_detail("Generating build graph");
  }

  let spec = NinjaBuildSpec {
    compiler_kind: toolchain.compiler.kind,
    compiler_executable: toolchain.compiler.executable_name.clone(),
    cpp_standard: manifest.project.cpp_standard.clone(),
    compile_units,
    binary_file: relative_or_absolute(&project_root, &binary_path),
    include_dirs: include_dirs.iter().map(|dir| relative_or_absolute(&project_root, dir)).collect(),
    link_dirs: native_link
      .link_dirs
      .iter()
      .map(|dir| relative_or_absolute(&project_root, dir))
      .collect(),
    link_libs: native_link.link_libs.clone(),
    profile,
  };
  ninja::write_build_ninja(&build_file, &spec)
    .map_err(|err| JoyError::new("build", "ninja_file_write_failed", err.to_string(), 1))?;

  if options.progress {
    progress_detail("Compiling and linking");
  }
  let ninja_output = run_ninja_build(&toolchain, &project_root, &build_file)?;

  let lockfile_updated = write_lockfile_if_needed(
    "build",
    lock_plan,
    &lockfile_path,
    &manifest_hash,
    &native_link.lockfile_packages,
  )?;

  Ok(BuildExecution {
    project_root,
    manifest_path,
    build_file,
    binary_path,
    source_file,
    compiled_sources: source_files,
    target_name: selected_target.name,
    target_default: selected_target.is_default,
    include_dirs,
    link_dirs: native_link.link_dirs,
    link_libs: native_link.link_libs,
    toolchain,
    profile,
    ninja_status: ninja_output.status_code,
    ninja_stdout: ninja_output.stdout,
    ninja_stderr: ninja_output.stderr,
    compiled_dependencies_built: native_link.built_packages,
    lockfile_path,
    lockfile_updated,
  })
}

/// Materialize dependencies and lockfile state without compiling the final application binary.
pub(crate) fn sync_project(options: BuildOptions) -> Result<SyncExecution, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: options.offline,
    progress: options.progress,
  });
  let project_root = env::current_dir().map_err(|err| {
    JoyError::new("sync", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = project_root.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "sync",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }

  let manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("sync", "manifest_parse_error", err.to_string(), 1))?;
  let manifest_hash = lockfile::compute_manifest_hash(&manifest_path)
    .map_err(|err| JoyError::new("sync", "lockfile_hash_failed", err.to_string(), 1))?;
  let lockfile_path = project_root.join("joy.lock");
  let env_layout = project_env::ensure_layout(&project_root)
    .map_err(|err| JoyError::new("sync", "env_setup_failed", err.to_string(), 1))?;
  let lock_plan = evaluate_lockfile_plan("sync", &lockfile_path, &manifest_hash, &options)
    .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?;
  let profile = BuildProfile::from_release_flag(options.release);

  let mut toolchain = None;
  let native_link = if manifest.dependencies.is_empty() {
    NativeLinkInputs::default()
  } else {
    let resolved_stage = resolve_dependency_stage(&manifest)
      .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?;
    let PrefetchedDependencyStage { all_by_key, build_by_key } =
      prefetch_dependency_stage(&resolved_stage)
        .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?;

    let has_compiled = resolved_stage
      .build_order_ids
      .iter()
      .any(|id| resolved_stage.resolved.package(id).is_some_and(|pkg| !pkg.header_only));

    let compiled_stage = if has_compiled {
      let discovered = toolchain::discover().map_err(|err| map_toolchain_error("sync", err))?;
      toolchain = Some(discovered.clone());
      build_compiled_dependency_stage(
        &manifest,
        &env_layout.lib_dir,
        &discovered,
        profile,
        &resolved_stage,
        build_by_key,
      )
      .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?
    } else {
      CompiledDependencyBuildStage::default()
    };

    let lockfile_packages = assemble_lockfile_packages(
      &resolved_stage.resolved,
      &resolved_stage.recipes,
      &all_by_key,
      &compiled_stage.compiled_lock_metadata,
    )
    .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?;

    NativeLinkInputs {
      link_dirs: compiled_stage.link_dirs,
      link_libs: compiled_stage.link_libs,
      built_packages: compiled_stage.built_packages,
      installed_lib_files: compiled_stage.installed_lib_files,
      lockfile_packages,
    }
  };

  validate_locked_package_metadata_if_needed("sync", &lock_plan, &native_link.lockfile_packages)
    .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?;
  refresh_install_index(&env_layout, &manifest, &native_link)
    .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?;

  let include_dirs = collect_include_dirs(&env_layout.include_dir).map_err(|err| {
    JoyError::io("sync", "reading include directories", &env_layout.include_dir, &err)
  })?;
  let lockfile_updated = write_lockfile_if_needed(
    "sync",
    lock_plan,
    &lockfile_path,
    &manifest_hash,
    &native_link.lockfile_packages,
  )
  .map_err(|err| JoyError::new("sync", err.code, err.message, err.exit_code))?;

  Ok(SyncExecution {
    project_root,
    manifest_path,
    include_dirs,
    link_dirs: native_link.link_dirs,
    link_libs: native_link.link_libs,
    toolchain,
    profile,
    compiled_dependencies_built: native_link.built_packages,
    lockfile_path,
    lockfile_updated,
  })
}

#[derive(Debug, Clone, Default)]
struct NativeLinkInputs {
  link_dirs: Vec<PathBuf>,
  link_libs: Vec<String>,
  built_packages: Vec<String>,
  installed_lib_files: Vec<PathBuf>,
  lockfile_packages: Vec<lockfile::LockedPackage>,
}

#[derive(Debug, Clone)]
struct NinjaRunOutput {
  status_code: i32,
  stdout: String,
  stderr: String,
}

#[derive(Debug, Clone)]
struct CompiledLockMetadata {
  abi_hash: String,
  libs: Vec<String>,
  linkage: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedDependencyStage {
  recipes: RecipeStore,
  resolved: resolver::ResolvedGraph,
  build_order_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct PrefetchedDependencyStage {
  all_by_key: BTreeMap<(String, String), fetch::FetchResult>,
  build_by_key: BTreeMap<(String, String), fetch::FetchResult>,
}

#[derive(Debug, Clone, Default)]
struct CompiledDependencyBuildStage {
  link_dirs: Vec<PathBuf>,
  link_libs: Vec<String>,
  built_packages: Vec<String>,
  installed_lib_files: Vec<PathBuf>,
  compiled_lock_metadata: BTreeMap<String, CompiledLockMetadata>,
}

fn prepare_compiled_dependencies(
  manifest: &Manifest,
  project_lib_dir: &Path,
  toolchain: &toolchain::Toolchain,
  profile: BuildProfile,
) -> Result<NativeLinkInputs, JoyError> {
  if manifest.dependencies.is_empty() {
    return Ok(NativeLinkInputs::default());
  }

  let resolved_stage = resolve_dependency_stage(manifest)?;
  let PrefetchedDependencyStage { all_by_key, build_by_key } =
    prefetch_dependency_stage(&resolved_stage)?;
  let compiled_stage = build_compiled_dependency_stage(
    manifest,
    project_lib_dir,
    toolchain,
    profile,
    &resolved_stage,
    build_by_key,
  )?;
  let lockfile_packages = assemble_lockfile_packages(
    &resolved_stage.resolved,
    &resolved_stage.recipes,
    &all_by_key,
    &compiled_stage.compiled_lock_metadata,
  )?;

  Ok(NativeLinkInputs {
    link_dirs: compiled_stage.link_dirs,
    link_libs: compiled_stage.link_libs,
    built_packages: compiled_stage.built_packages,
    installed_lib_files: compiled_stage.installed_lib_files,
    lockfile_packages,
  })
}

fn resolve_dependency_stage(manifest: &Manifest) -> Result<ResolvedDependencyStage, JoyError> {
  let recipes = RecipeStore::load_default()
    .map_err(|err| JoyError::new("build", "recipe_load_failed", err.to_string(), 1))?;
  let resolved =
    resolver::resolve_manifest(manifest, &recipes).map_err(map_dependency_resolve_error)?;
  let build_order_ids = resolved
    .build_order_ids()
    .map_err(|err| JoyError::new("build", "dependency_graph_invalid", err.to_string(), 1))?;
  Ok(ResolvedDependencyStage { recipes, resolved, build_order_ids })
}

fn prefetch_dependency_stage(
  resolved_stage: &ResolvedDependencyStage,
) -> Result<PrefetchedDependencyStage, JoyError> {
  let requests = resolved_stage
    .build_order_ids
    .iter()
    .map(|id| {
      let pkg = resolved_stage
        .resolved
        .package(id)
        .expect("build_order_ids must correspond to resolved packages");
      (pkg.id.clone(), pkg.requested_rev.clone())
    })
    .collect::<Vec<_>>();
  let prefetched =
    fetch::prefetch_github_packages(requests).map_err(|err| map_fetch_error("build", err))?;
  let all_by_key = prefetched
    .into_iter()
    .map(|f| ((f.package.to_string(), f.requested_rev.clone()), f))
    .collect::<BTreeMap<_, _>>();
  let build_by_key = all_by_key.clone();
  Ok(PrefetchedDependencyStage { all_by_key, build_by_key })
}

fn build_compiled_dependency_stage(
  manifest: &Manifest,
  project_lib_dir: &Path,
  toolchain: &toolchain::Toolchain,
  profile: BuildProfile,
  resolved_stage: &ResolvedDependencyStage,
  mut prefetched_for_build: BTreeMap<(String, String), fetch::FetchResult>,
) -> Result<CompiledDependencyBuildStage, JoyError> {
  let cache = global_cache::GlobalCache::resolve()
    .map_err(|err| JoyError::new("build", "cache_setup_failed", err.to_string(), 1))?;
  cache
    .ensure_layout()
    .map_err(|err| JoyError::new("build", "cache_setup_failed", err.to_string(), 1))?;

  let mut stage = CompiledDependencyBuildStage::default();

  for id in &resolved_stage.build_order_ids {
    let pkg = resolved_stage
      .resolved
      .package(id)
      .expect("build_order_ids must correspond to resolved packages");
    if pkg.header_only {
      continue;
    }

    let Some(recipe) = resolved_stage.recipes.get(&pkg.id) else {
      return Err(JoyError::new(
        "build",
        "missing_recipe",
        format!("compiled dependency `{}` requires a curated recipe", pkg.id),
        1,
      ));
    };
    let Some(cmake_recipe) = recipe.cmake.as_ref() else {
      return Err(JoyError::new(
        "build",
        "missing_cmake_metadata",
        format!("recipe `{}` is missing `[cmake]` metadata", recipe.id),
        1,
      ));
    };
    let Some(link_recipe) = recipe.link.as_ref() else {
      return Err(JoyError::new(
        "build",
        "missing_link_metadata",
        format!("recipe `{}` is missing `[link]` metadata", recipe.id),
        1,
      ));
    };
    if link_recipe.libs.is_empty() {
      continue;
    }

    let fetched = prefetched_for_build
      .remove(&(pkg.id.to_string(), pkg.requested_rev.clone()))
      .ok_or_else(|| {
        JoyError::new(
          "build",
          "fetch_failed",
          format!("missing prefetched source checkout for `{}` at `{}`", pkg.id, pkg.requested_rev),
          1,
        )
      })?;

    let recipe_file =
      resolved_stage.recipes.root_dir().join("packages").join(format!("{}.toml", recipe.slug));
    let recipe_contents = fs::read_to_string(&recipe_file)
      .map_err(|err| JoyError::io("build", "reading recipe file", &recipe_file, &err))?;
    let preferred_linkage = link_recipe.preferred_linkage.unwrap_or(RecipeLinkage::Static);
    let abi_hash = abi::compute_abi_hash(&abi::AbiHashInput {
      package_id: pkg.id.to_string(),
      resolved_commit: pkg.resolved_commit.clone(),
      recipe_content_hash: abi::hash_recipe_contents(&recipe_contents),
      compiler_kind: toolchain.compiler.kind.as_str().to_string(),
      compiler_version: toolchain.compiler.version.clone(),
      target_triple: target_triple_guess(toolchain.compiler.kind),
      host_os: std::env::consts::OS.to_string(),
      host_arch: std::env::consts::ARCH.to_string(),
      profile: match profile {
        BuildProfile::Debug => abi::AbiBuildProfile::Debug,
        BuildProfile::Release => abi::AbiBuildProfile::Release,
      },
      cpp_standard: manifest.project.cpp_standard.clone(),
      linkage: match preferred_linkage {
        RecipeLinkage::Static => abi::AbiLinkage::Static,
        RecipeLinkage::Shared => abi::AbiLinkage::Shared,
      },
      cxxflags: Vec::new(),
      ldflags: Vec::new(),
      recipe_configure_args: cmake_recipe.configure_args.clone(),
      env: Default::default(),
    });
    stage.compiled_lock_metadata.insert(
      pkg.id.to_string(),
      CompiledLockMetadata {
        abi_hash: abi_hash.clone(),
        libs: link_recipe.libs.clone(),
        linkage: Some(linkage_name(preferred_linkage)),
      },
    );

    let layout = cache
      .ensure_compiled_build_layout(&abi_hash)
      .map_err(|err| JoyError::new("build", "cache_setup_failed", err.to_string(), 1))?;
    let source_dir = if let Some(fetch) = recipe.fetch.as_ref() {
      if fetch.subdir.trim().is_empty() {
        fetched.source_dir.clone()
      } else {
        fetched.source_dir.join(&fetch.subdir)
      }
    } else {
      fetched.source_dir.clone()
    };

    let cmake_result = cmake::build_into_cache(&cmake::CmakeBuildRequest {
      source_dir,
      build_layout: layout.clone(),
      profile,
      compiler_kind: toolchain.compiler.kind,
      compiler_path: toolchain.compiler.path.clone(),
      configure_args: cmake_recipe.configure_args.clone(),
      build_targets: cmake_recipe.build_targets.clone(),
      header_roots: recipe.include_roots().to_vec(),
    })
    .map_err(|err| JoyError::new("build", "cmake_build_failed", err.to_string(), 1))?;
    let lib_install =
      linking::install_compiled_libraries(project_lib_dir, &layout.lib_dir, &link_recipe.libs)
        .map_err(|err| JoyError::new("build", "library_install_failed", err.to_string(), 1))?;

    if !cmake_result.cache_hit {
      stage.built_packages.push(pkg.id.to_string());
    }
    if !stage.link_dirs.iter().any(|p| p == &lib_install.project_lib_dir) {
      stage.link_dirs.push(lib_install.project_lib_dir.clone());
    }
    for lib in lib_install.link_libs {
      if !stage.link_libs.contains(&lib) {
        stage.link_libs.push(lib);
      }
    }
    for file in lib_install.installed_files {
      if !stage.installed_lib_files.contains(&file) {
        stage.installed_lib_files.push(file);
      }
    }
  }

  Ok(stage)
}

fn collect_include_dirs(project_include_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
  let deps_dir = project_include_dir.join("deps");
  if !deps_dir.is_dir() {
    return Ok(Vec::new());
  }

  let mut dirs = Vec::new();
  for entry in fs::read_dir(deps_dir)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      dirs.push(path);
    }
  }
  dirs.sort();
  Ok(dirs)
}

fn collect_user_include_dirs(
  project_root: &Path,
  manifest: &Manifest,
) -> Result<Vec<PathBuf>, JoyError> {
  let mut dirs = Vec::new();
  for raw in &manifest.project.include_dirs {
    let path = project_root.join(raw);
    if !path.is_dir() {
      return Err(JoyError::new(
        "build",
        "include_dir_not_found",
        format!("project include dir `{}` does not exist", path.display()),
        1,
      ));
    }
    dirs.push(path);
  }
  Ok(dirs)
}

fn collect_target_include_dirs(
  project_root: &Path,
  target: &SelectedTarget,
) -> Result<Vec<PathBuf>, JoyError> {
  let mut dirs = Vec::new();
  for raw in &target.include_dirs {
    let path = project_root.join(raw);
    if !path.is_dir() {
      return Err(JoyError::new(
        "build",
        "include_dir_not_found",
        format!("target include dir `{}` does not exist", path.display()),
        1,
      ));
    }
    dirs.push(path);
  }
  Ok(dirs)
}

fn collect_project_source_files(
  project_root: &Path,
  target: &SelectedTarget,
) -> Result<Vec<PathBuf>, JoyError> {
  let entry = project_root.join(&target.entry);
  if !entry.is_file() {
    return Err(JoyError::new(
      "build",
      "entry_not_found",
      format!("entry source file `{}` does not exist", entry.display()),
      1,
    ));
  }

  let mut seen = BTreeSet::new();
  let mut files = Vec::new();

  for path in
    std::iter::once(entry).chain(target.extra_sources.iter().map(|p| project_root.join(p)))
  {
    if !path.is_file() {
      return Err(JoyError::new(
        "build",
        "source_not_found",
        format!("source file `{}` does not exist", path.display()),
        1,
      ));
    }
    let key = normalize_path_for_hash(project_root, &path);
    if seen.insert(key) {
      files.push(path);
    }
  }

  Ok(files)
}

fn object_file_name_for_source(
  project_root: &Path,
  target_name: &str,
  source_file: &Path,
) -> String {
  let normalized = format!("{target_name}::{}", normalize_path_for_hash(project_root, source_file));
  let stem = source_file.file_stem().and_then(|s| s.to_str()).unwrap_or("obj");
  let sanitized_stem =
    stem.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect::<String>();
  let mut hasher = Sha256::new();
  hasher.update(normalized.as_bytes());
  let hash = format!("{:x}", hasher.finalize());
  let short_hash = &hash[..12];
  format!("{sanitized_stem}-{short_hash}.o")
}

fn sanitize_target_name(name: &str) -> String {
  name.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect()
}

fn normalize_path_for_hash(project_root: &Path, path: &Path) -> String {
  relative_or_absolute(project_root, path).to_string_lossy().replace('\\', "/")
}

fn run_ninja_build(
  toolchain: &toolchain::Toolchain,
  project_root: &Path,
  build_file: &Path,
) -> Result<NinjaRunOutput, JoyError> {
  let output = Command::new(&toolchain.ninja.path)
    .current_dir(project_root)
    .arg("-f")
    .arg(relative_or_absolute(project_root, build_file))
    .output()
    .map_err(|err| {
      JoyError::new("build", "ninja_spawn_failed", format!("failed to run ninja: {err}"), 1)
    })?;
  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  if !output.status.success() {
    return Err(JoyError::new(
      "build",
      "build_failed",
      format!(
        "ninja build failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout.trim_end(),
        stderr.trim_end()
      ),
      1,
    ));
  }

  Ok(NinjaRunOutput { status_code: output.status.code().unwrap_or_default(), stdout, stderr })
}

fn relative_or_absolute(root: &Path, path: &Path) -> PathBuf {
  path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn binary_name(project_name: &str) -> String {
  if cfg!(windows) { format!("{project_name}.exe") } else { project_name.to_string() }
}

fn assemble_lockfile_packages(
  resolved: &resolver::ResolvedGraph,
  recipes: &RecipeStore,
  prefetched_by_key: &BTreeMap<(String, String), fetch::FetchResult>,
  compiled_lock_metadata: &BTreeMap<String, CompiledLockMetadata>,
) -> Result<Vec<lockfile::LockedPackage>, JoyError> {
  let mut packages = Vec::new();

  for pkg in resolved.packages() {
    let key = (pkg.id.to_string(), pkg.requested_rev.clone());
    let fetched = prefetched_by_key.get(&key).ok_or_else(|| {
      JoyError::new(
        "build",
        "lockfile_package_assembly_failed",
        format!(
          "missing prefetched source checkout metadata for `{}` at `{}` while assembling lockfile",
          pkg.id, pkg.requested_rev
        ),
        1,
      )
    })?;
    let recipe = recipes.get(&pkg.id);

    let mut header_roots = if let Some(recipe) = recipe {
      recipe.include_roots().to_vec()
    } else {
      infer_header_roots_from_source_dir(&fetched.source_dir)
    };
    header_roots.sort();
    header_roots.dedup();

    let mut libs =
      recipe.and_then(|r| r.link.as_ref()).map(|link| link.libs.clone()).unwrap_or_default();
    let mut linkage = recipe
      .and_then(|r| r.link.as_ref())
      .and_then(|link| link.preferred_linkage)
      .map(linkage_name);
    let mut abi_hash = String::new();
    if let Some(compiled_meta) = compiled_lock_metadata.get(pkg.id.as_str()) {
      libs = compiled_meta.libs.clone();
      linkage = compiled_meta.linkage.clone();
      abi_hash = compiled_meta.abi_hash.clone();
    }

    packages.push(lockfile::LockedPackage {
      id: pkg.id.to_string(),
      source: dependency_source_name(&pkg.source).to_string(),
      requested_rev: pkg.requested_rev.clone(),
      requested_requirement: pkg.requested_requirement.clone(),
      resolved_version: pkg.resolved_version.clone(),
      resolved_commit: pkg.resolved_commit.clone(),
      resolved_ref: None,
      recipe: pkg.recipe_slug.clone(),
      header_only: pkg.header_only,
      header_roots,
      deps: resolved.dependency_ids(pkg.id.as_str()).unwrap_or_default(),
      abi_hash,
      libs,
      linkage,
    });
  }

  packages.sort_by(|a, b| a.id.cmp(&b.id));
  Ok(packages)
}

fn infer_header_roots_from_source_dir(source_dir: &Path) -> Vec<String> {
  let mut roots = ["include", "single_include"]
    .into_iter()
    .filter_map(|candidate| source_dir.join(candidate).is_dir().then_some(candidate.to_string()))
    .collect::<Vec<_>>();

  if roots.is_empty()
    && let Ok(header_root) = linking::discover_header_root(source_dir)
    && let Ok(relative) = header_root.strip_prefix(source_dir)
  {
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if !normalized.is_empty() {
      roots.push(normalized);
    }
  }

  roots
}

fn dependency_source_name(source: &crate::manifest::DependencySource) -> &'static str {
  match source {
    crate::manifest::DependencySource::Github => "github",
  }
}

fn linkage_name(linkage: RecipeLinkage) -> String {
  match linkage {
    RecipeLinkage::Static => "static".to_string(),
    RecipeLinkage::Shared => "shared".to_string(),
  }
}

fn target_triple_guess(compiler_kind: toolchain::CompilerKind) -> String {
  std::env::var("TARGET").unwrap_or_else(|_| {
    let env_suffix = if cfg!(windows) {
      match compiler_kind {
        toolchain::CompilerKind::Msvc => "pc-windows-msvc",
        toolchain::CompilerKind::Clang | toolchain::CompilerKind::Gcc => "pc-windows-gnu",
      }
    } else if cfg!(target_os = "macos") {
      "apple-darwin"
    } else {
      "unknown-linux-gnu"
    };
    format!("{}-{env_suffix}", std::env::consts::ARCH)
  })
}

fn refresh_install_index(
  env_layout: &project_env::ProjectEnvLayout,
  manifest: &Manifest,
  native_link: &NativeLinkInputs,
) -> Result<(), JoyError> {
  let index_path = env_layout.state_dir.join("install-index.json");
  let mut index = InstallIndex::load_or_default(&index_path)
    .map_err(|err| JoyError::new("build", "state_index_error", err.to_string(), 1))?;

  let desired_header_links = manifest
    .dependencies
    .keys()
    .filter_map(|id| crate::package_id::PackageId::parse(id).ok())
    .map(|pkg| env_layout.include_dir.join("deps").join(pkg.slug()))
    .collect::<BTreeSet<_>>();
  let desired_library_files =
    native_link.installed_lib_files.iter().cloned().collect::<BTreeSet<_>>();

  crate::install_index::cleanup_tracked_orphans(
    &index,
    &desired_header_links,
    &desired_library_files,
  )
  .map_err(|err| JoyError::new("build", "state_cleanup_failed", err.to_string(), 1))?;

  index.set_header_links(desired_header_links);
  index.set_library_files(desired_library_files);
  index
    .save(&index_path)
    .map_err(|err| JoyError::new("build", "state_index_error", err.to_string(), 1))
}

#[derive(Debug, Clone)]
struct LockfilePlan {
  write_after_build: bool,
  locked_existing: Option<lockfile::Lockfile>,
}

fn write_lockfile_if_needed(
  command: &'static str,
  lock_plan: LockfilePlan,
  lockfile_path: &Path,
  manifest_hash: &str,
  lockfile_packages: &[lockfile::LockedPackage],
) -> Result<bool, JoyError> {
  if !lock_plan.write_after_build {
    return Ok(false);
  }

  let lock = lockfile::Lockfile {
    version: lockfile::Lockfile::VERSION,
    manifest_hash: manifest_hash.to_string(),
    generated_by: lockfile::generated_by_string(),
    packages: lockfile_packages.to_vec(),
  };
  lock
    .save(lockfile_path)
    .map_err(|err| JoyError::new(command, "lockfile_write_failed", err.to_string(), 1))?;
  Ok(true)
}

fn validate_locked_package_metadata_if_needed(
  command: &'static str,
  lock_plan: &LockfilePlan,
  expected_packages: &[lockfile::LockedPackage],
) -> Result<(), JoyError> {
  let Some(lock) = lock_plan.locked_existing.as_ref() else {
    return Ok(());
  };

  if !expected_packages.is_empty() && lock.packages.is_empty() {
    return Err(JoyError::new(
      command,
      "lockfile_incomplete",
      lockfile_refresh_message(
        "joy.lock package metadata is missing for current dependencies",
        lockfile_refresh_example(command),
      ),
      1,
    ));
  }

  let mut expected = expected_packages.to_vec();
  sort_locked_packages(&mut expected);
  let mut actual = lock.packages.clone();
  sort_locked_packages(&mut actual);

  if actual != expected {
    return Err(JoyError::new(
      command,
      "lockfile_mismatch",
      lockfile_refresh_message(
        "joy.lock package metadata does not match the resolved dependency graph",
        lockfile_refresh_example(command),
      ),
      1,
    ));
  }

  Ok(())
}

fn sort_locked_packages(packages: &mut [lockfile::LockedPackage]) {
  packages.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.requested_rev.cmp(&b.requested_rev)));
}

fn lockfile_refresh_example(command: &'static str) -> String {
  format!("joy {command} --update-lock")
}

fn lockfile_refresh_message(problem: &str, example_command: String) -> String {
  format!("{problem}; rerun with `--update-lock` (for example `{example_command}`)")
}

fn evaluate_lockfile_plan(
  command: &'static str,
  lockfile_path: &Path,
  manifest_hash: &str,
  options: &BuildOptions,
) -> Result<LockfilePlan, JoyError> {
  if options.locked && options.update_lock {
    return Err(JoyError::new(
      command,
      "invalid_lock_flags",
      "cannot use --locked and --update-lock together",
      1,
    ));
  }

  let lock_exists = lockfile_path.is_file();
  let lock = if lock_exists {
    Some(
      lockfile::Lockfile::load(lockfile_path)
        .map_err(|err| JoyError::new(command, "lockfile_parse_error", err.to_string(), 1))?,
    )
  } else {
    None
  };

  if options.locked {
    let Some(lock) = lock else {
      return Err(JoyError::new(
        command,
        "lockfile_missing",
        format!(
          "`--locked` requires `{}` to exist; create or refresh it with `{}` (or rerun with `--update-lock`)",
          lockfile_path.display(),
          lockfile_refresh_example(command),
        ),
        1,
      ));
    };
    if lock.manifest_hash != manifest_hash {
      return Err(JoyError::new(
        command,
        "lockfile_stale",
        lockfile_refresh_message(
          "joy.lock manifest hash does not match joy.toml",
          lockfile_refresh_example(command),
        ),
        1,
      ));
    }
    return Ok(LockfilePlan { write_after_build: false, locked_existing: Some(lock) });
  }

  if let Some(ref lock) = lock
    && lock.manifest_hash != manifest_hash
    && !options.update_lock
  {
    return Err(JoyError::new(
      command,
      "lockfile_stale",
      lockfile_refresh_message(
        "joy.lock manifest hash does not match joy.toml",
        lockfile_refresh_example(command),
      ),
      1,
    ));
  }

  let stale = lock.as_ref().is_some_and(|l| l.manifest_hash != manifest_hash);
  Ok(LockfilePlan {
    write_after_build: options.update_lock || !lock_exists || stale,
    locked_existing: None,
  })
}

fn map_toolchain_error(command: &'static str, err: toolchain::ToolchainError) -> JoyError {
  let message = err.to_string();
  let code = match &err {
    toolchain::ToolchainError::NinjaNotFound | toolchain::ToolchainError::CompilerNotFound => {
      "toolchain_not_found"
    },
    toolchain::ToolchainError::Spawn { .. } | toolchain::ToolchainError::CommandFailed { .. } => {
      "toolchain_probe_failed"
    },
  };
  JoyError::new(command, code, message, 1)
}

fn map_fetch_error(command: &'static str, err: fetch::FetchError) -> JoyError {
  let code = if err.is_offline_cache_miss() {
    "offline_cache_miss"
  } else if err.is_offline_network_disabled() {
    "offline_network_disabled"
  } else if err.is_invalid_version_requirement() {
    "invalid_version_requirement"
  } else if err.is_version_not_found() {
    "version_not_found"
  } else {
    "fetch_failed"
  };
  JoyError::new(command, code, err.to_string(), 1)
}

fn map_dependency_resolve_error(err: resolver::ResolverError) -> JoyError {
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
    _ => "dependency_resolve_failed",
  };
  JoyError::new("build", code, err.to_string(), 1)
}
