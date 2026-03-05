use serde_json::json;
use std::env;
use std::fs;
use std::path::Path;

use crate::cli::{RuntimeFlags, VendorArgs};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fetch;
use crate::lockfile::Lockfile;
use crate::output::HumanMessageBuilder;
use crate::package_id::PackageId;

pub fn handle(args: VendorArgs, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let _fetch_runtime = fetch::push_runtime_options(fetch::RuntimeOptions {
    offline: runtime.offline,
    progress: runtime.progress,
  });
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("vendor", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let lockfile_path = cwd.join("joy.lock");
  if !lockfile_path.is_file() {
    return Err(JoyError::new(
      "vendor",
      "lockfile_not_found",
      format!("no `joy.lock` found at {}", lockfile_path.display()),
      1,
    ));
  }
  let lock = Lockfile::load(&lockfile_path)
    .map_err(|err| JoyError::new("vendor", "lockfile_parse_error", err.to_string(), 1))?;
  let output_root = cwd.join(args.output.as_deref().unwrap_or("vendor"));
  fs::create_dir_all(&output_root).map_err(|err| {
    JoyError::io("vendor", "creating vendor output directory", &output_root, &err)
  })?;

  let mut vendored = Vec::new();
  let mut skipped = Vec::new();
  for pkg in &lock.packages {
    if pkg.source != "github" && pkg.source != "registry" {
      skipped.push(json!({
        "id": pkg.id,
        "source": pkg.source,
        "reason": "vendor currently supports github/registry lockfile entries only",
      }));
      continue;
    }
    let package = match PackageId::parse(&pkg.id) {
      Ok(package) => package,
      Err(err) => {
        skipped.push(json!({
          "id": pkg.id,
          "source": pkg.source,
          "reason": format!("invalid package id in lockfile: {err}"),
        }));
        continue;
      },
    };
    let fetched = fetch::fetch_github(&package, &pkg.resolved_commit)
      .map_err(|err| JoyError::new("vendor", "fetch_failed", err.to_string(), 1))?;
    let target_dir = output_root.join(vendor_slug(&pkg.id)).join(&pkg.resolved_commit);
    if target_dir.exists() {
      fs::remove_dir_all(&target_dir).map_err(|err| {
        JoyError::io("vendor", "removing stale vendor directory", &target_dir, &err)
      })?;
    }
    copy_dir_recursive(&fetched.source_dir, &target_dir)?;
    vendored.push(json!({
      "id": pkg.id,
      "source": pkg.source,
      "resolved_commit": pkg.resolved_commit,
      "source_dir": fetched.source_dir.display().to_string(),
      "vendor_dir": target_dir.display().to_string(),
      "cache_hit": fetched.cache_hit,
    }));
  }

  let human = HumanMessageBuilder::new("Vendored lockfile dependencies")
    .kv("output", output_root.display().to_string())
    .kv("vendored", vendored.len().to_string())
    .kv("skipped", skipped.len().to_string())
    .build();
  Ok(CommandOutput::new(
    "vendor",
    human,
    json!({
      "project_root": cwd.display().to_string(),
      "lockfile_path": lockfile_path.display().to_string(),
      "output_dir": output_root.display().to_string(),
      "vendored_count": vendored.len(),
      "skipped_count": skipped.len(),
      "vendored": vendored,
      "skipped": skipped,
    }),
  ))
}

fn vendor_slug(id: &str) -> String {
  id.chars()
    .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') { ch } else { '_' })
    .collect()
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), JoyError> {
  if !src.is_dir() {
    return Err(JoyError::new(
      "vendor",
      "vendor_copy_failed",
      format!("source directory `{}` does not exist", src.display()),
      1,
    ));
  }
  fs::create_dir_all(dst)
    .map_err(|err| JoyError::io("vendor", "creating vendor directory", dst, &err))?;
  for entry in fs::read_dir(src)
    .map_err(|err| JoyError::io("vendor", "reading source directory", src, &err))?
  {
    let entry =
      entry.map_err(|err| JoyError::new("vendor", "vendor_copy_failed", err.to_string(), 1))?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());
    let metadata = entry
      .metadata()
      .map_err(|err| JoyError::io("vendor", "reading source metadata", &src_path, &err))?;
    if metadata.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else if metadata.is_file() {
      fs::copy(&src_path, &dst_path)
        .map_err(|err| JoyError::io("vendor", "copying source file", &dst_path, &err))?;
    }
  }
  Ok(())
}
