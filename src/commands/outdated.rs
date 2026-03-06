use semver::Version;
use serde_json::json;
use std::collections::BTreeMap;
use std::env;

use super::dependency_common::map_registry_error;
use super::graph_common::validate_locked_graph_lockfile;
use crate::cli::{
  OutdatedArgs, OutdatedSourceArg, OutdatedSourceOverride, RuntimeFlags,
  take_outdated_source_override,
};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::lockfile;
use crate::manifest::Manifest;
use crate::output::HumanMessageBuilder;
use crate::package_id::PackageId;
use crate::registry::{RegistryRequirement, RegistryStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutdatedSources {
  All,
  Registry,
  Github,
  Git,
  Path,
  Archive,
}

impl From<OutdatedSourceArg> for OutdatedSources {
  fn from(value: OutdatedSourceArg) -> Self {
    match value {
      OutdatedSourceArg::All => Self::All,
      OutdatedSourceArg::Registry => Self::Registry,
      OutdatedSourceArg::Github => Self::Github,
    }
  }
}

impl From<OutdatedSourceOverride> for OutdatedSources {
  fn from(value: OutdatedSourceOverride) -> Self {
    match value {
      OutdatedSourceOverride::Git => Self::Git,
      OutdatedSourceOverride::Path => Self::Path,
      OutdatedSourceOverride::Archive => Self::Archive,
    }
  }
}

impl OutdatedSources {
  fn includes(self, source: &str) -> bool {
    match self {
      Self::All => true,
      Self::Registry => source == "registry",
      Self::Github => source == "github",
      Self::Git => source == "git",
      Self::Path => source == "path",
      Self::Archive => source == "archive",
    }
  }

  fn as_str(self) -> &'static str {
    match self {
      Self::All => "all",
      Self::Registry => "registry",
      Self::Github => "github",
      Self::Git => "git",
      Self::Path => "path",
      Self::Archive => "archive",
    }
  }
}

pub fn handle(args: OutdatedArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });

  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("outdated", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "outdated",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }
  let manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("outdated", "manifest_parse_error", err.to_string(), 1))?;

  let lockfile_path = cwd.join("joy.lock");
  let lock = validate_locked_graph_lockfile("outdated", &manifest, &manifest_path, &lockfile_path)?;

  let mut roots = manifest
    .dependencies
    .iter()
    .map(|(key, spec)| spec.declared_package(key).to_string())
    .collect::<Vec<_>>();
  roots.sort();

  let sources =
    take_outdated_source_override().map_or_else(|| OutdatedSources::from(args.sources), Into::into);
  let mut registry_stores = BTreeMap::<String, RegistryStore>::new();

  let mut rows = lock
    .packages
    .iter()
    .filter(|pkg| sources.includes(pkg.source.as_str()))
    .map(|pkg| {
      let direct = manifest.resolve_dependency_key(&pkg.id).is_some();
      compute_outdated_row(pkg, direct, &cwd, &mut registry_stores)
    })
    .collect::<Result<Vec<_>, JoyError>>()?;
  rows.sort_by(|a, b| a.id.cmp(&b.id));

  let outdated_rows =
    rows.iter().filter(|row| row.newer_compatible || row.newer_available).collect::<Vec<_>>();
  let registry_rows = rows.iter().filter(|row| row.source == "registry").count();
  let github_rows = rows.iter().filter(|row| row.source == "github").count();
  let git_rows = rows.iter().filter(|row| row.source == "git").count();
  let path_rows = rows.iter().filter(|row| row.source == "path").count();
  let archive_rows = rows.iter().filter(|row| row.source == "archive").count();
  let unsupported_rows = rows.iter().filter(|row| row.status == "unknown_source").count();
  let direct_count = rows.iter().filter(|row| row.direct).count();
  let transitive_count = rows.len().saturating_sub(direct_count);

  let human = render_human_outdated(&rows, outdated_rows.len(), sources);

  Ok(CommandOutput::new(
    "outdated",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "lockfile_path": lockfile_path.display().to_string(),
      "roots": roots,
      "sources": sources.as_str(),
      "summary": {
        "package_count": rows.len(),
        "direct_count": direct_count,
        "transitive_count": transitive_count,
        "registry_backed_count": registry_rows,
        "github_backed_count": github_rows,
        "git_backed_count": git_rows,
        "path_backed_count": path_rows,
        "archive_backed_count": archive_rows,
        "outdated_count": outdated_rows.len(),
        "unsupported_count": unsupported_rows,
      },
      "packages": rows.iter().map(OutdatedRow::json).collect::<Vec<_>>(),
      "outdated": outdated_rows.iter().map(|row| row.json()).collect::<Vec<_>>(),
    }),
  ))
}

