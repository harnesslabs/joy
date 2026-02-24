use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ProjectEnvLayout {
  pub root: PathBuf,
  pub include_dir: PathBuf,
  pub lib_dir: PathBuf,
  pub build_dir: PathBuf,
  pub bin_dir: PathBuf,
  pub state_dir: PathBuf,
  pub created_paths: Vec<PathBuf>,
}

pub fn ensure_layout(project_root: &Path) -> Result<ProjectEnvLayout, ProjectEnvError> {
  let joy_root = project_root.join(".joy");
  let include_dir = joy_root.join("include");
  let lib_dir = joy_root.join("lib");
  let build_dir = joy_root.join("build");
  let bin_dir = joy_root.join("bin");
  let state_dir = joy_root.join("state");

  let mut created_paths = Vec::new();
  for path in [&joy_root, &include_dir, &lib_dir, &build_dir, &bin_dir, &state_dir] {
    if !path.exists() {
      fs::create_dir_all(path)
        .map_err(|source| ProjectEnvError::Io { path: path.to_path_buf(), source })?;
      created_paths.push(path.to_path_buf());
    }
  }

  Ok(ProjectEnvLayout {
    root: joy_root,
    include_dir,
    lib_dir,
    build_dir,
    bin_dir,
    state_dir,
    created_paths,
  })
}

#[derive(Debug, Error)]
pub enum ProjectEnvError {
  #[error("failed to create local joy environment path `{path}`: {source}")]
  Io {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
}

#[cfg(test)]
mod tests {
  use tempfile::TempDir;

  use super::ensure_layout;

  #[test]
  fn ensure_layout_creates_expected_directories() {
    let temp = TempDir::new().expect("tempdir");
    let layout = ensure_layout(temp.path()).expect("create layout");

    assert!(layout.root.is_dir());
    assert!(layout.include_dir.is_dir());
    assert!(layout.lib_dir.is_dir());
    assert!(layout.build_dir.is_dir());
    assert!(layout.bin_dir.is_dir());
    assert!(layout.state_dir.is_dir());
    assert!(layout.created_paths.len() >= 6);
  }
}
