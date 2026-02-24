use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AbiBuildProfile {
  Debug,
  Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AbiLinkage {
  Static,
  Shared,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AbiHashInput {
  pub package_id: String,
  pub resolved_commit: String,
  pub recipe_content_hash: String,
  pub compiler_kind: String,
  pub compiler_version: String,
  pub target_triple: String,
  pub host_os: String,
  pub host_arch: String,
  pub profile: AbiBuildProfile,
  pub cpp_standard: String,
  pub linkage: AbiLinkage,
  #[serde(default)]
  pub cxxflags: Vec<String>,
  #[serde(default)]
  pub ldflags: Vec<String>,
  #[serde(default)]
  pub recipe_configure_args: Vec<String>,
  #[serde(default)]
  pub env: BTreeMap<String, String>,
}

pub fn compute_abi_hash(input: &AbiHashInput) -> String {
  hash_serialized(input)
}

pub fn hash_recipe_contents(contents: &str) -> String {
  hash_bytes(contents.as_bytes())
}

fn hash_serialized<T: Serialize>(value: &T) -> String {
  // `AbiHashInput` is fully JSON-serializable; this stays infallible for current inputs.
  // TODO(phase7): Return a `Result` from ABI hashing if/when non-JSON-serializable inputs are
  // introduced.
  let bytes = serde_json::to_vec(value).expect("ABI hash input serialization should not fail");
  hash_bytes(&bytes)
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

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use super::{AbiBuildProfile, AbiHashInput, AbiLinkage, compute_abi_hash, hash_recipe_contents};

  #[test]
  fn abi_hash_is_stable_for_identical_inputs() {
    let input = sample_input();
    let a = compute_abi_hash(&input);
    let b = compute_abi_hash(&input);
    assert_eq!(a, b);
    assert_eq!(a.len(), 64);
  }

  #[test]
  fn abi_hash_changes_when_compiler_version_changes() {
    let mut a = sample_input();
    let mut b = sample_input();
    b.compiler_version = "17.0.0".into();

    assert_ne!(compute_abi_hash(&a), compute_abi_hash(&b));

    a.profile = AbiBuildProfile::Release;
    assert_ne!(compute_abi_hash(&a), compute_abi_hash(&b));
  }

  #[test]
  fn recipe_content_hash_is_sha256_hex() {
    let hash = hash_recipe_contents("id = \"fmtlib/fmt\"\n");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|ch| ch.is_ascii_hexdigit()));
  }

  fn sample_input() -> AbiHashInput {
    let mut env = BTreeMap::new();
    env.insert("CXXFLAGS".into(), "-Wall".into());
    env.insert("LDFLAGS".into(), "-pthread".into());

    AbiHashInput {
      package_id: "fmtlib/fmt".into(),
      resolved_commit: "0123456789abcdef0123456789abcdef01234567".into(),
      recipe_content_hash: hash_recipe_contents("fmt recipe"),
      compiler_kind: "clang".into(),
      compiler_version: "16.0.0".into(),
      target_triple: "x86_64-unknown-linux-gnu".into(),
      host_os: "linux".into(),
      host_arch: "x86_64".into(),
      profile: AbiBuildProfile::Debug,
      cpp_standard: "c++20".into(),
      linkage: AbiLinkage::Static,
      cxxflags: vec!["-O0".into(), "-g".into()],
      ldflags: vec!["-pthread".into()],
      recipe_configure_args: vec!["-DBUILD_SHARED_LIBS=OFF".into()],
      env,
    }
  }
}