#[derive(Debug, Clone)]
struct OutdatedRow {
  id: String,
  direct: bool,
  source: String,
  registry: Option<String>,
  source_package: Option<String>,
  requested_requirement: Option<String>,
  resolved_version: Option<String>,
  latest_compatible: Option<String>,
  latest_available: Option<String>,
  newer_compatible: bool,
  newer_available: bool,
  status: String,
  check_method: String,
  note: Option<String>,
}

impl OutdatedRow {
  fn json(&self) -> serde_json::Value {
    json!({
      "id": self.id,
      "direct": self.direct,
      "source": self.source,
      "registry": self.registry,
      "source_package": self.source_package,
      "requested_requirement": self.requested_requirement,
      "resolved_version": self.resolved_version,
      "latest_compatible": self.latest_compatible,
      "latest_available": self.latest_available,
      "newer_compatible": self.newer_compatible,
      "newer_available": self.newer_available,
      "status": self.status,
      "check_method": self.check_method,
      "note": self.note,
    })
  }
}

fn compute_outdated_row(
  pkg: &lockfile::LockedPackage,
  direct: bool,
  project_root: &std::path::Path,
  registry_stores: &mut BTreeMap<String, RegistryStore>,
) -> Result<OutdatedRow, JoyError> {
  match pkg.source.as_str() {
    "registry" => compute_registry_outdated_row(pkg, direct, project_root, registry_stores),
    "github" => Ok(compute_github_outdated_row(pkg, direct)),
    "git" => Ok(compute_git_outdated_row(pkg, direct)),
    "path" => Ok(compute_path_outdated_row(pkg, direct)),
    "archive" => Ok(compute_archive_outdated_row(pkg, direct)),
    _ => Ok(
      base_row(pkg, direct)
        .status("unknown_source", "unknown")
        .note(format!("Unknown dependency source `{}` for `joy outdated`", pkg.source)),
    ),
  }
}

#[derive(Debug, Clone)]
struct OutdatedRowBuilder(OutdatedRow);

impl OutdatedRowBuilder {
  fn status(mut self, status: &str, check_method: &str) -> Self {
    self.0.status = status.to_string();
    self.0.check_method = check_method.to_string();
    self
  }

  fn latest_available(mut self, value: Option<String>) -> Self {
    self.0.latest_available = value;
    self
  }

  fn outdated(mut self, outdated: bool) -> Self {
    self.0.newer_available = outdated;
    self
  }

  fn note(mut self, note: impl Into<String>) -> OutdatedRow {
    self.0.note = Some(note.into());
    self.0
  }

  fn build(self) -> OutdatedRow {
    self.0
  }
}

fn base_row(pkg: &lockfile::LockedPackage, direct: bool) -> OutdatedRowBuilder {
  OutdatedRowBuilder(OutdatedRow {
    id: pkg.id.clone(),
    direct,
    source: pkg.source.clone(),
    registry: pkg.registry.clone(),
    source_package: pkg.source_package.clone(),
    requested_requirement: pkg.requested_requirement.clone(),
    resolved_version: pkg.resolved_version.clone(),
    latest_compatible: None,
    latest_available: None,
    newer_compatible: false,
    newer_available: false,
    status: "unknown".into(),
    check_method: "unknown".into(),
    note: None,
  })
}

