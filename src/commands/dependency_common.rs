use crate::error::JoyError;
use crate::fetch;
use crate::manifest::DependencySource;
use crate::registry::RegistryError;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct ParsedDependencyInput {
  pub source: DependencySource,
  pub reference: String,
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
      source: DependencySource::Registry,
      reference: id.to_string(),
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
      source: DependencySource::Github,
      reference: id.to_string(),
    });
  }
  if let Some(url) = raw.strip_prefix("git+") {
    if url.trim().is_empty() {
      return Err(JoyError::new(
        command,
        "invalid_dependency_source",
        "invalid dependency `git+`; expected `git+<url-or-path>`",
        1,
      ));
    }
    return Ok(ParsedDependencyInput { source: DependencySource::Git, reference: url.to_string() });
  }
  if let Some(url) = raw.strip_prefix("git:") {
    if url.trim().is_empty() {
      return Err(JoyError::new(
        command,
        "invalid_dependency_source",
        "invalid dependency `git:`; expected `git:<url-or-path>`",
        1,
      ));
    }
    return Ok(ParsedDependencyInput { source: DependencySource::Git, reference: url.to_string() });
  }
  if let Some(path) = raw.strip_prefix("path:") {
    if path.trim().is_empty() {
      return Err(JoyError::new(
        command,
        "invalid_dependency_source",
        "invalid dependency `path:`; expected `path:<relative-or-absolute-path>`",
        1,
      ));
    }
    return Ok(ParsedDependencyInput {
      source: DependencySource::Path,
      reference: path.to_string(),
    });
  }
  if let Some(url) = raw.strip_prefix("archive:") {
    if url.trim().is_empty() {
      return Err(JoyError::new(
        command,
        "invalid_dependency_source",
        "invalid dependency `archive:`; expected `archive:<url>`",
        1,
      ));
    }
    return Ok(ParsedDependencyInput {
      source: DependencySource::Archive,
      reference: url.to_string(),
    });
  }
  Ok(ParsedDependencyInput { source: DependencySource::Github, reference: raw.to_string() })
}

pub(crate) fn normalize_dependency_arg(raw: &str) -> String {
  raw
    .strip_prefix("registry:")
    .or_else(|| raw.strip_prefix("github:"))
    .or_else(|| raw.strip_prefix("git+"))
    .or_else(|| raw.strip_prefix("git:"))
    .or_else(|| raw.strip_prefix("path:"))
    .or_else(|| raw.strip_prefix("archive:"))
    .unwrap_or(raw)
    .to_string()
}

pub(crate) fn infer_dependency_key(source: &DependencySource, reference: &str) -> Option<String> {
  let raw = match source {
    DependencySource::Github | DependencySource::Registry => reference.split('/').next_back(),
    DependencySource::Git => {
      let tail = reference.trim_end_matches('/').split('/').next_back()?;
      Some(tail.trim_end_matches(".git"))
    },
    DependencySource::Path => Path::new(reference)
      .file_name()
      .and_then(|name| name.to_str())
      .filter(|name| !name.trim().is_empty()),
    DependencySource::Archive => {
      let tail = reference.trim_end_matches('/').split('/').next_back()?;
      let without_query = tail.split('?').next().unwrap_or(tail);
      let stem =
        without_query.trim_end_matches(".tar.gz").trim_end_matches(".tgz").trim_end_matches(".zip");
      Some(stem)
    },
  }?;
  let sanitized = raw
    .chars()
    .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') { ch } else { '_' })
    .collect::<String>()
    .trim_matches('_')
    .to_string();
  (!sanitized.is_empty()).then_some(sanitized)
}

pub(crate) fn map_fetch_error(command: &'static str, err: fetch::FetchError) -> JoyError {
  let code = if err.is_offline_cache_miss() {
    "offline_cache_miss"
  } else if err.is_offline_network_disabled() {
    "offline_network_disabled"
  } else if err.is_invalid_version_requirement() {
    "invalid_version_requirement"
  } else if err.is_invalid_checksum() {
    "invalid_checksum"
  } else if err.is_checksum_mismatch() {
    "checksum_mismatch"
  } else if err.is_unsupported_archive_format() {
    "archive_format_unsupported"
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
  use super::{infer_dependency_key, normalize_dependency_arg, parse_dependency_input};
  use crate::manifest::DependencySource;

  #[test]
  fn parse_dependency_defaults_to_github() {
    let parsed = parse_dependency_input("add", "fmtlib/fmt").expect("parse");
    assert_eq!(parsed.reference, "fmtlib/fmt");
    assert_eq!(parsed.source, DependencySource::Github);
  }

  #[test]
  fn parse_dependency_supports_registry_prefix() {
    let parsed = parse_dependency_input("add", "registry:fmtlib/fmt").expect("parse");
    assert_eq!(parsed.reference, "fmtlib/fmt");
    assert_eq!(parsed.source, DependencySource::Registry);
  }

  #[test]
  fn parse_dependency_supports_git_path_archive_prefixes() {
    let git = parse_dependency_input("add", "git+https://example.com/acme/lib.git").expect("git");
    assert_eq!(git.source, DependencySource::Git);
    assert_eq!(git.reference, "https://example.com/acme/lib.git");
    let path = parse_dependency_input("add", "path:../vendor/lib").expect("path");
    assert_eq!(path.source, DependencySource::Path);
    assert_eq!(path.reference, "../vendor/lib");
    let archive =
      parse_dependency_input("add", "archive:https://example.com/lib.tar.gz").expect("archive");
    assert_eq!(archive.source, DependencySource::Archive);
    assert_eq!(archive.reference, "https://example.com/lib.tar.gz");
  }

  #[test]
  fn normalize_dependency_strips_prefixes() {
    assert_eq!(normalize_dependency_arg("registry:fmtlib/fmt"), "fmtlib/fmt");
    assert_eq!(normalize_dependency_arg("github:fmtlib/fmt"), "fmtlib/fmt");
    assert_eq!(
      normalize_dependency_arg("git+https://example.com/acme/lib.git"),
      "https://example.com/acme/lib.git"
    );
    assert_eq!(normalize_dependency_arg("path:../vendor/lib"), "../vendor/lib");
    assert_eq!(
      normalize_dependency_arg("archive:https://example.com/lib.tar.gz"),
      "https://example.com/lib.tar.gz"
    );
    assert_eq!(normalize_dependency_arg("fmtlib/fmt"), "fmtlib/fmt");
  }

  #[test]
  fn infers_dependency_keys_for_supported_sources() {
    assert_eq!(
      infer_dependency_key(&DependencySource::Github, "fmtlib/fmt").as_deref(),
      Some("fmt")
    );
    assert_eq!(
      infer_dependency_key(&DependencySource::Git, "https://example.com/acme/lib.git").as_deref(),
      Some("lib")
    );
    assert_eq!(
      infer_dependency_key(&DependencySource::Path, "../vendor/local-lib").as_deref(),
      Some("local-lib")
    );
    assert_eq!(
      infer_dependency_key(&DependencySource::Archive, "https://example.com/lib-1.0.0.tar.gz")
        .as_deref(),
      Some("lib-1.0.0")
    );
  }
}
