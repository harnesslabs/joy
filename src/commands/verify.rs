use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::{RuntimeFlags, VerifyArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::lockfile::{LockedPackage, Lockfile};
use crate::output::HumanMessageBuilder;
use crate::package_id::PackageId;

pub fn handle(args: VerifyArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("verify", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let project_root = runtime.workspace_root.clone().unwrap_or_else(|| cwd.clone());
  let lockfile_path = project_root.join("joy.lock");
  if !lockfile_path.is_file() {
    return Err(JoyError::new(
      "verify",
      "lockfile_not_found",
      format!("no `joy.lock` found at {}", lockfile_path.display()),
      1,
    ));
  }
  let lock = Lockfile::load(&lockfile_path)
    .map_err(|err| JoyError::new("verify", "lockfile_parse_error", err.to_string(), 1))?;

  let mut passed = 0usize;
  let mut warnings = 0usize;
  let mut failed = 0usize;
  let mut rows = Vec::new();
  let mut failed_summaries = Vec::new();
  let mut sbom_components = Vec::new();

  for pkg in &lock.packages {
    let mut issues = provenance_issues(pkg);
    let mut advisory = Vec::<String>::new();
    let expected_checksum = pkg.source_checksum_sha256.clone();

    let (source_dir, actual_checksum) = resolve_source_checksum(pkg, &project_root)?;

    if let Some(expected) = expected_checksum.as_deref() {
      if let Some(actual) = actual_checksum.as_deref() {
        if !actual.eq_ignore_ascii_case(expected) {
          issues.push(format!("source checksum mismatch: expected {expected}, actual {actual}"));
        }
      } else {
        issues.push(
          "lockfile checksum is present, but this source backend cannot be verified yet".into(),
        );
      }
    } else if args.strict {
      issues
        .push("missing lockfile checksum (strict mode requires `source_checksum_sha256`)".into());
    } else {
      advisory.push("checksum not pinned in lockfile".into());
    }

    let vendor_dir =
      project_root.join("vendor").join(vendor_slug(&pkg.id)).join(&pkg.resolved_commit);
    let vendor_checksum =
      if vendor_dir.exists() { Some(hash_path_sha256("verify", &vendor_dir)?) } else { None };
    if let Some(vendor_hash) = vendor_checksum.as_deref() {
      if let Some(actual) = actual_checksum.as_deref() {
        if !vendor_hash.eq_ignore_ascii_case(actual) {
          issues.push(format!(
            "vendored source checksum mismatch: expected {actual}, actual {vendor_hash}"
          ));
        }
      } else if let Some(expected) = expected_checksum.as_deref()
        && !vendor_hash.eq_ignore_ascii_case(expected)
      {
        issues.push(format!(
          "vendored source checksum mismatch: expected {expected}, actual {vendor_hash}"
        ));
      }
    }

    let status = if !issues.is_empty() {
      failed += 1;
      failed_summaries.push(format!("{} ({})", pkg.id, issues.join("; ")));
      "failed"
    } else if !advisory.is_empty() {
      warnings += 1;
      "warning"
    } else {
      passed += 1;
      "ok"
    };

    rows.push(json!({
      "id": pkg.id,
      "source": pkg.source,
      "status": status,
      "issues": issues,
      "advisory": advisory,
      "checksum_expected": expected_checksum,
      "checksum_actual": actual_checksum,
      "source_dir": source_dir.map(|p| p.display().to_string()),
      "vendor_dir": vendor_dir.exists().then(|| vendor_dir.display().to_string()),
      "vendor_checksum": vendor_checksum,
    }));

    sbom_components.push(json!({
      "id": pkg.id,
      "source": pkg.source,
      "registry": pkg.registry,
      "source_package": pkg.source_package,
      "resolved_version": pkg.resolved_version,
      "resolved_commit": pkg.resolved_commit,
      "source_git": pkg.source_git,
      "source_path": pkg.source_path,
      "source_url": pkg.source_url,
      "checksum_sha256": expected_checksum,
      "header_only": pkg.header_only,
      "libs": pkg.libs,
      "linkage": pkg.linkage,
    }));
  }

  let sbom = json!({
    "format": "joy-sbom-v1",
    "schema_version": "1",
    "generated_at_unix": now_unix_seconds(),
    "lockfile": lockfile_path.display().to_string(),
    "components": sbom_components,
  });

  let sbom_path = if let Some(raw) = args.sbom.as_deref() {
    let path = resolve_output_path(&project_root, raw);
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)
        .map_err(|err| JoyError::io("verify", "creating sbom output directory", parent, &err))?;
    }
    let raw = serde_json::to_vec_pretty(&sbom)
      .map_err(|err| JoyError::new("verify", "sbom_serialize_failed", err.to_string(), 1))?;
    fs::write(&path, raw)
      .map_err(|err| JoyError::io("verify", "writing sbom file", &path, &err))?;
    Some(path)
  } else {
    None
  };

  if failed > 0 {
    let mut message = format!("verification failed for {failed} package(s)");
    for item in failed_summaries {
      message.push_str(&format!("\n- {item}"));
    }
    return Err(JoyError::new("verify", "verify_failed", message, 1));
  }

  let human = HumanMessageBuilder::new("Dependency integrity verification passed")
    .kv("lockfile", lockfile_path.display().to_string())
    .kv("packages checked", lock.packages.len().to_string())
    .kv("passed", passed.to_string())
    .kv("warnings", warnings.to_string())
    .kv("failed", failed.to_string())
    .kv(
      "sbom",
      sbom_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "inline (JSON data.sbom)".to_string()),
    )
    .build();

  Ok(CommandOutput::new(
    "verify",
    human,
    json!({
      "project_root": project_root.display().to_string(),
      "lockfile_path": lockfile_path.display().to_string(),
      "strict": args.strict,
      "summary": {
        "package_count": lock.packages.len(),
        "passed_count": passed,
        "warning_count": warnings,
        "failed_count": failed,
      },
      "results": rows,
      "sbom": sbom,
      "sbom_path": sbom_path.map(|p| p.display().to_string()),
    }),
  ))
}

