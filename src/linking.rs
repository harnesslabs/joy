//! Local project installation helpers for headers and compiled libraries.
//!
//! Header installs prefer symlinks for speed and disk efficiency, but transparently fall back to a
//! recursive copy when symlink creation is unavailable (common on some Windows setups).

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::package_id::PackageId;

/// Result of installing a header-only package into `.joy/include/deps/<slug>`.
#[derive(Debug, Clone)]
pub struct HeaderInstall {
  pub header_root: PathBuf,
  pub link_path: PathBuf,
  pub link_kind: &'static str,
}

/// Result of installing compiled library artifacts into `.joy/lib`.
#[derive(Debug, Clone)]
pub struct LibraryInstall {
  pub project_lib_dir: PathBuf,
  pub installed_files: Vec<PathBuf>,
  pub link_libs: Vec<String>,
}

/// Discover the package header root used for header installation heuristics.
pub fn discover_header_root(source_dir: &Path) -> Result<PathBuf, LinkingError> {
  for candidate in ["include", "single_include"] {
    let path = source_dir.join(candidate);
    if path.is_dir() {
      return Ok(path);
    }
  }

  Err(LinkingError::NoHeaderRoot { source_dir: source_dir.to_path_buf() })
}

/// Install headers into the project-local include dependency directory.
pub fn install_headers(
  project_include_dir: &Path,
  package: &PackageId,
  source_dir: &Path,
) -> Result<HeaderInstall, LinkingError> {
  install_headers_inner(project_include_dir, package, source_dir, LinkMode::Auto)
}

/// Copy compiled library artifacts from the ABI cache into the project-local `.joy/lib`.
pub fn install_compiled_libraries(
  project_lib_dir: &Path,
  cache_lib_dir: &Path,
  link_libs: &[String],
) -> Result<LibraryInstall, LinkingError> {
  fs::create_dir_all(project_lib_dir).map_err(|source| LinkingError::Io {
    action: "creating project library directory".into(),
    path: project_lib_dir.to_path_buf(),
    source,
  })?;
  if !cache_lib_dir.is_dir() {
    return Err(LinkingError::MissingLibraryArtifact {
      lib: "<cache_lib_dir>".into(),
      cache_lib_dir: cache_lib_dir.to_path_buf(),
    });
  }

  let mut installed_files = Vec::new();
  for lib in link_libs {
    let matches =
      find_library_artifacts(cache_lib_dir, lib).map_err(|source| LinkingError::Io {
        action: format!("scanning cached library artifacts for `{lib}`"),
        path: cache_lib_dir.to_path_buf(),
        source,
      })?;
    if matches.is_empty() {
      return Err(LinkingError::MissingLibraryArtifact {
        lib: lib.clone(),
        cache_lib_dir: cache_lib_dir.to_path_buf(),
      });
    }

    for src in matches {
      let file_name = src.file_name().ok_or_else(|| LinkingError::Io {
        action: "reading library artifact file name".into(),
        path: src.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, "missing file name"),
      })?;
      let file_name_string = file_name.to_string_lossy().into_owned();
      let dst = project_lib_dir.join(file_name);
      fs::copy(&src, &dst).map_err(|source| LinkingError::Io {
        action: format!("copying library artifact `{}`", src.display()),
        path: dst.clone(),
        source,
      })?;
      installed_files.push(dst);

      if let Some(alias_name) = canonical_library_alias_name(&file_name_string, lib) {
        let alias_path = project_lib_dir.join(alias_name);
        if !alias_path.exists() {
          fs::copy(&src, &alias_path).map_err(|source| LinkingError::Io {
            action: format!("copying canonical library alias for `{lib}` from `{}`", src.display()),
            path: alias_path.clone(),
            source,
          })?;
          installed_files.push(alias_path);
        }
      }
    }
  }

  installed_files.sort();
  installed_files.dedup();

  Ok(LibraryInstall {
    project_lib_dir: project_lib_dir.to_path_buf(),
    installed_files,
    link_libs: link_libs.to_vec(),
  })
}

