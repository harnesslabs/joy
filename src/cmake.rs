//! CMake build adapter for compiled third-party dependencies.
//!
//! The adapter builds recipe-backed dependencies into an ABI-keyed global cache layout and copies
//! library/header artifacts into stable cache directories that can later be installed into project
//! local `.joy/` paths.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

use crate::fs_ops;
use crate::global_cache::BuildCacheLayout;
use crate::ninja::BuildProfile;
use crate::toolchain::CompilerKind;

/// Inputs for building a recipe-backed dependency into an ABI cache directory.
#[derive(Debug, Clone)]
pub struct CmakeBuildRequest {
  pub source_dir: PathBuf,
  pub build_layout: BuildCacheLayout,
  pub profile: BuildProfile,
  pub compiler_kind: CompilerKind,
  pub compiler_path: PathBuf,
  pub configure_args: Vec<String>,
  pub build_targets: Vec<String>,
  pub header_roots: Vec<String>,
}

/// Indexed result of a CMake build cache execution (or cache hit).
#[derive(Debug, Clone)]
pub struct CmakeBuildResult {
  pub cache_hit: bool,
  pub lib_files: Vec<PathBuf>,
  pub bin_files: Vec<PathBuf>,
  pub include_paths: Vec<PathBuf>,
  pub manifest_file: PathBuf,
}

/// Build a CMake project into the provided ABI cache layout.
///
/// A cache hit is returned if the cache manifest already exists and artifact directories are
/// populated.
pub fn build_into_cache(request: &CmakeBuildRequest) -> Result<CmakeBuildResult, CmakeError> {
  ensure_tools()?;
  ensure_layout_dirs(&request.build_layout)?;

  if request.build_layout.manifest_file.is_file() {
    let indexed = index_cached_artifacts(&request.build_layout)?;
    if !indexed.lib_files.is_empty() || !indexed.bin_files.is_empty() {
      return Ok(CmakeBuildResult { cache_hit: true, ..indexed });
    }
  }

  if request.build_layout.work_dir.exists() {
    remove_dir_contents(&request.build_layout.work_dir)?;
  }

  run_cmake_configure(request)?;
  run_cmake_build(request)?;

  let (lib_files, bin_files) = collect_and_copy_artifacts(&request.build_layout)?;
  let include_paths = copy_header_roots_if_present(
    &request.source_dir,
    &request.build_layout.include_dir,
    &request.header_roots,
  )?;

  write_manifest(
    &request.build_layout.manifest_file,
    &request.source_dir,
    request,
    &lib_files,
    &bin_files,
    &include_paths,
  )?;

  Ok(CmakeBuildResult {
    cache_hit: false,
    lib_files,
    bin_files,
    include_paths,
    manifest_file: request.build_layout.manifest_file.clone(),
  })
}

fn ensure_tools() -> Result<(), CmakeError> {
  which::which("cmake").map_err(|_| CmakeError::ToolNotFound("cmake"))?;
  which::which("ninja")
    .or_else(|_| which::which("ninja-build"))
    .map_err(|_| CmakeError::ToolNotFound("ninja"))?;
  Ok(())
}

fn ensure_layout_dirs(layout: &BuildCacheLayout) -> Result<(), CmakeError> {
  for path in [
    &layout.root,
    &layout.work_dir,
    &layout.lib_dir,
    &layout.bin_dir,
    &layout.include_dir,
    &layout.state_dir,
  ] {
    fs::create_dir_all(path).map_err(|source| CmakeError::Io {
      action: "creating build cache directory".into(),
      path: path.to_path_buf(),
      source,
    })?;
  }
  Ok(())
}

fn remove_dir_contents(dir: &Path) -> Result<(), CmakeError> {
  if !dir.is_dir() {
    return Ok(());
  }
  for entry in fs::read_dir(dir).map_err(|source| CmakeError::Io {
    action: "reading build work directory".into(),
    path: dir.to_path_buf(),
    source,
  })? {
    let entry = entry.map_err(|source| CmakeError::Io {
      action: "iterating build work directory".into(),
      path: dir.to_path_buf(),
      source,
    })?;
    let path = entry.path();
    fs_ops::remove_path_if_exists(&path).map_err(|source| CmakeError::Io {
      action: "removing stale cached path".into(),
      path,
      source,
    })?;
  }
  Ok(())
}

fn run_cmake_configure(request: &CmakeBuildRequest) -> Result<(), CmakeError> {
  let mut args = vec![
    "-S".to_string(),
    request.source_dir.display().to_string(),
    "-B".to_string(),
    request.build_layout.work_dir.display().to_string(),
    "-G".to_string(),
    "Ninja".to_string(),
    format!(
      "-DCMAKE_BUILD_TYPE={}",
      match request.profile {
        BuildProfile::Debug => "Debug",
        BuildProfile::Release => "Release",
      }
    ),
  ];
  args.extend(cmake_compiler_args(request));
  args.extend(request.configure_args.iter().cloned());
  // TODO(phase7): Consider normalizing duplicate/conflicting configure args before invocation so
  // recipe defaults and user overrides can compose predictably.
  run_cmake(&args, "configuring CMake project")
}

