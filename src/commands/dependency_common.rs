use crate::error::JoyError;
use crate::fetch;
use crate::manifest::DependencySource;
use crate::registry::RegistryError;

#[derive(Debug, Clone)]
pub(crate) struct ParsedDependencyInput {
  pub package_id: String,
  pub source: DependencySource,
}

pub(crate) fn parse_dependency_input(
  command: &'static str,
  raw: &str,
) -> Result<ParsedDependencyInput, JoyError> {
  if let Some(id) = raw.strip_prefix("registry:") {
    if id.trim().is_empty() {
      return Err(JoyError::new(
        command,
        "invalid_package_id",
        "invalid package `registry:`; expected `registry:owner/repo`",
        1,
      ));
    }
    return Ok(ParsedDependencyInput {
      package_id: id.to_string(),
      source: DependencySource::Registry,
    });
  }
  if let Some(id) = raw.strip_prefix("github:") {
    if id.trim().is_empty() {
      return Err(JoyError::new(
        command,
        "invalid_package_id",
        "invalid package `github:`; expected `github:owner/repo`",
        1,
      ));
    }
    return Ok(ParsedDependencyInput {
      package_id: id.to_string(),
      source: DependencySource::Github,
    });
  }
  Ok(ParsedDependencyInput { package_id: raw.to_string(), source: DependencySource::Github })
}

pub(crate) fn normalize_dependency_arg(raw: &str) -> String {
  raw.strip_prefix("registry:").or_else(|| raw.strip_prefix("github:")).unwrap_or(raw).to_string()
}

pub(crate) fn map_fetch_error(command: &'static str, err: fetch::FetchError) -> JoyError {
  let code = if err.is_offline_cache_miss() {
    "offline_cache_miss"
  } else if err.is_offline_network_disabled() {
    "offline_network_disabled"
  } else if err.is_invalid_version_requirement() {
    "invalid_version_requirement"
  } else if err.is_version_not_found() {
    "version_not_found"
  } else {
    "fetch_failed"
  };
  JoyError::new(command, code, err.to_string(), 1)
}

pub(crate) fn map_registry_error(command: &'static str, err: RegistryError) -> JoyError {
  let code = if err.is_offline_cache_miss() {
    "offline_cache_miss"
  } else if err.is_not_configured() {
    "registry_not_configured"
  } else if err.is_package_not_found() {
    "registry_package_not_found"
  } else if err.is_invalid_version_requirement() {
    "invalid_version_requirement"
  } else if err.is_version_not_found() {
    "version_not_found"
  } else {
    "registry_load_failed"
  };
  JoyError::new(command, code, err.to_string(), 1)
}

#[cfg(test)]
mod tests {
  use super::{normalize_dependency_arg, parse_dependency_input};
  use crate::manifest::DependencySource;

  #[test]
  fn parse_dependency_defaults_to_github() {
    let parsed = parse_dependency_input("add", "fmtlib/fmt").expect("parse");
    assert_eq!(parsed.package_id, "fmtlib/fmt");
    assert_eq!(parsed.source, DependencySource::Github);
  }

  #[test]
  fn parse_dependency_supports_registry_prefix() {
    let parsed = parse_dependency_input("add", "registry:fmtlib/fmt").expect("parse");
    assert_eq!(parsed.package_id, "fmtlib/fmt");
    assert_eq!(parsed.source, DependencySource::Registry);
  }

  #[test]
  fn normalize_dependency_strips_prefixes() {
    assert_eq!(normalize_dependency_arg("registry:fmtlib/fmt"), "fmtlib/fmt");
    assert_eq!(normalize_dependency_arg("github:fmtlib/fmt"), "fmtlib/fmt");
    assert_eq!(normalize_dependency_arg("fmtlib/fmt"), "fmtlib/fmt");
  }
}