fn install_headers_inner(
  project_include_dir: &Path,
  package: &PackageId,
  source_dir: &Path,
  link_mode: LinkMode,
) -> Result<HeaderInstall, LinkingError> {
  let header_root = discover_header_root(source_dir)?;
  let deps_dir = project_include_dir.join("deps");
  fs::create_dir_all(&deps_dir).map_err(|source| LinkingError::Io {
    action: "creating include/deps directory".into(),
    path: deps_dir.clone(),
    source,
  })?;

  let link_path = deps_dir.join(package.slug());
  remove_existing_path(&link_path).map_err(|source| LinkingError::Io {
    action: "removing existing header link".into(),
    path: link_path.clone(),
    source,
  })?;

  let link_kind = link_or_copy_dir(&header_root, &link_path, link_mode)?;
  Ok(HeaderInstall { header_root, link_path, link_kind })
}

fn remove_existing_path(path: &Path) -> std::io::Result<()> {
  match fs::symlink_metadata(path) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() {
        match fs::remove_file(path) {
          Ok(()) => Ok(()),
          Err(err)
            if matches!(
              err.kind(),
              std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::IsADirectory
            ) =>
          {
            fs::remove_dir(path)
          },
          Err(err) => Err(err),
        }
      } else if metadata.is_file() {
        fs::remove_file(path)
      } else if metadata.is_dir() {
        fs::remove_dir_all(path)
      } else {
        fs::remove_file(path)
      }
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(err) => Err(err),
  }
}

fn find_library_artifacts(cache_lib_dir: &Path, lib: &str) -> std::io::Result<Vec<PathBuf>> {
  let mut matches = Vec::new();
  for entry in fs::read_dir(cache_lib_dir)? {
    let entry = entry?;
    let path = entry.path();
    if !path.is_file() {
      continue;
    }
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
      continue;
    };
    if is_matching_library_file(name, lib) {
      matches.push(path);
    }
  }
  matches.sort();
  Ok(matches)
}

fn is_matching_library_file(file_name: &str, lib: &str) -> bool {
  is_exact_library_file(file_name, lib) || is_exact_library_file(file_name, &format!("{lib}d"))
}

fn is_exact_library_file(file_name: &str, lib: &str) -> bool {
  let unix_static = format!("lib{lib}.a");
  let unix_shared_so = format!("lib{lib}.so");
  let unix_shared_dylib = format!("lib{lib}.dylib");
  let win_import_plain = format!("{lib}.lib");
  let win_import_prefixed = format!("lib{lib}.lib");
  let win_dll = format!("{lib}.dll");
  let versioned_so_prefix = format!("lib{lib}.so.");

  file_name == unix_static
    || file_name == unix_shared_so
    || file_name == unix_shared_dylib
    || file_name == win_import_plain
    || file_name == win_import_prefixed
    || file_name == win_dll
    || file_name.starts_with(&versioned_so_prefix)
}