fn cmake_compiler_args(request: &CmakeBuildRequest) -> Vec<String> {
  let compiler = request.compiler_path.display().to_string();
  let mut args = vec![format!("-DCMAKE_CXX_COMPILER={compiler}")];
  if request.compiler_kind == CompilerKind::Msvc {
    // `cl.exe` is the driver for both C and C++ in a VS developer environment.
    args.push(format!("-DCMAKE_C_COMPILER={compiler}"));
  }
  args
}

fn run_cmake_build(request: &CmakeBuildRequest) -> Result<(), CmakeError> {
  let mut args = vec!["--build".to_string(), request.build_layout.work_dir.display().to_string()];
  if !request.build_targets.is_empty() {
    args.push("--target".to_string());
    args.extend(request.build_targets.iter().cloned());
  }
  run_cmake(&args, "building CMake targets")
}

fn run_cmake(args: &[String], action: &str) -> Result<(), CmakeError> {
  let output = Command::new("cmake")
    .args(args)
    .output()
    .map_err(|source| CmakeError::Spawn { action: action.into(), source })?;
  if output.status.success() {
    Ok(())
  } else {
    Err(CmakeError::CommandFailed {
      action: action.into(),
      status: output.status.code(),
      stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
      stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
  }
}

fn collect_and_copy_artifacts(
  layout: &BuildCacheLayout,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), CmakeError> {
  // TODO(phase7): Split artifact discovery from copy/staging so future recipe metadata can express
  // explicit output locations and names instead of relying on broad file scanning.
  let mut lib_files = Vec::new();
  let mut bin_files = Vec::new();
  let mut stack = vec![layout.work_dir.clone()];

  while let Some(dir) = stack.pop() {
    for entry in fs::read_dir(&dir).map_err(|source| CmakeError::Io {
      action: "scanning cmake build outputs".into(),
      path: dir.clone(),
      source,
    })? {
      let entry = entry.map_err(|source| CmakeError::Io {
        action: "iterating cmake build outputs".into(),
        path: dir.clone(),
        source,
      })?;
      let path = entry.path();
      if entry
        .file_type()
        .map_err(|source| CmakeError::Io {
          action: "reading file type while scanning build outputs".into(),
          path: path.clone(),
          source,
        })?
        .is_dir()
      {
        stack.push(path);
        continue;
      }

      let Some(kind) = classify_artifact_path(&path) else {
        continue;
      };
      let target_dir = match kind {
        ArtifactKind::Library => &layout.lib_dir,
        ArtifactKind::Binary => &layout.bin_dir,
      };
      let file_name = path.file_name().ok_or_else(|| CmakeError::Io {
        action: "reading build artifact filename".into(),
        path: path.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, "missing file name"),
      })?;
      let dst = target_dir.join(file_name);
      fs::copy(&path, &dst).map_err(|source| CmakeError::Io {
        action: format!("copying build artifact `{}`", path.display()),
        path: dst.clone(),
        source,
      })?;
      match kind {
        ArtifactKind::Library => lib_files.push(dst),
        ArtifactKind::Binary => bin_files.push(dst),
      }
    }
  }

  lib_files.sort();
  lib_files.dedup();
  bin_files.sort();
  bin_files.dedup();
  Ok((lib_files, bin_files))
}

fn copy_header_roots_if_present(
  source_dir: &Path,
  cache_include_dir: &Path,
  header_roots: &[String],
) -> Result<Vec<PathBuf>, CmakeError> {
  let mut include_paths = Vec::new();
  for root in header_roots {
    let src = source_dir.join(root);
    if !src.exists() {
      continue;
    }
    let leaf = sanitized_header_root_name(root);
    let dst = cache_include_dir.join(leaf);
    if dst.exists() {
      fs_ops::remove_path_if_exists(&dst).map_err(|source| CmakeError::Io {
        action: "removing stale cached path".into(),
        path: dst.clone(),
        source,
      })?;
    }
    if src.is_dir() {
      copy_dir_recursive(&src, &dst)?;
    } else {
      fs::create_dir_all(&dst).map_err(|source| CmakeError::Io {
        action: "creating cached include root directory".into(),
        path: dst.clone(),
        source,
      })?;
    }
    include_paths.push(dst);
  }
  include_paths.sort();
  Ok(include_paths)
}

fn sanitized_header_root_name(root: &str) -> String {
  let trimmed = root.trim_matches('/');
  if trimmed.is_empty() || trimmed == "." {
    "root".to_string()
  } else {
    trimmed.replace(['/', '\\'], "_")
  }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), CmakeError> {
  fs::create_dir_all(dst).map_err(|source| CmakeError::Io {
    action: "creating cached include directory".into(),
    path: dst.to_path_buf(),
    source,
  })?;
  for entry in fs::read_dir(src).map_err(|source| CmakeError::Io {
    action: "reading include source directory".into(),
    path: src.to_path_buf(),
    source,
  })? {
    let entry = entry.map_err(|source| CmakeError::Io {
      action: "iterating include source directory".into(),
      path: src.to_path_buf(),
      source,
    })?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());
    if entry
      .file_type()
      .map_err(|source| CmakeError::Io {
        action: "reading include source file type".into(),
        path: src_path.clone(),
        source,
      })?
      .is_dir()
    {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else {
      fs::copy(&src_path, &dst_path).map_err(|source| CmakeError::Io {
        action: format!("copying include file `{}`", src_path.display()),
        path: dst_path.clone(),
        source,
      })?;
    }
  }
  Ok(())
}

fn write_manifest(
  manifest_file: &Path,
  source_dir: &Path,
  request: &CmakeBuildRequest,
  lib_files: &[PathBuf],
  bin_files: &[PathBuf],
  include_paths: &[PathBuf],
) -> Result<(), CmakeError> {
  if let Some(parent) = manifest_file.parent() {
    fs::create_dir_all(parent).map_err(|source| CmakeError::Io {
      action: "creating build manifest parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }

  let payload = json!({
    "source_dir": source_dir.display().to_string(),
    "profile": match request.profile { BuildProfile::Debug => "debug", BuildProfile::Release => "release" },
    "configure_args": request.configure_args,
    "build_targets": request.build_targets,
    "header_roots": request.header_roots,
    "lib_files": lib_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    "bin_files": bin_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    "include_paths": include_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
  });
  let raw = serde_json::to_vec_pretty(&payload).map_err(CmakeError::SerializeManifest)?;
  fs::write(manifest_file, raw).map_err(|source| CmakeError::Io {
    action: "writing build cache manifest".into(),
    path: manifest_file.to_path_buf(),
    source,
  })
}

fn index_cached_artifacts(layout: &BuildCacheLayout) -> Result<CmakeBuildResult, CmakeError> {
  let lib_files = list_files(&layout.lib_dir)?;
  let bin_files = list_files(&layout.bin_dir)?;
  let include_paths = list_dirs(&layout.include_dir)?;
  Ok(CmakeBuildResult {
    cache_hit: true,
    lib_files,
    bin_files,
    include_paths,
    manifest_file: layout.manifest_file.clone(),
  })
}

fn list_files(dir: &Path) -> Result<Vec<PathBuf>, CmakeError> {
  if !dir.is_dir() {
    return Ok(Vec::new());
  }
  let mut out = Vec::new();
  for entry in fs::read_dir(dir).map_err(|source| CmakeError::Io {
    action: "listing cached files".into(),
    path: dir.to_path_buf(),
    source,
  })? {
    let entry = entry.map_err(|source| CmakeError::Io {
      action: "iterating cached files".into(),
      path: dir.to_path_buf(),
      source,
    })?;
    let path = entry.path();
    if path.is_file() {
      out.push(path);
    }
  }
  out.sort();
  Ok(out)
}

fn list_dirs(dir: &Path) -> Result<Vec<PathBuf>, CmakeError> {
  if !dir.is_dir() {
    return Ok(Vec::new());
  }
  let mut out = Vec::new();
  for entry in fs::read_dir(dir).map_err(|source| CmakeError::Io {
    action: "listing cached include dirs".into(),
    path: dir.to_path_buf(),
    source,
  })? {
    let entry = entry.map_err(|source| CmakeError::Io {
      action: "iterating cached include dirs".into(),
      path: dir.to_path_buf(),
      source,
    })?;
    let path = entry.path();
    if path.is_dir() {
      out.push(path);
    }
  }
  out.sort();
  Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtifactKind {
  Library,
  Binary,
}

fn classify_artifact_path(path: &Path) -> Option<ArtifactKind> {
  let file_name = path.file_name()?.to_str()?;
  let ext = path.extension()?.to_str()?;

  if file_name.ends_with(".so") || file_name.contains(".so.") {
    return Some(ArtifactKind::Library);
  }

  match ext {
    "a" | "lib" | "dylib" => Some(ArtifactKind::Library),
    "dll" | "exe" => Some(ArtifactKind::Binary),
    _ => None,
  }
}

#[derive(Debug, Error)]
pub enum CmakeError {
  #[error("required tool `{0}` was not found on PATH")]
  ToolNotFound(&'static str),
  #[error("failed to spawn cmake while {action}: {source}")]
  Spawn {
    action: String,
    #[source]
    source: std::io::Error,
  },
  #[error("cmake failed while {action} (status {status:?})\nstdout: {stdout}\nstderr: {stderr}")]
  CommandFailed { action: String, status: Option<i32>, stdout: String, stderr: String },
  #[error("filesystem error while {action} at `{path}`: {source}")]
  Io {
    action: String,
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to serialize cmake cache manifest: {0}")]
  SerializeManifest(serde_json::Error),
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;

  use tempfile::TempDir;

  use super::{CmakeBuildRequest, build_into_cache, classify_artifact_path};
  use crate::global_cache::GlobalCache;
  use crate::ninja::BuildProfile;
  use crate::toolchain::CompilerKind;

  #[test]
  fn classifies_common_library_and_binary_artifacts() {
    let lib_a = std::path::Path::new("/tmp/libfmt.a");
    let lib_so = std::path::Path::new("/tmp/libfmt.so.1");
    let lib_dylib = std::path::Path::new("/tmp/libfmt.dylib");
    let bin_exe = std::path::Path::new("/tmp/demo.exe");
    let dll = std::path::Path::new("/tmp/fmt.dll");
    let txt = std::path::Path::new("/tmp/readme.txt");

    assert!(classify_artifact_path(lib_a).is_some());
    assert!(classify_artifact_path(lib_so).is_some());
    assert!(classify_artifact_path(lib_dylib).is_some());
    assert!(classify_artifact_path(bin_exe).is_some());
    assert!(classify_artifact_path(dll).is_some());
    assert!(classify_artifact_path(txt).is_none());
  }

  #[test]
  fn builds_local_cmake_fixture_into_cache_when_tools_are_available() {
    if which::which("cmake").is_err()
      || (which::which("ninja").is_err() && which::which("ninja-build").is_err())
    {
      eprintln!("skipping cmake adapter test: cmake/ninja not available");
      return;
    }

    let temp = TempDir::new().expect("tempdir");
    let source_dir = temp.path().join("srcpkg");
    fs::create_dir_all(source_dir.join("include")).expect("include dir");
    fs::write(source_dir.join("include/demo.hpp"), "#pragma once\nint demo();\n").expect("header");
    fs::write(source_dir.join("demo.cpp"), "int demo() { return 42; }\n").expect("source");
    fs::write(
      source_dir.join("CMakeLists.txt"),
      r#"cmake_minimum_required(VERSION 3.16)
project(demo LANGUAGES CXX)
add_library(demo STATIC demo.cpp)
target_include_directories(demo PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
"#,
    )
    .expect("cmakelists");

    let cache = GlobalCache::from_joy_home(temp.path().join(".joy"));
    let layout = cache.ensure_compiled_build_layout("cmake-fixture").expect("build layout");
    let request = CmakeBuildRequest {
      source_dir: source_dir.clone(),
      build_layout: layout.clone(),
      profile: BuildProfile::Debug,
      compiler_kind: CompilerKind::Clang,
      compiler_path: std::path::PathBuf::from("clang++"),
      configure_args: Vec::new(),
      build_targets: vec!["demo".into()],
      header_roots: vec!["include".into()],
    };

    let first = build_into_cache(&request).expect("first build");
    assert!(!first.cache_hit);
    assert!(layout.manifest_file.is_file());
    assert!(
      !first.lib_files.is_empty(),
      "expected library artifacts copied into cache, got {:?}",
      first.lib_files
    );
    assert!(layout.include_dir.join("include").is_dir());

    let second = build_into_cache(&request).expect("second build");
    assert!(second.cache_hit);
  }

  #[test]
  fn cmake_configure_args_pin_msvc_c_and_cxx_compilers() {
    let temp = TempDir::new().expect("tempdir");
    let cache = GlobalCache::from_joy_home(temp.path().join(".joy"));
    let layout = cache.ensure_compiled_build_layout("cmake-msvc-args").expect("build layout");
    let request = CmakeBuildRequest {
      source_dir: temp.path().join("src"),
      build_layout: layout,
      profile: BuildProfile::Debug,
      compiler_kind: CompilerKind::Msvc,
      compiler_path: PathBuf::from(r"C:\VS\VC\Tools\MSVC\bin\Hostx64\x64\cl.exe"),
      configure_args: Vec::new(),
      build_targets: Vec::new(),
      header_roots: Vec::new(),
    };

    let args = super::cmake_compiler_args(&request);
    assert!(args.iter().any(|arg| arg.starts_with("-DCMAKE_CXX_COMPILER=")));
    assert!(args.iter().any(|arg| arg.starts_with("-DCMAKE_C_COMPILER=")));
  }
}
