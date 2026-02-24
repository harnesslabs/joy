use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::package_id::PackageId;

#[derive(Debug, Clone)]
pub struct HeaderInstall {
  pub header_root: PathBuf,
  pub link_path: PathBuf,
  pub link_kind: &'static str,
}

pub fn discover_header_root(source_dir: &Path) -> Result<PathBuf, LinkingError> {
  for candidate in ["include", "single_include"] {
    let path = source_dir.join(candidate);
    if path.is_dir() {
      return Ok(path);
    }
  }

  Err(LinkingError::NoHeaderRoot { source_dir: source_dir.to_path_buf() })
}

pub fn install_headers(
  project_include_dir: &Path,
  package: &PackageId,
  source_dir: &Path,
) -> Result<HeaderInstall, LinkingError> {
  install_headers_inner(project_include_dir, package, source_dir, LinkMode::Auto)
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
      if metadata.file_type().is_symlink() || metadata.is_file() {
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
}

#[cfg(test)]
mod tests {
  use std::fs;

  use tempfile::TempDir;

  use super::{discover_header_root, install_headers};
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
}