fn canonical_library_alias_name(file_name: &str, lib: &str) -> Option<String> {
  let debug_lib = format!("{lib}d");
  if !is_exact_library_file(file_name, &debug_lib) || is_exact_library_file(file_name, lib) {
    return None;
  }

  let debug_unix_static = format!("lib{debug_lib}.a");
  if file_name == debug_unix_static {
    return Some(format!("lib{lib}.a"));
  }
  let debug_unix_shared_so = format!("lib{debug_lib}.so");
  if file_name == debug_unix_shared_so {
    return Some(format!("lib{lib}.so"));
  }
  let debug_unix_shared_dylib = format!("lib{debug_lib}.dylib");
  if file_name == debug_unix_shared_dylib {
    return Some(format!("lib{lib}.dylib"));
  }
  let debug_win_import_plain = format!("{debug_lib}.lib");
  if file_name == debug_win_import_plain {
    return Some(format!("{lib}.lib"));
  }
  let debug_win_import_prefixed = format!("lib{debug_lib}.lib");
  if file_name == debug_win_import_prefixed {
    return Some(format!("lib{lib}.lib"));
  }
  let debug_win_dll = format!("{debug_lib}.dll");
  if file_name == debug_win_dll {
    return Some(format!("{lib}.dll"));
  }

  let debug_versioned_so_prefix = format!("lib{debug_lib}.so.");
  if let Some(rest) = file_name.strip_prefix(&debug_versioned_so_prefix) {
    return Some(format!("lib{lib}.so.{rest}"));
  }

  None
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
enum LinkMode {
  Auto,
  CopyOnly,
}

fn link_or_copy_dir(src: &Path, dst: &Path, mode: LinkMode) -> Result<&'static str, LinkingError> {
  if matches!(mode, LinkMode::CopyOnly) {
    copy_dir_recursive(src, dst).map_err(|source| LinkingError::Io {
      action: "copying headers".into(),
      path: dst.to_path_buf(),
      source,
    })?;
    return Ok("copy");
  }

  #[cfg(unix)]
  {
    match std::os::unix::fs::symlink(src, dst) {
      Ok(()) => Ok("symlink"),
      Err(err) => {
        copy_dir_recursive(src, dst).map_err(|source| LinkingError::Io {
          action: format!("copying headers after symlink failure: {err}"),
          path: dst.to_path_buf(),
          source,
        })?;
        Ok("copy")
      },
    }
  }

  #[cfg(windows)]
  {
    // TODO(phase7): Add a Windows junction fallback before full copy for large header trees.
    match std::os::windows::fs::symlink_dir(src, dst) {
      Ok(()) => Ok("symlink"),
      Err(err) => {
        copy_dir_recursive(src, dst).map_err(|source| LinkingError::Io {
          action: format!("copying headers after symlink failure: {err}"),
          path: dst.to_path_buf(),
          source,
        })?;
        Ok("copy")
      },
    }
  }

  #[cfg(not(any(unix, windows)))]
  {
    copy_dir_recursive(src, dst).map_err(|source| LinkingError::Io {
      action: "copying headers".into(),
      path: dst.to_path_buf(),
      source,
    })?;
    Ok("copy")
  }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
  fs::create_dir_all(dst)?;
  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());
    if file_type.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else if file_type.is_symlink() {
      let target = fs::read_link(&src_path)?;
      #[cfg(unix)]
      std::os::unix::fs::symlink(target, &dst_path)?;
      #[cfg(windows)]
      {
        let target_abs =
          if target.is_absolute() { target } else { src_path.parent().unwrap_or(src).join(target) };
        if target_abs.is_dir() {
          std::os::windows::fs::symlink_dir(target_abs, &dst_path)?;
        } else {
          std::os::windows::fs::symlink_file(target_abs, &dst_path)?;
        }
      }
    } else {
      fs::copy(&src_path, &dst_path)?;
    }
  }
  Ok(())
}

#[derive(Debug, Error)]
pub enum LinkingError {
  #[error(
    "no supported header root found in `{source_dir}` (checked `include` and `single_include`)"
  )]
  NoHeaderRoot { source_dir: PathBuf },
  #[error("filesystem error while {action} at `{path}`: {source}")]
  Io {
    action: String,
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("no compiled artifact matching `{lib}` found in `{cache_lib_dir}`")]
  MissingLibraryArtifact { lib: String, cache_lib_dir: PathBuf },
}

#[cfg(test)]
mod tests {
  use std::fs;

  use tempfile::TempDir;

  use super::{discover_header_root, install_compiled_libraries, install_headers};
  use crate::package_id::PackageId;

  #[test]
  fn discovers_include_before_single_include() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(temp.path().join("include")).expect("include dir");
    fs::create_dir_all(temp.path().join("single_include")).expect("single_include dir");

