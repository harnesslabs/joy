use std::fmt;
use thiserror::Error;

use crate::package_id::{PackageId, PackageIdError};

/// Generalized dependency coordinate as declared in `joy.toml`.
///
/// Unlike [`PackageId`], this coordinate is not restricted to `owner/repo`.
/// It supports additive source backends (git/path/archive) while preserving
/// compatibility helpers for legacy canonical package IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageCoordinate {
  raw: String,
}

impl PackageCoordinate {
  /// Parse a dependency coordinate.
  pub fn parse(raw: &str) -> Result<Self, PackageCoordinateError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      return Err(PackageCoordinateError::Invalid(raw.to_string()));
    }
    if trimmed.chars().any(|ch| ch.is_whitespace() || ch.is_control()) {
      return Err(PackageCoordinateError::Invalid(raw.to_string()));
    }
    Ok(Self { raw: trimmed.to_string() })
  }

  /// Return the original coordinate string.
  pub fn as_str(&self) -> &str {
    &self.raw
  }

  /// Parse the coordinate as a legacy canonical `owner/repo` package ID.
  pub fn as_legacy_package_id(&self) -> Result<PackageId, PackageIdError> {
    PackageId::parse(&self.raw)
  }
}

impl fmt::Display for PackageCoordinate {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.raw.fmt(f)
  }
}

/// Parsing/validation errors for [`PackageCoordinate`].
#[derive(Debug, Error)]
pub enum PackageCoordinateError {
  #[error("invalid dependency coordinate `{0}`")]
  Invalid(String),
}

#[cfg(test)]
mod tests {
  use super::PackageCoordinate;

  #[test]
  fn parses_generalized_coordinates() {
    for value in ["fmtlib/fmt", "json", "localdep", "vendor::archive_dep", "my-lib_v2"] {
      let parsed = PackageCoordinate::parse(value).expect("coordinate");
      assert_eq!(parsed.as_str(), value);
    }
  }

  #[test]
  fn rejects_empty_or_whitespace_coordinates() {
    for value in ["", "   ", "dep name", "dep\tname"] {
      PackageCoordinate::parse(value).expect_err("invalid coordinate");
    }
  }

  #[test]
  fn can_parse_legacy_package_id_when_coordinate_is_owner_repo() {
    let parsed = PackageCoordinate::parse("nlohmann/json").expect("coordinate");
    let package = parsed.as_legacy_package_id().expect("legacy package id");
    assert_eq!(package.as_str(), "nlohmann/json");
  }
}