fn parse_or_synthetic_package_id(raw: &str) -> Result<PackageId, JoyError> {
  if let Ok(package) = PackageId::parse(raw) {
    return Ok(package);
  }
  let mut slug = String::new();
  for ch in raw.chars() {
    if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
      slug.push(ch);
    } else {
      slug.push('_');
    }
  }
  let slug = slug.trim_matches('_');
  let slug = if slug.is_empty() { "dep" } else { slug };
  PackageId::parse(&format!("local/{slug}"))
    .map_err(|err| JoyError::new("outdated", "invalid_package_id", err.to_string(), 1))
}

fn compute_git_outdated_row(pkg: &lockfile::LockedPackage, direct: bool) -> OutdatedRow {
  let package = match parse_or_synthetic_package_id(&pkg.id) {
    Ok(package) => package,
    Err(err) => {
      return base_row(pkg, direct).status("invalid_package_id", "git_rev").note(err.message);
    },
  };
  let Some(source_git) = pkg.source_git.as_deref() else {
    return base_row(pkg, direct)
      .status("invalid_source_metadata", "git_rev")
      .note("missing `source_git` metadata in lockfile");
  };
  let requested_rev = if pkg.requested_rev.trim().is_empty() { "HEAD" } else { &pkg.requested_rev };
  match fetch::fetch_git(&package, source_git, requested_rev) {
    Ok(fetched) => {
      let outdated = fetched.resolved_commit != pkg.resolved_commit;
      base_row(pkg, direct)
        .status(if outdated { "outdated_commit" } else { "up_to_date" }, "git_rev")
        .latest_available(Some(fetched.resolved_commit.clone()))
        .outdated(outdated)
        .note(format!(
          "requested rev `{requested_rev}` currently resolves to `{}`",
          fetched.resolved_commit
        ))
    },
    Err(err) if err.is_offline_cache_miss() || err.is_offline_network_disabled() => {
      base_row(pkg, direct).status("git_lookup_unavailable", "git_rev").note(err.to_string())
    },
    Err(err) => base_row(pkg, direct).status("git_lookup_failed", "git_rev").note(err.to_string()),
  }
}

fn compute_path_outdated_row(pkg: &lockfile::LockedPackage, direct: bool) -> OutdatedRow {
  let package = match parse_or_synthetic_package_id(&pkg.id) {
    Ok(package) => package,
    Err(err) => {
      return base_row(pkg, direct).status("invalid_package_id", "path_hash").note(err.message);
    },
  };
  let Some(source_path) = pkg.source_path.as_deref() else {
    return base_row(pkg, direct)
      .status("invalid_source_metadata", "path_hash")
      .note("missing `source_path` metadata in lockfile");
  };
  match fetch::fetch_path(&package, source_path) {
    Ok(fetched) => {
      let outdated = fetched.resolved_commit != pkg.resolved_commit;
      base_row(pkg, direct)
        .status(if outdated { "outdated_path_content" } else { "up_to_date" }, "path_hash")
        .latest_available(Some(fetched.resolved_commit))
        .outdated(outdated)
        .build()
    },
    Err(err) if err.is_offline_cache_miss() || err.is_offline_network_disabled() => {
      base_row(pkg, direct).status("path_lookup_unavailable", "path_hash").note(err.to_string())
    },
    Err(err) => {
      base_row(pkg, direct).status("path_lookup_failed", "path_hash").note(err.to_string())
    },
  }
}