    let root = discover_header_root(temp.path()).expect("header root");
    assert_eq!(root, temp.path().join("include"));
  }

  #[test]
  fn errors_when_no_header_root_exists() {
    let temp = TempDir::new().expect("tempdir");
    let err = discover_header_root(temp.path()).expect_err("missing headers");
    assert!(err.to_string().contains("no supported header root"));
  }

  #[test]
  fn installs_headers_into_project_include_deps_slug() {
    let temp = TempDir::new().expect("tempdir");
    let source_dir = temp.path().join("pkg");
    let include_dir = source_dir.join("include").join("nlohmann");
    fs::create_dir_all(&include_dir).expect("mkdir");
    fs::write(include_dir.join("json.hpp"), "// header\n").expect("write header");

    let project_include = temp.path().join("project").join(".joy").join("include");
    fs::create_dir_all(&project_include).expect("project include");

    let pkg = PackageId::parse("nlohmann/json").expect("package");
    let installed = install_headers(&project_include, &pkg, &source_dir).expect("install headers");

    assert_eq!(installed.link_path, project_include.join("deps").join("nlohmann_json"));
    assert!(installed.link_path.exists());
    assert!(installed.link_path.join("nlohmann").join("json.hpp").is_file());
  }

  #[test]
  fn tests_copy_install_mode_without_relying_on_symlink_failure() {
    let temp = TempDir::new().expect("tempdir");
    let source_dir = temp.path().join("pkg");
    let include_dir = source_dir.join("single_include");
    fs::create_dir_all(&include_dir).expect("mkdir");
    fs::write(include_dir.join("demo.hpp"), "// header\n").expect("write header");

    let project_include = temp.path().join("project").join(".joy").join("include");
    fs::create_dir_all(&project_include).expect("project include");

    let pkg = PackageId::parse("owner/demo").expect("package");
    let installed =
      super::install_headers_inner(&project_include, &pkg, &source_dir, super::LinkMode::CopyOnly)
        .expect("install headers");

    assert_eq!(installed.link_kind, "copy");
    assert!(installed.link_path.join("demo.hpp").is_file());
  }

  #[test]
  fn installs_compiled_library_artifacts_into_project_lib_dir() {
    let temp = TempDir::new().expect("tempdir");
    let cache_lib_dir = temp.path().join("cache").join("lib");
    fs::create_dir_all(&cache_lib_dir).expect("cache lib dir");
    fs::write(cache_lib_dir.join("libfmt.a"), b"stub").expect("static lib");
    fs::write(cache_lib_dir.join("libfmt.so.1"), b"stub").expect("shared lib");
    fs::write(cache_lib_dir.join("README.txt"), b"ignore").expect("noise");

    let project_lib_dir = temp.path().join("project").join(".joy").join("lib");
    let install =
      install_compiled_libraries(&project_lib_dir, &cache_lib_dir, &[String::from("fmt")])
        .expect("install libs");

    assert_eq!(install.project_lib_dir, project_lib_dir);
    assert_eq!(install.link_libs, vec!["fmt"]);
    assert!(install.installed_files.iter().any(|p| p.ends_with("libfmt.a")));
    assert!(install.installed_files.iter().any(|p| p.ends_with("libfmt.so.1")));
    assert!(project_lib_dir.join("libfmt.a").is_file());
    assert!(project_lib_dir.join("libfmt.so.1").is_file());
  }

  #[test]
  fn installs_canonical_alias_for_debug_suffixed_library_artifact() {
    let temp = TempDir::new().expect("tempdir");
    let cache_lib_dir = temp.path().join("cache").join("lib");
    fs::create_dir_all(&cache_lib_dir).expect("cache lib dir");
    fs::write(cache_lib_dir.join("libfmtd.a"), b"stub").expect("debug static lib");

    let project_lib_dir = temp.path().join("project").join(".joy").join("lib");
    let install =
      install_compiled_libraries(&project_lib_dir, &cache_lib_dir, &[String::from("fmt")])
        .expect("install libs");

    assert!(install.installed_files.iter().any(|p| p.ends_with("libfmtd.a")));
    assert!(install.installed_files.iter().any(|p| p.ends_with("libfmt.a")));
    assert!(project_lib_dir.join("libfmtd.a").is_file());
    assert!(project_lib_dir.join("libfmt.a").is_file());
  }

  #[test]
  fn errors_when_requested_compiled_library_artifact_is_missing() {
    let temp = TempDir::new().expect("tempdir");
    let cache_lib_dir = temp.path().join("cache").join("lib");
    fs::create_dir_all(&cache_lib_dir).expect("cache lib dir");

    let project_lib_dir = temp.path().join("project").join(".joy").join("lib");
    let err = install_compiled_libraries(&project_lib_dir, &cache_lib_dir, &[String::from("z")])
      .expect_err("missing lib should fail");
    assert!(err.to_string().contains("no compiled artifact"));
  }
}
