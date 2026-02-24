use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::package_id::PackageId;

#[derive(Debug, Clone)]
pub struct GlobalCache {
  pub joy_home: PathBuf,
  pub cache_root: PathBuf,
  pub src_root: PathBuf,
  pub git_root: PathBuf,
  pub archives_root: PathBuf,
  pub builds_root: PathBuf,
  pub tmp_root: PathBuf,
}

impl GlobalCache {
  pub fn resolve() -> Result<Self, GlobalCacheError> {
    if let Some(path) = env::var_os("JOY_HOME") {
      return Ok(Self::from_joy_home(PathBuf::from(path)));
    }

    let home = env::var_os("HOME")
      .or_else(|| env::var_os("USERPROFILE"))
      .map(PathBuf::from)
      .ok_or(GlobalCacheError::HomeDirUnavailable)?;
    Ok(Self::from_joy_home(home.join(".joy")))
  }

  pub fn from_joy_home(joy_home: PathBuf) -> Self {
    let cache_root = joy_home.join("cache");
    Self {
      joy_home,
      src_root: cache_root.join("src"),
      git_root: cache_root.join("git"),
      archives_root: cache_root.join("archives"),
      builds_root: cache_root.join("builds"),
      tmp_root: cache_root.join("tmp"),
      cache_root,
    }
  }

  pub fn ensure_layout(&self) -> Result<(), GlobalCacheError> {
    for path in [
      &self.joy_home,
      &self.cache_root,
      &self.src_root,
      &self.git_root,
      &self.archives_root,
      &self.builds_root,
      &self.tmp_root,
    ] {
      fs::create_dir_all(path)
        .map_err(|source| GlobalCacheError::Io { path: path.to_path_buf(), source })?;
    }
    Ok(())
  }

  pub fn source_checkout_dir(&self, package: &PackageId, commit: &str) -> PathBuf {
    self.src_root.join("github").join(package.owner()).join(package.repo()).join(commit)
  }

  pub fn git_mirror_dir(&self, package: &PackageId) -> PathBuf {
    self.git_root.join("github").join(package.owner()).join(format!("{}.git", package.repo()))
  }

  pub fn source_parent_dir(&self, package: &PackageId) -> PathBuf {
    self.src_root.join("github").join(package.owner()).join(package.repo())
  }

  pub fn tmp_dir(&self) -> &Path {
    &self.tmp_root
  }
}

#[derive(Debug, Error)]
pub enum GlobalCacheError {
  #[error("could not determine a home directory for global joy cache")]
  HomeDirUnavailable,
  #[error("failed to create global cache path `{path}`: {source}")]
  Io {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::GlobalCache;
  use crate::package_id::PackageId;

  #[test]
  fn builds_paths_from_explicit_joy_home() {
    let cache = GlobalCache::from_joy_home(PathBuf::from("/tmp/joy-home"));
    let pkg = PackageId::parse("nlohmann/json").expect("package");

    assert_eq!(cache.cache_root, PathBuf::from("/tmp/joy-home/cache"));
    assert_eq!(
      cache.source_checkout_dir(&pkg, "abc123"),
      PathBuf::from("/tmp/joy-home/cache/src/github/nlohmann/json/abc123")
    );
    assert_eq!(
      cache.git_mirror_dir(&pkg),
      PathBuf::from("/tmp/joy-home/cache/git/github/nlohmann/json.git")
    );
  }
}
