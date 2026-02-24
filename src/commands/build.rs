use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::BuildArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::manifest::Manifest;
use crate::ninja::{BuildProfile, NinjaBuildSpec};
use crate::{ninja, project_env, toolchain};

#[derive(Debug, Clone)]
pub(crate) struct BuildExecution {
  pub project_root: PathBuf,
  pub manifest_path: PathBuf,
  pub build_file: PathBuf,
  pub binary_path: PathBuf,
  pub source_file: PathBuf,
  pub include_dirs: Vec<PathBuf>,
  pub toolchain: toolchain::Toolchain,
  pub profile: BuildProfile,
  pub ninja_status: i32,
  pub ninja_stdout: String,
  pub ninja_stderr: String,
}

pub fn handle(args: BuildArgs) -> Result<CommandOutput, JoyError> {
  let execution = build_project(args.release)?;

  Ok(CommandOutput::new(
    "build",
    format!(
      "Built `{}` using {} {}",
      execution.binary_path.display(),
      execution.toolchain.compiler.kind.as_str(),
      execution.toolchain.compiler.version
    ),
    json!({
      "project_root": execution.project_root.display().to_string(),
      "manifest_path": execution.manifest_path.display().to_string(),
      "build_file": execution.build_file.display().to_string(),
      "binary_path": execution.binary_path.display().to_string(),
      "source_file": execution.source_file.display().to_string(),
      "profile": match execution.profile { BuildProfile::Debug => "debug", BuildProfile::Release => "release" },
      "include_dirs": execution.include_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
      "toolchain": {
        "compiler_kind": execution.toolchain.compiler.kind.as_str(),
        "compiler_version": execution.toolchain.compiler.version,
        "compiler_path": execution.toolchain.compiler.path.display().to_string(),
        "ninja_path": execution.toolchain.ninja.path.display().to_string(),
      },
      "ninja_status": execution.ninja_status,
      "ninja_stdout": execution.ninja_stdout,
      "ninja_stderr": execution.ninja_stderr,
    }),
  ))
}

pub(crate) fn build_project(release: bool) -> Result<BuildExecution, JoyError> {
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
  let env_layout = project_env::ensure_layout(&project_root)
    .map_err(|err| JoyError::new("build", "env_setup_failed", err.to_string(), 1))?;

  let toolchain = toolchain::discover().map_err(map_toolchain_error)?;
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

  let spec = NinjaBuildSpec {
    compiler_executable: toolchain.compiler.executable_name.clone(),
    cpp_standard: manifest.project.cpp_standard.clone(),
    source_file: relative_or_absolute(&project_root, &source_file),
    object_file: relative_or_absolute(&project_root, &object_file),
    binary_file: relative_or_absolute(&project_root, &binary_path),
    include_dirs: include_dirs.iter().map(|dir| relative_or_absolute(&project_root, dir)).collect(),
    link_dirs: Vec::new(),
    link_libs: Vec::new(),
    profile: BuildProfile::from_release_flag(release),
  };
  ninja::write_build_ninja(&build_file, &spec)
    .map_err(|err| JoyError::new("build", "ninja_file_write_failed", err.to_string(), 1))?;

  let output = Command::new(&toolchain.ninja.path)
    .current_dir(&project_root)
    .arg("-f")
    .arg(relative_or_absolute(&project_root, &build_file))
    .output()
    .map_err(|err| {
      JoyError::new("build", "ninja_spawn_failed", format!("failed to run ninja: {err}"), 1)
    })?;
  let ninja_stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let ninja_stderr = String::from_utf8_lossy(&output.stderr).to_string();
  if !output.status.success() {
    return Err(JoyError::new(
      "build",
      "build_failed",
      format!(
        "ninja build failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        ninja_stdout.trim_end(),
        ninja_stderr.trim_end()
      ),
      1,
    ));
  }

  Ok(BuildExecution {
    project_root,
    manifest_path,
    build_file,
    binary_path,
    source_file,
    include_dirs,
    toolchain,
    profile: BuildProfile::from_release_flag(release),
    ninja_status: output.status.code().unwrap_or_default(),
    ninja_stdout,
    ninja_stderr,
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

fn relative_or_absolute(root: &Path, path: &Path) -> PathBuf {
  path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn binary_name(project_name: &str) -> String {
  if cfg!(windows) { format!("{project_name}.exe") } else { project_name.to_string() }
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