fn compute_archive_outdated_row(pkg: &lockfile::LockedPackage, direct: bool) -> OutdatedRow {
  let package = match parse_or_synthetic_package_id(&pkg.id) {
    Ok(package) => package,
    Err(err) => {
      return base_row(pkg, direct)
        .status("invalid_package_id", "archive_checksum")
        .note(err.message);
    },
  };
  let Some(source_url) = pkg.source_url.as_deref() else {
    return base_row(pkg, direct)
      .status("invalid_source_metadata", "archive_checksum")
      .note("missing `source_url` metadata in lockfile");
  };
  let Some(sha256) = pkg.source_checksum_sha256.as_deref() else {
    return base_row(pkg, direct)
      .status("invalid_source_metadata", "archive_checksum")
      .note("missing `source_checksum_sha256` metadata in lockfile");
  };
  match fetch::fetch_archive(&package, source_url, sha256) {
    Ok(fetched) => {
      let outdated = fetched.resolved_commit != pkg.resolved_commit;
      base_row(pkg, direct)
        .status(
          if outdated { "outdated_archive_content" } else { "up_to_date" },
          "archive_checksum",
        )
        .latest_available(Some(fetched.resolved_commit))
        .outdated(outdated)
        .build()
    },
    Err(err) if err.is_checksum_mismatch() => base_row(pkg, direct)
      .status("archive_checksum_mismatch", "archive_checksum")
      .note(err.to_string()),
    Err(err) if err.is_invalid_checksum() => {
      base_row(pkg, direct).status("invalid_checksum", "archive_checksum").note(err.to_string())
    },
    Err(err) if err.is_unsupported_archive_format() => base_row(pkg, direct)
      .status("archive_format_unsupported", "archive_checksum")
      .note(err.to_string()),
    Err(err) if err.is_offline_cache_miss() || err.is_offline_network_disabled() => {
      base_row(pkg, direct)
        .status("archive_lookup_unavailable", "archive_checksum")
        .note(err.to_string())
    },
    Err(err) => base_row(pkg, direct)
      .status("archive_lookup_failed", "archive_checksum")
      .note(err.to_string()),
  }
}

fn compute_registry_outdated_row(
  pkg: &lockfile::LockedPackage,
  direct: bool,
  project_root: &std::path::Path,
  registry_stores: &mut BTreeMap<String, RegistryStore>,
) -> Result<OutdatedRow, JoyError> {
  let current_version = match pkg.resolved_version.as_deref() {
    Some(v) if !v.trim().is_empty() => Some(Version::parse(v).map_err(|err| {
      JoyError::new(
        "outdated",
        "lockfile_incomplete",
        format!("invalid `resolved_version` `{v}` in joy.lock for `{}`: {err}", pkg.id),
        1,
      )
    })?),
    _ => None,
  };

  let registry_name = pkg.registry.as_deref().unwrap_or("default").to_string();
  let store = if let Some(store) = registry_stores.get(&registry_name) {
    store.clone()
  } else {
    let loaded = RegistryStore::load_named_for_project(&registry_name, Some(project_root))
      .map_err(|err| map_registry_error("outdated", err))?;
    registry_stores.insert(registry_name.clone(), loaded.clone());
    loaded
  };

  let latest_available_release = store
    .resolve(&pkg.id, RegistryRequirement::Semver("*"))
    .map_err(|err| map_registry_error("outdated", err))?;
  let latest_available = latest_available_release.resolved_version.clone();
  let latest_available_parsed = Version::parse(&latest_available).map_err(|err| {
    JoyError::new(
      "outdated",
      "registry_load_failed",
      format!("registry returned invalid version `{}` for `{}`: {err}", latest_available, pkg.id),
      1,
    )
  })?;

  let latest_compatible = if let Some(req) = pkg.requested_requirement.as_deref() {
    Some(
      store
        .resolve(&pkg.id, RegistryRequirement::Semver(req))
        .map_err(|err| map_registry_error("outdated", err))?
        .resolved_version,
    )
  } else {
    None
  };
  let latest_compatible_parsed =
    latest_compatible.as_deref().map(Version::parse).transpose().map_err(|err| {
      JoyError::new(
        "outdated",
        "registry_load_failed",
        format!("registry returned invalid compatible version for `{}`: {err}", pkg.id),
        1,
      )
    })?;

  let (newer_available, newer_compatible, status, note) = match current_version {
    Some(ref current) => {
      let newer_available = latest_available_parsed > *current;
      let newer_compatible = latest_compatible_parsed.as_ref().is_some_and(|v| *v > *current);
      let status = if newer_compatible {
        "outdated_compatible"
      } else if newer_available {
        if pkg.requested_requirement.is_some() {
          "newer_available_outside_requirement"
        } else {
          "pinned_behind_latest"
        }
      } else {
        "up_to_date"
      };
      let note = if status == "newer_available_outside_requirement" {
        Some("A newer version exists, but it is outside the current version requirement".into())
      } else {
        None
      };
      (newer_available, newer_compatible, status.to_string(), note)
    },
    None => (
      false,
      false,
      "unknown".into(),
      Some("Missing `resolved_version` in lockfile for registry package".into()),
    ),
  };

  Ok(OutdatedRow {
    id: pkg.id.clone(),
    direct,
    source: pkg.source.clone(),
    registry: pkg.registry.clone(),
    source_package: pkg.source_package.clone(),
    requested_requirement: pkg.requested_requirement.clone(),
    resolved_version: pkg.resolved_version.clone(),
    latest_compatible,
    latest_available: Some(latest_available),
    newer_compatible,
    newer_available,
    status,
    check_method: "registry_index".into(),
    note,
  })
}