fn provenance_issues(pkg: &LockedPackage) -> Vec<String> {
  let mut issues = Vec::new();
  if pkg.resolved_commit.trim().is_empty() {
    issues.push("missing resolved commit in lockfile entry".to_string());
  }
  match pkg.source.as_str() {
    "github" => {},
    "registry" => {
      if pkg.registry.as_deref().is_none_or(|v| v.trim().is_empty()) {
        issues.push("registry source is missing `registry` provenance".to_string());
      }
    },
    "git" => {
      if pkg.source_git.as_deref().is_none_or(|v| v.trim().is_empty()) {
        issues.push("git source is missing `source_git` provenance".to_string());
      }
    },
    "path" => {
      if pkg.source_path.as_deref().is_none_or(|v| v.trim().is_empty()) {
        issues.push("path source is missing `source_path` provenance".to_string());
      }
    },
    "archive" => {
      if pkg.source_url.as_deref().is_none_or(|v| v.trim().is_empty()) {
        issues.push("archive source is missing `source_url` provenance".to_string());
      }
      if pkg.source_checksum_sha256.as_deref().is_none_or(|v| v.trim().is_empty()) {
        issues.push("archive source is missing `source_checksum_sha256` provenance".to_string());
      }
    },
    other => {
      issues.push(format!("unsupported lockfile source `{other}`"));
    },
  }
  issues
}

fn resolve_source_checksum(
  pkg: &LockedPackage,
  project_root: &Path,
) -> Result<(Option<PathBuf>, Option<String>), JoyError> {
  match pkg.source.as_str() {
    "github" | "registry" => {
      let package = PackageId::parse(&pkg.id)
        .map_err(|err| JoyError::new("verify", "invalid_package_id", err.to_string(), 1))?;
      let fetched = fetch::fetch_github(&package, &pkg.resolved_commit)
        .map_err(|err| JoyError::new("verify", "fetch_failed", err.to_string(), 1))?;
      let checksum = hash_path_sha256("verify", &fetched.source_dir)?;
      Ok((Some(fetched.source_dir), Some(checksum)))
    },
    "path" => {
      let Some(raw) = pkg.source_path.as_deref() else {
        return Ok((None, None));
      };
      let path = resolve_input_path(project_root, raw);
      if !path.exists() {
        return Ok((Some(path), None));
      }
      let checksum = hash_path_sha256("verify", &path)?;
      Ok((Some(path), Some(checksum)))
    },
    _ => Ok((None, None)),
  }
}

fn resolve_input_path(project_root: &Path, raw: &str) -> PathBuf {
  let path = Path::new(raw);
  if path.is_absolute() { path.to_path_buf() } else { project_root.join(path) }
}

fn resolve_output_path(project_root: &Path, raw: &str) -> PathBuf {
  let path = Path::new(raw);
  if path.is_absolute() { path.to_path_buf() } else { project_root.join(path) }
}

fn hash_path_sha256(command: &'static str, path: &Path) -> Result<String, JoyError> {
  if path.is_file() {
    let bytes = fs::read(path).map_err(|err| JoyError::io(command, "reading file", path, &err))?;
    let mut hasher = Sha256::new();
    hasher.update(path.file_name().and_then(|v| v.to_str()).unwrap_or("file").as_bytes());
    hasher.update([0u8]);
    hasher.update(bytes);
    return Ok(format!("{:x}", hasher.finalize()));
  }

  if path.is_dir() {
    let mut files = Vec::new();
    collect_files_recursive(command, path, path, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for rel in files {
      let abs = path.join(&rel);
      let bytes =
        fs::read(&abs).map_err(|err| JoyError::io(command, "reading source file", &abs, &err))?;
      hasher.update(rel.as_bytes());
      hasher.update([0u8]);
      hasher.update(bytes);
      hasher.update([0u8]);
    }
    return Ok(format!("{:x}", hasher.finalize()));
  }

  Err(JoyError::new(
    command,
    "verify_hash_path_missing",
    format!("cannot hash `{}` because it does not exist", path.display()),
    1,
  ))
}

fn collect_files_recursive(
  command: &'static str,
  root: &Path,
  current: &Path,
  out: &mut Vec<String>,
) -> Result<(), JoyError> {
  for entry in fs::read_dir(current)
    .map_err(|err| JoyError::io(command, "reading source directory", current, &err))?
  {
    let entry =
      entry.map_err(|err| JoyError::new(command, "verify_scan_failed", err.to_string(), 1))?;
    let path = entry.path();
    let metadata = entry
      .metadata()
      .map_err(|err| JoyError::io(command, "reading source metadata", &path, &err))?;
    if metadata.is_dir() {
      collect_files_recursive(command, root, &path, out)?;
    } else if metadata.is_file() {
      let rel =
        path.strip_prefix(root).unwrap_or(path.as_path()).to_string_lossy().replace('\\', "/");
      out.push(rel);
    }
  }
  Ok(())
}

fn vendor_slug(id: &str) -> String {
  id.chars()
    .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') { ch } else { '_' })
    .collect()
}

fn now_unix_seconds() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()
}
