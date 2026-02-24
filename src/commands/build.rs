//! `joy build` implementation and shared project build pipeline used by `joy run`.

use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::BuildArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::install_index::InstallIndex;
use crate::lockfile;
use crate::manifest::Manifest;
use crate::ninja::{BuildProfile, NinjaBuildSpec};
use crate::recipes::{Linkage as RecipeLinkage, RecipeStore};
use crate::{abi, cmake, fetch, global_cache, linking, ninja, project_env, resolver, toolchain};

#[derive(Debug, Clone)]
pub(crate) struct BuildExecution {
  pub project_root: PathBuf,
  pub manifest_path: PathBuf,
  pub build_file: PathBuf,
  pub binary_path: PathBuf,
  pub source_file: PathBuf,
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct BuildOptions {
  pub release: bool,
  pub locked: bool,
  pub update_lock: bool,
}

/// Handle `joy build` by executing the local build pipeline and returning a CLI output payload.
pub fn handle(args: BuildArgs) -> Result<CommandOutput, JoyError> {
  let execution = build_project(BuildOptions {
    release: args.release,
    locked: args.locked,
    update_lock: args.update_lock,
  })?;

  Ok(CommandOutput::new(
    "build",
    format!(
      "Built `{}` using {} {}",
      execution.binary_path.display(),
      execution.toolchain.compiler.kind.as_str(),
      execution.toolchain.compiler.version
    ),
    execution.json_data(),
  ))
}

/// Build the current project and return the execution metadata reused by `joy run`.
pub(crate) fn build_project(options: BuildOptions) -> Result<BuildExecution, JoyError> {
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
  let manifest_hash = lockfile::compute_manifest_hash(&manifest_path)
    .map_err(|err| JoyError::new("build", "lockfile_hash_failed", err.to_string(), 1))?;
  let lockfile_path = project_root.join("joy.lock");
  let env_layout = project_env::ensure_layout(&project_root)
    .map_err(|err| JoyError::new("build", "env_setup_failed", err.to_string(), 1))?;
  let lock_plan = evaluate_lockfile_plan(&lockfile_path, &manifest_hash, options)?;

  let toolchain = toolchain::discover().map_err(map_toolchain_error)?;
  let profile = BuildProfile::from_release_flag(options.release);
  let source_file = project_root.join(&manifest.project.entry);
  if !source_file.is_file() {
    return Err(JoyError::new(
      "build",
      "entry_not_found",
      format!("entry source file `{}` does not exist", source_file.display()),
      1,
    ));
  }

  let obj_dir = env_layout.build_dir.join("obj");
  fs::create_dir_all(&obj_dir)
    .map_err(|err| JoyError::io("build", "creating object directory", &obj_dir, &err))?;
  let object_file = obj_dir
    .join(format!("{}.o", source_file.file_stem().and_then(|s| s.to_str()).unwrap_or("main")));
  let binary_path = env_layout.bin_dir.join(binary_name(&manifest.project.name));
  let build_file = env_layout.build_dir.join("build.ninja");
  let include_dirs = collect_include_dirs(&env_layout.include_dir).map_err(|err| {
    JoyError::io("build", "reading include directories", &env_layout.include_dir, &err)
  })?;
  let native_link =
    prepare_compiled_dependencies(&manifest, &env_layout.lib_dir, &toolchain, profile)?;
  refresh_install_index(&env_layout, &manifest, &native_link)?;

  let spec = NinjaBuildSpec {
    compiler_executable: toolchain.compiler.executable_name.clone(),
    cpp_standard: manifest.project.cpp_standard.clone(),
    source_file: relative_or_absolute(&project_root, &source_file),
    object_file: relative_or_absolute(&project_root, &object_file),
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

  let ninja_output = run_ninja_build(&toolchain, &project_root, &build_file)?;

  let lockfile_updated = write_lockfile_if_needed(
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

fn prepare_compiled_dependencies(
  manifest: &Manifest,
  project_lib_dir: &Path,
  toolchain: &toolchain::Toolchain,
  profile: BuildProfile,
) -> Result<NativeLinkInputs, JoyError> {
  // TODO(phase7): Split this function into resolve/prefetch/build/install stages to shrink the
  // surface area of `joy build` and make per-stage diagnostics easier to test in isolation.
  if manifest.dependencies.is_empty() {
    return Ok(NativeLinkInputs::default());
  }

  let recipes = RecipeStore::load_default()
    .map_err(|err| JoyError::new("build", "recipe_load_failed", err.to_string(), 1))?;
  let resolved = resolver::resolve_manifest(manifest, &recipes)
    .map_err(|err| JoyError::new("build", "dependency_resolve_failed", err.to_string(), 1))?;
  let order = resolved
    .build_order()
    .map_err(|err| JoyError::new("build", "dependency_graph_invalid", err.to_string(), 1))?;
  let prefetched = fetch::prefetch_github_packages(
    order.iter().map(|pkg| (pkg.id.clone(), pkg.requested_rev.clone())).collect(),
  )
  .map_err(|err| JoyError::new("build", "fetch_failed", err.to_string(), 1))?;
  let prefetched_by_key = prefetched
    .into_iter()
    .map(|f| ((f.package.to_string(), f.requested_rev.clone()), f))
    .collect::<BTreeMap<_, _>>();
  let mut prefetched_for_build = prefetched_by_key.clone();

  let cache = global_cache::GlobalCache::resolve()
    .map_err(|err| JoyError::new("build", "cache_setup_failed", err.to_string(), 1))?;
  cache
    .ensure_layout()
    .map_err(|err| JoyError::new("build", "cache_setup_failed", err.to_string(), 1))?;

  let mut link_dirs = Vec::<PathBuf>::new();
  let mut link_libs = Vec::<String>::new();
  let mut built_packages = Vec::<String>::new();
  let mut installed_lib_files = Vec::<PathBuf>::new();
  let mut compiled_lock_metadata = BTreeMap::<String, CompiledLockMetadata>::new();

  for pkg in order {
    if pkg.header_only {
      continue;
    }

    let Some(recipe) = recipes.get(&pkg.id) else {
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

    let recipe_file = recipes.root_dir().join("packages").join(format!("{}.toml", recipe.slug));
    let recipe_contents = fs::read_to_string(&recipe_file)
      .map_err(|err| JoyError::io("build", "reading recipe file", &recipe_file, &err))?;
    let preferred_linkage = link_recipe.preferred_linkage.unwrap_or(RecipeLinkage::Static);
    let abi_hash = abi::compute_abi_hash(&abi::AbiHashInput {
      package_id: pkg.id.to_string(),
      resolved_commit: pkg.resolved_commit.clone(),
      recipe_content_hash: abi::hash_recipe_contents(&recipe_contents),
      compiler_kind: toolchain.compiler.kind.as_str().to_string(),
      compiler_version: toolchain.compiler.version.clone(),
      target_triple: target_triple_guess(),
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
    compiled_lock_metadata.insert(
      pkg.id.to_string(),
      CompiledLockMetadata {
        abi_hash: abi_hash.clone(),
        libs: link_recipe.libs.clone(),
        linkage: Some(match preferred_linkage {
          RecipeLinkage::Static => "static".to_string(),
          RecipeLinkage::Shared => "shared".to_string(),
        }),
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
      configure_args: cmake_recipe.configure_args.clone(),
      build_targets: cmake_recipe.build_targets.clone(),
      header_roots: recipe.include_roots().to_vec(),
    })
    .map_err(|err| JoyError::new("build", "cmake_build_failed", err.to_string(), 1))?;
    let lib_install =
      linking::install_compiled_libraries(project_lib_dir, &layout.lib_dir, &link_recipe.libs)
        .map_err(|err| JoyError::new("build", "library_install_failed", err.to_string(), 1))?;

    if !cmake_result.cache_hit {
      built_packages.push(pkg.id.to_string());
    }
    if !link_dirs.iter().any(|p| p == &lib_install.project_lib_dir) {
      link_dirs.push(lib_install.project_lib_dir.clone());
    }
    for lib in lib_install.link_libs {
      if !link_libs.contains(&lib) {
        link_libs.push(lib);
      }
    }
    for file in lib_install.installed_files {
      if !installed_lib_files.contains(&file) {
        installed_lib_files.push(file);
      }
    }
  }

  let lockfile_packages =
    assemble_lockfile_packages(&resolved, &recipes, &prefetched_by_key, &compiled_lock_metadata)?;

  Ok(NativeLinkInputs {
    link_dirs,
    link_libs,
    built_packages,
    installed_lib_files,
    lockfile_packages,
  })
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

fn target_triple_guess() -> String {
  std::env::var("TARGET").unwrap_or_else(|_| {
    let env_suffix = if cfg!(windows) {
      "windows-gnu"
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

#[derive(Debug, Clone, Copy)]
struct LockfilePlan {
  write_after_build: bool,
}

fn write_lockfile_if_needed(
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
    .map_err(|err| JoyError::new("build", "lockfile_write_failed", err.to_string(), 1))?;
  Ok(true)
}

fn evaluate_lockfile_plan(
  lockfile_path: &Path,
  manifest_hash: &str,
  options: BuildOptions,
) -> Result<LockfilePlan, JoyError> {
  if options.locked && options.update_lock {
    return Err(JoyError::new(
      "build",
      "invalid_lock_flags",
      "cannot use --locked and --update-lock together",
      1,
    ));
  }

  let lock_exists = lockfile_path.is_file();
  let lock = if lock_exists {
    Some(
      lockfile::Lockfile::load(lockfile_path)
        .map_err(|err| JoyError::new("build", "lockfile_parse_error", err.to_string(), 1))?,
    )
  } else {
    None
  };

  if options.locked {
    let Some(lock) = lock else {
      return Err(JoyError::new(
        "build",
        "lockfile_missing",
        format!(
          "`--locked` requires `{}` to exist; run `joy build --update-lock` first",
          lockfile_path.display()
        ),
        1,
      ));
    };
    if lock.manifest_hash != manifest_hash {
      return Err(JoyError::new(
        "build",
        "lockfile_stale",
        "lockfile manifest hash does not match joy.toml; rerun with --update-lock".to_string(),
        1,
      ));
    }
    return Ok(LockfilePlan { write_after_build: false });
  }

  if let Some(ref lock) = lock
    && lock.manifest_hash != manifest_hash
    && !options.update_lock
  {
    return Err(JoyError::new(
      "build",
      "lockfile_stale",
      "lockfile manifest hash does not match joy.toml; rerun with --update-lock".to_string(),
      1,
    ));
  }

  let stale = lock.as_ref().is_some_and(|l| l.manifest_hash != manifest_hash);
  Ok(LockfilePlan { write_after_build: options.update_lock || !lock_exists || stale })
}

fn map_toolchain_error(err: toolchain::ToolchainError) -> JoyError {
  let message = err.to_string();
  let code = match &err {
    toolchain::ToolchainError::NinjaNotFound | toolchain::ToolchainError::CompilerNotFound => {
      "toolchain_not_found"
    },
    toolchain::ToolchainError::MsvcUnsupportedPhase4 => "toolchain_unsupported",
    toolchain::ToolchainError::Spawn { .. } | toolchain::ToolchainError::CommandFailed { .. } => {
      "toolchain_probe_failed"
    },
  };
  JoyError::new("build", code, message, 1)
}
