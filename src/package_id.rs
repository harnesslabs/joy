use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
  raw: String,
  owner: String,
  repo: String,
}

impl PackageId {
  pub fn parse(raw: &str) -> Result<Self, PackageIdError> {
    let mut parts = raw.split('/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();
    let extra = parts.next();

    let valid = extra.is_none()
      && !owner.is_empty()
      && !repo.is_empty()
      && owner.chars().all(is_valid_package_char)
      && repo.chars().all(is_valid_package_char);

    if !valid {
      return Err(PackageIdError::Invalid(raw.to_string()));
    }

    Ok(Self { raw: raw.to_string(), owner: owner.to_string(), repo: repo.to_string() })
  }

  pub fn as_str(&self) -> &str {
    &self.raw
  }

  pub fn owner(&self) -> &str {
    &self.owner
  }

  pub fn repo(&self) -> &str {
    &self.repo
  }

  pub fn slug(&self) -> String {
    format!("{}_{}", self.owner, self.repo)
  }
}

impl fmt::Display for PackageId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.raw.fmt(f)
  }
}

fn is_valid_package_char(ch: char) -> bool {
  ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')
}

#[derive(Debug, Error)]
pub enum PackageIdError {
  #[error("invalid package `{0}`; expected `owner/repo`")]
  Invalid(String),
}

#[cfg(test)]
mod tests {
  use super::PackageId;

  #[test]
  fn parses_valid_github_shorthand_package_ids() {
    for valid in ["nlohmann/json", "fmtlib/fmt", "owner/repo-name_1.2"] {
      let parsed = PackageId::parse(valid).expect("valid package");
      assert_eq!(parsed.as_str(), valid);
    }
  }

  #[test]
  fn rejects_invalid_package_ids() {
    for invalid in ["", "owner", "/repo", "owner/", "owner/repo/extra", "ow ner/repo"] {
      PackageId::parse(invalid).expect_err("invalid package");
    }
  }

  #[test]
  fn slug_uses_owner_and_repo() {
    let parsed = PackageId::parse("nlohmann/json").expect("package");
    assert_eq!(parsed.slug(), "nlohmann_json");
  }
}
