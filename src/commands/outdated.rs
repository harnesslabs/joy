use semver::Version;
use serde_json::json;
use std::env;

use crate::cli::{OutdatedArgs, RuntimeFlags};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::lockfile;
use crate::manifest::Manifest;
use crate::output::HumanMessageBuilder;
use crate::registry::{RegistryError, RegistryRequirement, RegistryStore};

pub fn handle(_args: OutdatedArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
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
  if !lockfile_path.is_file() {
    return Err(JoyError::new(
      "outdated",
      "lockfile_missing",
      format!(
        "`joy outdated` requires `{}`; create or refresh it with `joy sync --update-lock`",
        lockfile_path.display()
      ),
      1,
    ));
  }
  let lock = lockfile::Lockfile::load(&lockfile_path)
    .map_err(|err| JoyError::new("outdated", "lockfile_parse_error", err.to_string(), 1))?;
  let manifest_hash = lockfile::compute_manifest_hash(&manifest_path)
    .map_err(|err| JoyError::new("outdated", "lockfile_hash_failed", err.to_string(), 1))?;
  if lock.manifest_hash != manifest_hash {
    return Err(JoyError::new(
      "outdated",
      "lockfile_stale",
      "joy.lock manifest hash does not match joy.toml; rerun `joy sync --update-lock`".to_string(),
      1,
    ));
  }
  if !manifest.dependencies.is_empty() && lock.packages.is_empty() {
    return Err(JoyError::new(
      "outdated",
      "lockfile_incomplete",
      "joy.lock package metadata is missing for current dependencies; rerun `joy sync --update-lock`"
        .to_string(),
      1,
    ));
  }

  let mut roots = manifest.dependencies.keys().cloned().collect::<Vec<_>>();
  roots.sort();
  let mut registry_store = None::<RegistryStore>;

  let mut rows = lock
    .packages
    .iter()
    .map(|pkg| {
      let direct = manifest.dependencies.contains_key(&pkg.id);
      compute_outdated_row(pkg, direct, &mut registry_store)
    })
    .collect::<Result<Vec<_>, JoyError>>()?;
  rows.sort_by(|a, b| a.id.cmp(&b.id));

  let outdated_rows =
    rows.iter().filter(|row| row.newer_compatible || row.newer_available).collect::<Vec<_>>();
  let registry_rows = rows.iter().filter(|row| row.source == "registry").count();
  let unsupported_rows = rows
    .iter()
    .filter(|row| row.status == "unsupported_source" || row.status == "unsupported")
    .count();
  let direct_count = rows.iter().filter(|row| row.direct).count();
  let transitive_count = rows.len().saturating_sub(direct_count);

  let human = render_human_outdated(&rows, outdated_rows.len(), unsupported_rows);

  Ok(CommandOutput::new(
    "outdated",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "manifest_path": manifest_path.display().to_string(),
      "lockfile_path": lockfile_path.display().to_string(),
      "roots": roots,
      "summary": {
        "package_count": rows.len(),
        "direct_count": direct_count,
        "transitive_count": transitive_count,
        "registry_backed_count": registry_rows,
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
      "note": self.note,
    })
  }
}

fn compute_outdated_row(
  pkg: &lockfile::LockedPackage,
  direct: bool,
  registry_store: &mut Option<RegistryStore>,
) -> Result<OutdatedRow, JoyError> {
  if pkg.source != "registry" {
    return Ok(OutdatedRow {
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
      status: "unsupported_source".into(),
      note: Some("Only registry-backed version checks are supported by `joy outdated`".into()),
    });
  }

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

  let store = if let Some(store) = registry_store.as_ref() {
    store.clone()
  } else {
    let loaded = RegistryStore::load_default().map_err(map_registry_error)?;
    *registry_store = Some(loaded.clone());
    loaded
  };

  let latest_available_release =
    store.resolve(&pkg.id, RegistryRequirement::Semver("*")).map_err(map_registry_error)?;
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
        .map_err(map_registry_error)?
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
    note,
  })
}

fn render_human_outdated(
  rows: &[OutdatedRow],
  outdated_count: usize,
  unsupported_count: usize,
) -> String {
  if rows.is_empty() {
    return HumanMessageBuilder::new("No dependencies")
      .hint("Add one with `joy add <owner/repo>`")
      .build();
  }

  let mut builder = if outdated_count == 0 {
    HumanMessageBuilder::new("No outdated dependencies found")
  } else {
    HumanMessageBuilder::new(format!("Found {outdated_count} outdated dependencies"))
  }
  .kv("package count", rows.len().to_string())
  .kv("unsupported", unsupported_count.to_string());

  for row in rows {
    if !(row.newer_compatible || row.newer_available) {
      continue;
    }
    let scope = if row.direct { "direct" } else { "transitive" };
    let current = row.resolved_version.as_deref().unwrap_or("<unknown>");
    let compat = row.latest_compatible.as_deref().unwrap_or("-");
    let latest = row.latest_available.as_deref().unwrap_or("-");
    builder = builder.line(format!(
      "- {} ({scope}, {}): current {current}, compatible {compat}, latest {latest} [{}]",
      row.id, row.source, row.status
    ));
  }

  if outdated_count == 0 && unsupported_count > 0 {
    builder = builder.hint(
      "Only registry-backed packages are currently checked by `joy outdated`; GitHub packages are listed as unsupported in JSON output",
    );
  }

  builder.build()
}

fn map_registry_error(err: RegistryError) -> JoyError {
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
  JoyError::new("outdated", code, err.to_string(), 1)
}
