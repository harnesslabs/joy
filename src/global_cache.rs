use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::package_id::PackageId;

/// Global cache roots under `JOY_HOME` (or `~/.joy` by default).
#[derive(Debug, Clone)]
pub struct GlobalCache {
  pub joy_home: PathBuf,
  pub cache_root: PathBuf,
  pub src_root: PathBuf,
  pub git_root: PathBuf,
  pub builds_root: PathBuf,
  pub tmp_root: PathBuf,
}

/// ABI-keyed cache layout used for compiled dependency builds.
#[derive(Debug, Clone)]
pub struct BuildCacheLayout {
  pub root: PathBuf,
  pub work_dir: PathBuf,
  pub lib_dir: PathBuf,
  pub bin_dir: PathBuf,
  pub include_dir: PathBuf,
  pub state_dir: PathBuf,
  pub manifest_file: PathBuf,
}

impl GlobalCache {
  /// Resolve the global cache location from `JOY_HOME`, then HOME/USERPROFILE fallback.
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

  /// Construct cache paths from an explicit `JOY_HOME` root (used heavily in tests).
  pub fn from_joy_home(joy_home: PathBuf) -> Self {
    let cache_root = joy_home.join("cache");
    Self {
      joy_home,
      src_root: cache_root.join("src"),
      git_root: cache_root.join("git"),
      builds_root: cache_root.join("builds"),
      tmp_root: cache_root.join("tmp"),
      cache_root,
    }
  }

  /// Ensure the top-level global cache directory structure exists.
  pub fn ensure_layout(&self) -> Result<(), GlobalCacheError> {
    for path in [
      &self.joy_home,
      &self.cache_root,
      &self.src_root,
      &self.git_root,
      &self.builds_root,
      &self.tmp_root,
    ] {
      fs::create_dir_all(path)
        .map_err(|source| GlobalCacheError::Io { path: path.to_path_buf(), source })?;
    }
    Ok(())
  }

  /// Path for a materialized source checkout pinned by resolved commit.
  pub fn source_checkout_dir(&self, package: &PackageId, commit: &str) -> PathBuf {
    self.src_root.join("github").join(package.owner()).join(package.repo()).join(commit)
  }

  /// Path for the cached git mirror used to resolve refs and clone checkouts quickly.
  pub fn git_mirror_dir(&self, package: &PackageId) -> PathBuf {
    self.git_root.join("github").join(package.owner()).join(format!("{}.git", package.repo()))
  }

  /// Temporary directory root used for atomic-ish cache materialization.
  pub fn tmp_dir(&self) -> &Path {
    &self.tmp_root
  }

  /// Compute the compiled build cache layout for a specific ABI hash.
  pub fn compiled_build_layout(&self, abi_hash: &str) -> BuildCacheLayout {
    let root = self.builds_root.join(abi_hash);
    BuildCacheLayout {
      work_dir: root.join("work"),
      lib_dir: root.join("lib"),
      bin_dir: root.join("bin"),
      include_dir: root.join("include"),
      state_dir: root.join("state"),
      manifest_file: root.join("state").join("build-manifest.json"),
      root,
    }
  }

  /// Ensure the ABI-keyed compiled build layout exists and return it.
  pub fn ensure_compiled_build_layout(
    &self,
    abi_hash: &str,
  ) -> Result<BuildCacheLayout, GlobalCacheError> {
    let layout = self.compiled_build_layout(abi_hash);
    self.ensure_layout()?;
    for path in [
      &layout.root,
      &layout.work_dir,
      &layout.lib_dir,
      &layout.bin_dir,
      &layout.include_dir,
      &layout.state_dir,
    ] {
      fs::create_dir_all(path)
        .map_err(|source| GlobalCacheError::Io { path: path.to_path_buf(), source })?;
    }
    Ok(layout)
  }
}

/// Errors while resolving or creating the global cache layout.
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
  use tempfile::TempDir;

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
    let build_layout = cache.compiled_build_layout("deadbeef");
    assert_eq!(build_layout.root, PathBuf::from("/tmp/joy-home/cache/builds/deadbeef"));
    assert_eq!(build_layout.work_dir, PathBuf::from("/tmp/joy-home/cache/builds/deadbeef/work"));
    assert_eq!(
      build_layout.manifest_file,
      PathBuf::from("/tmp/joy-home/cache/builds/deadbeef/state/build-manifest.json")
    );
  }

  #[test]
  fn ensures_compiled_build_layout_directories() {
    let temp = TempDir::new().expect("tempdir");
    let cache = GlobalCache::from_joy_home(temp.path().join(".joy"));
    let layout = cache.ensure_compiled_build_layout("abc123").expect("ensure compiled layout");

    assert!(layout.root.is_dir());
    assert!(layout.work_dir.is_dir());
    assert!(layout.lib_dir.is_dir());
    assert!(layout.bin_dir.is_dir());
    assert!(layout.include_dir.is_dir());
    assert!(layout.state_dir.is_dir());
  }
}
