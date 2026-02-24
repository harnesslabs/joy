use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallIndex {
  pub version: u32,
  #[serde(default)]
  pub header_links: BTreeSet<String>,
  #[serde(default)]
  pub library_files: BTreeSet<String>,
}

impl Default for InstallIndex {
  fn default() -> Self {
    Self { version: 1, header_links: BTreeSet::new(), library_files: BTreeSet::new() }
  }
}

impl InstallIndex {
  pub const VERSION: u32 = 1;

  pub fn load_or_default(path: &Path) -> Result<Self, InstallIndexError> {
    match fs::read_to_string(path) {
      Ok(raw) => {
        let parsed: Self = serde_json::from_str(&raw)
          .map_err(|source| InstallIndexError::Parse { path: path.to_path_buf(), source })?;
        if parsed.version != Self::VERSION {
          return Err(InstallIndexError::UnsupportedVersion(parsed.version));
        }
        Ok(parsed)
      },
      Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
      Err(source) => Err(InstallIndexError::Io { path: path.to_path_buf(), source }),
    }
  }

  pub fn save(&self, path: &Path) -> Result<(), InstallIndexError> {
    if self.version != Self::VERSION {
      return Err(InstallIndexError::UnsupportedVersion(self.version));
    }
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)
        .map_err(|source| InstallIndexError::Io { path: parent.to_path_buf(), source })?;
    }
    let raw = serde_json::to_vec_pretty(self).map_err(InstallIndexError::Serialize)?;
    fs::write(path, raw)
      .map_err(|source| InstallIndexError::Io { path: path.to_path_buf(), source })
  }

  pub fn set_header_links<I>(&mut self, paths: I)
  where
    I: IntoIterator<Item = PathBuf>,
  {
    self.header_links = paths.into_iter().map(path_key).collect();
  }

  pub fn set_library_files<I>(&mut self, paths: I)
  where
    I: IntoIterator<Item = PathBuf>,
  {
    self.library_files = paths.into_iter().map(path_key).collect();
  }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CleanupReport {
  pub removed_header_links: Vec<PathBuf>,
  pub removed_library_files: Vec<PathBuf>,
}

pub fn cleanup_tracked_orphans(
  index: &InstallIndex,
  desired_header_links: &BTreeSet<PathBuf>,
  desired_library_files: &BTreeSet<PathBuf>,
) -> Result<CleanupReport, InstallIndexError> {
  let desired_headers: BTreeSet<String> = desired_header_links.iter().map(path_key).collect();
  let desired_libs: BTreeSet<String> = desired_library_files.iter().map(path_key).collect();

  let mut report = CleanupReport::default();

  for tracked in &index.header_links {
    if desired_headers.contains(tracked) {
      continue;
    }
    let path = PathBuf::from(tracked);
    if remove_path_if_exists(&path)? {
      report.removed_header_links.push(path);
    }
  }

  for tracked in &index.library_files {
    if desired_libs.contains(tracked) {
      continue;
    }
    let path = PathBuf::from(tracked);
    if remove_path_if_exists(&path)? {
      report.removed_library_files.push(path);
    }
  }

  report.removed_header_links.sort();
  report.removed_library_files.sort();
  Ok(report)
}

fn path_key(path: impl AsRef<Path>) -> String {
  let path = path.as_ref();
  path.to_string_lossy().to_string()
}

fn remove_path_if_exists(path: &Path) -> Result<bool, InstallIndexError> {
  match fs::symlink_metadata(path) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)
          .map_err(|source| InstallIndexError::Io { path: path.to_path_buf(), source })?;
      } else if metadata.is_dir() {
        fs::remove_dir_all(path)
          .map_err(|source| InstallIndexError::Io { path: path.to_path_buf(), source })?;
      } else {
        fs::remove_file(path)
          .map_err(|source| InstallIndexError::Io { path: path.to_path_buf(), source })?;
      }
      Ok(true)
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
    Err(source) => Err(InstallIndexError::Io { path: path.to_path_buf(), source }),
  }
}

#[derive(Debug, Error)]
pub enum InstallIndexError {
  #[error("filesystem error at `{path}`: {source}")]
  Io {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to parse install index `{path}`: {source}")]
  Parse {
    path: PathBuf,
    #[source]
    source: serde_json::Error,
  },
  #[error("failed to serialize install index: {0}")]
  Serialize(serde_json::Error),
  #[error("unsupported install index version `{0}`")]
  UnsupportedVersion(u32),
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeSet;
  use std::fs;

  use tempfile::TempDir;

  use super::{InstallIndex, cleanup_tracked_orphans};

  #[test]
  fn roundtrips_index_json() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("state/install-index.json");

    let mut index = InstallIndex::default();
    index.set_header_links([temp.path().join(".joy/include/deps/fmt")]);
    index.set_library_files([temp.path().join(".joy/lib/libfmt.a")]);
    index.save(&path).expect("save");

    let loaded = InstallIndex::load_or_default(&path).expect("load");
    assert_eq!(loaded, index);
  }

  #[test]
  fn removes_only_tracked_orphans() {
    let temp = TempDir::new().expect("tempdir");
    let keep_header = temp.path().join(".joy/include/deps/keep");
    let old_header = temp.path().join(".joy/include/deps/old");
    let keep_lib = temp.path().join(".joy/lib/libkeep.a");
    let old_lib = temp.path().join(".joy/lib/libold.a");
    let unknown_file = temp.path().join(".joy/lib/user-note.txt");

    fs::create_dir_all(&keep_header).expect("keep header dir");
    fs::create_dir_all(&old_header).expect("old header dir");
    fs::create_dir_all(keep_lib.parent().expect("lib dir")).expect("lib dir");
    fs::write(&keep_lib, b"keep").expect("keep lib");
    fs::write(&old_lib, b"old").expect("old lib");
    fs::write(&unknown_file, b"note").expect("note");

    let mut index = InstallIndex::default();
    index.set_header_links([keep_header.clone(), old_header.clone()]);
    index.set_library_files([keep_lib.clone(), old_lib.clone()]);

    let mut desired_headers = BTreeSet::new();
    desired_headers.insert(keep_header.clone());
    let mut desired_libs = BTreeSet::new();
    desired_libs.insert(keep_lib.clone());

    let report = cleanup_tracked_orphans(&index, &desired_headers, &desired_libs).expect("cleanup");
    assert!(report.removed_header_links.contains(&old_header));
    assert!(report.removed_library_files.contains(&old_lib));
    assert!(keep_header.exists());
    assert!(keep_lib.exists());
    assert!(unknown_file.exists(), "unknown user file should not be deleted");
  }
}