fn compute_github_outdated_row(pkg: &lockfile::LockedPackage, direct: bool) -> OutdatedRow {
  let package = match PackageId::parse(&pkg.id) {
    Ok(package) => package,
    Err(err) => {
      return OutdatedRow {
        id: pkg.id.clone(),
        direct,
        source: pkg.source.clone(),
        registry: pkg.registry.clone(),
        source_package: pkg.source_package.clone(),
        requested_requirement: pkg.requested_requirement.clone(),
        resolved_version: pkg.resolved_version.clone(),
        latest_compatible: None,
        latest_available: None,
        newer_compatible: false,
        newer_available: false,
        status: "unknown_non_semver".into(),
        check_method: "unknown".into(),
        note: Some(format!(
          "invalid package id `{}` in lockfile prevents semver tag checks: {err}",
          pkg.id
        )),
      };
    },
  };

  let current_version = pkg.resolved_version.as_deref().and_then(|v| Version::parse(v).ok());

  let latest_available_outcome = resolve_github_semver(&package, "*");
  let latest_compatible_outcome =
    pkg.requested_requirement.as_deref().map(|req| resolve_github_semver(&package, req));

  let latest_available = latest_available_outcome.version();
  let latest_compatible = latest_compatible_outcome.as_ref().and_then(|outcome| outcome.version());

  let latest_available_parsed = latest_available.as_deref().and_then(|v| Version::parse(v).ok());
  let latest_compatible_parsed = latest_compatible.as_deref().and_then(|v| Version::parse(v).ok());

  let check_method = if latest_available.is_some() || latest_compatible.is_some() {
    "github_tags"
  } else {
    "unknown"
  }
  .to_string();

  let note = compose_github_note(
    pkg,
    &latest_available_outcome,
    latest_compatible_outcome.as_ref(),
    current_version.is_some(),
  );

  let (newer_available, newer_compatible, status) = if let (Some(current), Some(latest)) =
    (current_version.as_ref(), latest_available_parsed.as_ref())
  {
    let newer_available = latest > current;
    let newer_compatible = latest_compatible_parsed.as_ref().is_some_and(|v| v > current);
    let status = if newer_compatible {
      "outdated_compatible"
    } else if newer_available {
      if pkg.requested_requirement.is_some() {
        "newer_available_outside_requirement"
      } else {
        "pinned_behind_latest"
      }
    } else {
      "up_to_date"
    };
    (newer_available, newer_compatible, status.to_string())
  } else {
    let status = match latest_available_outcome {
      GithubSemverOutcome::NoSemverTags => "github_non_semver_tags",
      GithubSemverOutcome::OfflineUnavailable => "github_lookup_unavailable",
      GithubSemverOutcome::InvalidRequirement => "invalid_version_requirement",
      GithubSemverOutcome::LookupFailed => "github_lookup_failed",
      GithubSemverOutcome::Resolved(_) => "unknown_non_semver",
    };
    (false, false, status.to_string())
  };

  OutdatedRow {
    id: pkg.id.clone(),
    direct,
    source: pkg.source.clone(),
    registry: pkg.registry.clone(),
    source_package: pkg.source_package.clone(),
    requested_requirement: pkg.requested_requirement.clone(),
    resolved_version: pkg.resolved_version.clone(),
    latest_compatible,
    latest_available,
    newer_compatible,
    newer_available,
    status,
    check_method,
    note,
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GithubSemverOutcome {
  Resolved(String),
  NoSemverTags,
  OfflineUnavailable,
  InvalidRequirement,
  LookupFailed,
}

impl GithubSemverOutcome {
  fn version(&self) -> Option<String> {
    match self {
      Self::Resolved(version) => Some(version.clone()),
      Self::NoSemverTags
      | Self::OfflineUnavailable
      | Self::InvalidRequirement
      | Self::LookupFailed => None,
    }
  }
}

fn resolve_github_semver(package: &PackageId, requirement: &str) -> GithubSemverOutcome {
  match fetch::fetch_github_semver(package, requirement) {
    Ok(fetched) => {
      let Some(version) = fetched.resolved_version else {
        return GithubSemverOutcome::LookupFailed;
      };
      GithubSemverOutcome::Resolved(version)
    },
    Err(err) if err.is_version_not_found() => GithubSemverOutcome::NoSemverTags,
    Err(err) if err.is_offline_cache_miss() || err.is_offline_network_disabled() => {
      GithubSemverOutcome::OfflineUnavailable
    },
    Err(err) if err.is_invalid_version_requirement() => GithubSemverOutcome::InvalidRequirement,
    Err(_) => GithubSemverOutcome::LookupFailed,
  }
}

fn compose_github_note(
  pkg: &lockfile::LockedPackage,
  latest_available: &GithubSemverOutcome,
  latest_compatible: Option<&GithubSemverOutcome>,
  has_semver_current: bool,
) -> Option<String> {
  if !has_semver_current {
    return Some(
      "Current lockfile entry does not contain a semver-compatible `resolved_version`; using best-effort tag lookup"
        .into(),
    );
  }

  if matches!(latest_available, GithubSemverOutcome::NoSemverTags) {
    return Some("Repository tags are not semver-compatible; unable to compute updates".into());
  }

  if matches!(latest_available, GithubSemverOutcome::OfflineUnavailable) {
    return Some(
      "Offline mode prevented refreshing semver tag metadata for this GitHub package".into(),
    );
  }

  if matches!(latest_available, GithubSemverOutcome::LookupFailed) {
    return Some("Failed to read GitHub tag metadata for this package".into());
  }

  if matches!(latest_compatible, Some(GithubSemverOutcome::InvalidRequirement)) {
    return Some(format!(
      "Invalid version requirement `{}` recorded in lockfile for this package",
      pkg.requested_requirement.as_deref().unwrap_or_default()
    ));
  }

  if matches!(latest_compatible, Some(GithubSemverOutcome::NoSemverTags)) {
    return Some(
      "No semver tags satisfy the current version requirement for this GitHub package".into(),
    );
  }

  None
}

fn render_human_outdated(
  rows: &[OutdatedRow],
  outdated_count: usize,
  sources: OutdatedSources,
) -> String {
  if rows.is_empty() {
    return HumanMessageBuilder::new("No dependencies matched the requested source filter")
      .kv("sources", sources.as_str())
      .hint("Add one with `joy add <owner/repo>` or rerun with `joy outdated --sources all`")
      .build();
  }

  let mut builder = if outdated_count == 0 {
    HumanMessageBuilder::new("No outdated dependencies found")
  } else {
    HumanMessageBuilder::new(format!("Found {outdated_count} outdated dependencies"))
  }
  .kv("package count", rows.len().to_string())
  .kv("sources", sources.as_str());

  for row in rows {
    if !(row.newer_compatible || row.newer_available) {
      continue;
    }
    let scope = if row.direct { "direct" } else { "transitive" };
    let current = row.resolved_version.as_deref().unwrap_or("<unknown>");
    let compat = row.latest_compatible.as_deref().unwrap_or("-");
    let latest = row.latest_available.as_deref().unwrap_or("-");
    builder = builder.line(format!(
      "- {} ({scope}, {}, {}): current {current}, compatible {compat}, latest {latest} [{}]",
      row.id, row.source, row.check_method, row.status
    ));
  }

  builder.build()
}
