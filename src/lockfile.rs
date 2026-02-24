use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lockfile {
  pub version: u32,
  pub manifest_hash: String,
  pub generated_by: String,
  #[serde(default)]
  pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedPackage {
  pub id: String,
  pub source: String,
  pub requested_rev: String,
  pub resolved_commit: String,
  #[serde(default)]
  pub resolved_ref: Option<String>,
  #[serde(default)]
  pub recipe: Option<String>,
  pub header_only: bool,
  #[serde(default)]
  pub header_roots: Vec<String>,
  #[serde(default)]
  pub deps: Vec<String>,
  #[serde(default)]
  pub abi_hash: String,
  #[serde(default)]
  pub libs: Vec<String>,
  #[serde(default)]
  pub linkage: Option<String>,
}

impl Lockfile {
  pub const VERSION: u32 = 1;

  pub fn load(path: &Path) -> Result<Self, LockfileError> {
    let raw = fs::read_to_string(path)
      .map_err(|source| LockfileError::Io { path: path.to_path_buf(), source })?;
    let lock: Self = toml::from_str(&raw)
      .map_err(|source| LockfileError::Parse { path: path.to_path_buf(), source })?;
    if lock.version != Self::VERSION {
      return Err(LockfileError::UnsupportedVersion(lock.version));
    }
    Ok(lock)
  }

  pub fn save(&self, path: &Path) -> Result<(), LockfileError> {
    if self.version != Self::VERSION {
      return Err(LockfileError::UnsupportedVersion(self.version));
    }
    let mut raw = toml::to_string_pretty(self).map_err(LockfileError::Serialize)?;
    if !raw.ends_with('\n') {
      raw.push('\n');
    }
    fs::write(path, raw).map_err(|source| LockfileError::Io { path: path.to_path_buf(), source })
  }
}

pub fn compute_manifest_hash(path: &Path) -> Result<String, LockfileError> {
  let bytes =
    fs::read(path).map_err(|source| LockfileError::Io { path: path.to_path_buf(), source })?;
  Ok(hash_bytes(&bytes))
}

pub fn generated_by_string() -> String {
  format!("joy {}", env!("CARGO_PKG_VERSION"))
}

fn hash_bytes(bytes: &[u8]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  let digest = hasher.finalize();
  let mut out = String::with_capacity(digest.len() * 2);
  for byte in digest {
    use std::fmt::Write as _;
    let _ = write!(&mut out, "{byte:02x}");
  }
  out
}

#[derive(Debug, Error)]
pub enum LockfileError {
  #[error("filesystem error for `{path}`: {source}")]
  Io {
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to parse lockfile `{path}`: {source}")]
  Parse {
    path: PathBuf,
    #[source]
    source: toml::de::Error,
  },
  #[error("failed to serialize lockfile: {0}")]
  Serialize(toml::ser::Error),
  #[error("unsupported lockfile version `{0}`")]
  UnsupportedVersion(u32),
}

#[cfg(test)]
mod tests {
  use tempfile::TempDir;

  use super::{LockedPackage, Lockfile, compute_manifest_hash, generated_by_string};

  #[test]
  fn lockfile_roundtrips() {
    let lock = Lockfile {
      version: Lockfile::VERSION,
      manifest_hash: "abc".into(),
      generated_by: generated_by_string(),
      packages: vec![LockedPackage {
        id: "nlohmann/json".into(),
        source: "github".into(),
        requested_rev: "HEAD".into(),
        resolved_commit: "deadbeef".into(),
        resolved_ref: Some("refs/heads/main".into()),
        recipe: Some("nlohmann_json".into()),
        header_only: true,
        header_roots: vec!["include".into()],
        deps: Vec::new(),
        abi_hash: String::new(),
        libs: Vec::new(),
        linkage: None,
      }],
    };

    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("joy.lock");
    lock.save(&path).expect("save lockfile");
    let loaded = Lockfile::load(&path).expect("load lockfile");
    assert_eq!(loaded, lock);
  }

  #[test]
  fn manifest_hash_changes_with_contents() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("joy.toml");
    std::fs::write(&path, "[project]\nname = \"a\"\nversion=\"0.1.0\"\ncpp_standard=\"c++20\"\nentry=\"src/main.cpp\"\n")
      .expect("write");
    let a = compute_manifest_hash(&path).expect("hash a");
    std::fs::write(&path, "[project]\nname = \"b\"\nversion=\"0.1.0\"\ncpp_standard=\"c++20\"\nentry=\"src/main.cpp\"\n")
      .expect("write");
    let b = compute_manifest_hash(&path).expect("hash b");
    assert_ne!(a, b);
  }
}
