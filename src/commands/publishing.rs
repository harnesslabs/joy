use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{
  OwnerArgs, OwnerSubcommand, PackageArgs, PackageInitArgs, PackageKindArg, PackageSubcommand,
  PublishArgs, RuntimeFlags, YankArgs,
};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::git_ops;
use crate::lockfile;
use crate::manifest::{DependencyRequirementRef, PackageKind, PackageManifest};
use crate::output::HumanMessageBuilder;
use crate::package_id::PackageId;
use crate::registry_config;

const DEFAULT_REGISTRY_NAME: &str = "default";

pub fn handle_package(args: PackageArgs) -> Result<CommandOutput, JoyError> {
  match args.command {
    PackageSubcommand::Init(init) => handle_package_init(init),
  }
}

pub fn handle_publish(
  args: PublishArgs,
  _runtime: RuntimeFlags,
) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("publish", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  let package_manifest = PackageManifest::load(&manifest_path)
    .map_err(|err| JoyError::new("publish", "manifest_parse_error", err.to_string(), 1))?;
  let _ = PackageId::parse(&package_manifest.package.id)
    .map_err(|err| JoyError::new("publish", "invalid_package_id", err.to_string(), 1))?;
  let _ = Version::parse(&package_manifest.package.version).map_err(|err| {
    JoyError::new(
      "publish",
      "invalid_package_version",
      format!("invalid package version `{}`: {err}", package_manifest.package.version),
      1,
    )
  })?;

  let registry_target = resolve_registry_target("publish", args.registry.as_deref(), &cwd)?;
  let mut index = load_registry_index("publish", &registry_target.index_path)?;
  let package = ensure_package_entry(&mut index, &package_manifest.package.id);

  if package.versions.iter().any(|entry| entry.version == package_manifest.package.version) {
    return Err(JoyError::new(
      "publish",
      "publish_version_exists",
      format!(
        "package `{}` version `{}` already exists in registry `{}`",
        package_manifest.package.id, package_manifest.package.version, registry_target.name
      ),
      1,
    ));
  }

  let source_package = args
    .source_package
    .as_deref()
    .filter(|v| !v.trim().is_empty())
    .unwrap_or(&package_manifest.package.id)
    .to_string();
  let source_rev = args
    .rev
    .as_deref()
    .filter(|v| !v.trim().is_empty())
    .map(ToOwned::to_owned)
    .unwrap_or_else(|| format!("v{}", package_manifest.package.version));

  let manifest_digest = lockfile::compute_manifest_hash(&manifest_path)
    .map_err(|err| JoyError::new("publish", "manifest_hash_failed", err.to_string(), 1))?;
  let manifest_summary = RegistryIndexManifestSummary {
    digest: Some(format!("sha256:{manifest_digest}")),
    kind: Some(match package_manifest.package.kind {
      PackageKind::HeaderOnly => "header_only".to_string(),
      PackageKind::Cmake => "cmake".to_string(),
    }),
    headers_include_roots: package_manifest.headers.include_roots.clone(),
    dependencies: package_manifest
      .dependencies
      .iter()
      .filter_map(|(id, spec)| {
        let requirement = package_manifest.dependency_requirement(id)?;
        let (rev, version) = match requirement {
          DependencyRequirementRef::Rev(rev) => (Some(rev.to_string()), None),
          DependencyRequirementRef::Version(version) => (None, Some(version.to_string())),
        };
        Some(RegistryIndexManifestDependency {
          id: id.clone(),
          source: spec.source.clone(),
          rev,
          version,
        })
      })
      .collect(),
  };

  package.versions.push(RegistryIndexVersionEntry {
    version: package_manifest.package.version.clone(),
    source: "github".to_string(),
    package: source_package,
    rev: source_rev,
    yanked: false,
    manifest: Some(manifest_summary),
  });
  sort_package_versions(package);

  save_registry_index("publish", &registry_target.index_path, &index)?;
  let committed = commit_index_if_git_repo(
    "publish",
    &registry_target.registry_root,
    &format!("publish {} {}", package_manifest.package.id, package_manifest.package.version),
  )?;

  Ok(CommandOutput::new(
    "publish",
    HumanMessageBuilder::new("Published package to registry index")
      .kv("package", package_manifest.package.id.clone())
      .kv("version", package_manifest.package.version.clone())
      .kv("registry", registry_target.name.clone())
      .kv("index", registry_target.index_path.display().to_string())
      .kv("git committed", committed.to_string())
      .build(),
    json!({
      "package": package_manifest.package.id,
      "version": package_manifest.package.version,
      "registry": registry_target.name,
      "index_path": registry_target.index_path.display().to_string(),
      "git_committed": committed,
    }),
  ))
}

pub fn handle_owner(args: OwnerArgs, _runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  match args.command {
    OwnerSubcommand::List(list) => {
      let cwd = env::current_dir().map_err(|err| {
        JoyError::new("owner", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
      })?;
      let registry_target = resolve_registry_target("owner", list.registry.as_deref(), &cwd)?;
      let index = load_registry_index("owner", &registry_target.index_path)?;
      let pkg = index.packages.iter().find(|pkg| pkg.id == list.package).ok_or_else(|| {
        JoyError::new(
          "owner",
          "registry_package_not_found",
          format!("package `{}` not found in registry `{}`", list.package, registry_target.name),
          1,
        )
      })?;
      let mut owners = pkg.owners.clone();
      owners.sort();
      Ok(CommandOutput::new(
        "owner",
        HumanMessageBuilder::new("Package owners")
          .kv("package", pkg.id.clone())
          .kv("registry", registry_target.name.clone())
          .kv("owner count", owners.len().to_string())
          .build(),
        json!({
          "action": "list",
          "package": pkg.id,
          "registry": registry_target.name,
          "owners": owners,
        }),
      ))
    },
    OwnerSubcommand::Add(add) => {
      mutate_owner("add", &add.package, &add.owner, add.registry.as_deref())
    },
    OwnerSubcommand::Remove(remove) => {
      mutate_owner("remove", &remove.package, &remove.owner, remove.registry.as_deref())
    },
  }
}

pub fn handle_yank(args: YankArgs, _runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("yank", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let _ = Version::parse(&args.version).map_err(|err| {
    JoyError::new(
      "yank",
      "invalid_package_version",
      format!("invalid version `{}`: {err}", args.version),
      1,
    )
  })?;
  let registry_target = resolve_registry_target("yank", args.registry.as_deref(), &cwd)?;
  let mut index = load_registry_index("yank", &registry_target.index_path)?;

  let pkg = index.packages.iter_mut().find(|pkg| pkg.id == args.package).ok_or_else(|| {
    JoyError::new(
      "yank",
      "registry_package_not_found",
      format!("package `{}` not found in registry `{}`", args.package, registry_target.name),
      1,
    )
  })?;
  let release =
    pkg.versions.iter_mut().find(|entry| entry.version == args.version).ok_or_else(|| {
      JoyError::new(
        "yank",
        "version_not_found",
        format!(
          "version `{}` for `{}` not found in registry `{}`",
          args.version, args.package, registry_target.name
        ),
        1,
      )
    })?;

  let desired = !args.undo;
  let changed = release.yanked != desired;
  release.yanked = desired;

  if changed {
    save_registry_index("yank", &registry_target.index_path, &index)?;
    let _ = commit_index_if_git_repo(
      "yank",
      &registry_target.registry_root,
      &format!("{} {} {}", if args.undo { "unyank" } else { "yank" }, args.package, args.version),
    )?;
  }

  Ok(CommandOutput::new(
    "yank",
    HumanMessageBuilder::new(if args.undo {
      "Package version unyanked"
    } else {
      "Package version yanked"
    })
    .kv("package", args.package.clone())
    .kv("version", args.version.clone())
    .kv("registry", registry_target.name.clone())
    .kv("changed", changed.to_string())
    .build(),
    json!({
      "package": args.package,
      "version": args.version,
      "registry": registry_target.name,
      "yanked": desired,
      "changed": changed,
      "index_path": registry_target.index_path.display().to_string(),
    }),
  ))
}

fn handle_package_init(args: PackageInitArgs) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("package", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let _ = PackageId::parse(&args.id)
    .map_err(|err| JoyError::new("package", "invalid_package_id", err.to_string(), 1))?;
  let _ = Version::parse(&args.version).map_err(|err| {
    JoyError::new(
      "package",
      "invalid_package_version",
      format!("invalid package version `{}`: {err}", args.version),
      1,
    )
  })?;

  let manifest_path = cwd.join("joy.toml");
  if manifest_path.exists() && !args.force {
    return Err(JoyError::new(
      "package",
      "manifest_exists",
      format!("`{}` already exists; rerun with `--force` to overwrite", manifest_path.display()),
      1,
    ));
  }

  let kind = match args.kind {
    PackageKindArg::HeaderOnly => "header_only",
    PackageKindArg::Cmake => "cmake",
  };
  let mut raw = String::new();
  raw.push_str("[package]\n");
  raw.push_str(&format!("id = \"{}\"\n", args.id));
  raw.push_str(&format!("version = \"{}\"\n", args.version));
  raw.push_str(&format!("kind = \"{kind}\"\n\n"));
  raw.push_str("[headers]\n");
  raw.push_str("include_roots = [\"include\"]\n\n");
  raw.push_str("[dependencies]\n");

  fs::write(&manifest_path, raw)
    .map_err(|err| JoyError::io("package", "writing package manifest", &manifest_path, &err))?;

  Ok(CommandOutput::new(
    "package",
    HumanMessageBuilder::new("Initialized package manifest")
      .kv("manifest", manifest_path.display().to_string())
      .kv("id", args.id.clone())
      .kv("version", args.version.clone())
      .kv("kind", kind.to_string())
      .build(),
    json!({
      "action": "init",
      "manifest_path": manifest_path.display().to_string(),
      "id": args.id,
      "version": args.version,
      "kind": kind,
    }),
  ))
}

fn mutate_owner(
  action: &'static str,
  package: &str,
  owner: &str,
  registry: Option<&str>,
) -> Result<CommandOutput, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("owner", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  if owner.trim().is_empty() {
    return Err(JoyError::new("owner", "invalid_owner", "owner must not be empty", 1));
  }
  let registry_target = resolve_registry_target("owner", registry, &cwd)?;
  let mut index = load_registry_index("owner", &registry_target.index_path)?;

  let pkg = index.packages.iter_mut().find(|pkg| pkg.id == package).ok_or_else(|| {
    JoyError::new(
      "owner",
      "registry_package_not_found",
      format!("package `{}` not found in registry `{}`", package, registry_target.name),
      1,
    )
  })?;

  let changed = match action {
    "add" => {
      if pkg.owners.iter().any(|existing| existing == owner) {
        false
      } else {
        pkg.owners.push(owner.to_string());
        pkg.owners.sort();
        pkg.owners.dedup();
        true
      }
    },
    "remove" => {
      let before = pkg.owners.len();
      pkg.owners.retain(|existing| existing != owner);
      before != pkg.owners.len()
    },
    _ => false,
  };

  if changed {
    save_registry_index("owner", &registry_target.index_path, &index)?;
    let _ = commit_index_if_git_repo(
      "owner",
      &registry_target.registry_root,
      &format!("owner {action} {package} {owner}"),
    )?;
  }

  Ok(CommandOutput::new(
    "owner",
    HumanMessageBuilder::new(if action == "add" { "Owner added" } else { "Owner removed" })
      .kv("package", package.to_string())
      .kv("owner", owner.to_string())
      .kv("registry", registry_target.name.clone())
      .kv("changed", changed.to_string())
      .build(),
    json!({
      "action": action,
      "package": package,
      "owner": owner,
      "registry": registry_target.name,
      "changed": changed,
      "index_path": registry_target.index_path.display().to_string(),
    }),
  ))
}

#[derive(Debug, Clone)]
struct RegistryTarget {
  name: String,
  registry_root: PathBuf,
  index_path: PathBuf,
}

fn resolve_registry_target(
  command: &'static str,
  registry_name: Option<&str>,
  project_root: &Path,
) -> Result<RegistryTarget, JoyError> {
  let effective = registry_config::load_effective(Some(project_root))
    .map_err(|err| JoyError::new(command, "registry_config_error", err.to_string(), 1))?;
  let selected = registry_name
    .map(ToOwned::to_owned)
    .or(effective.default.clone())
    .or_else(|| effective.registries.keys().next().cloned())
    .unwrap_or_else(|| DEFAULT_REGISTRY_NAME.to_string());
  let raw = effective.resolve_url(&selected).ok_or_else(|| {
    JoyError::new(
      command,
      "registry_not_configured",
      format!(
        "registry `{selected}` is not configured for publish workflows; configure it with `joy registry add {selected} <index-path> [--project]`"
      ),
      1,
    )
  })?;
  if raw.contains("://") {
    return Err(JoyError::new(
      command,
      "publish_registry_transport_unsupported",
      format!(
        "registry `{selected}` uses remote transport `{raw}`; publish/owner/yank currently require a local filesystem index path"
      ),
      1,
    ));
  }
  let raw_path = PathBuf::from(raw);
  let index_path = if raw_path
    .file_name()
    .and_then(|v| v.to_str())
    .is_some_and(|name| name.eq_ignore_ascii_case("index.toml"))
  {
    raw_path.clone()
  } else {
    raw_path.join("index.toml")
  };
  let registry_root =
    index_path.parent().map(Path::to_path_buf).unwrap_or_else(|| project_root.to_path_buf());
  Ok(RegistryTarget { name: selected, registry_root, index_path })
}

fn load_registry_index(
  command: &'static str,
  index_path: &Path,
) -> Result<RegistryIndexFile, JoyError> {
  if !index_path.exists() {
    return Ok(RegistryIndexFile::default());
  }
  let raw = fs::read_to_string(index_path)
    .map_err(|err| JoyError::io(command, "reading registry index", index_path, &err))?;
  let parsed: RegistryIndexFile = toml::from_str(&raw)
    .map_err(|err| JoyError::new(command, "registry_index_parse_error", err.to_string(), 1))?;
  if parsed.version != 1 && parsed.version != 2 {
    return Err(JoyError::new(
      command,
      "registry_index_unsupported_version",
      format!("unsupported registry index version `{}`", parsed.version),
      1,
    ));
  }
  Ok(parsed)
}

fn save_registry_index(
  command: &'static str,
  index_path: &Path,
  index: &RegistryIndexFile,
) -> Result<(), JoyError> {
  if let Some(parent) = index_path.parent() {
    fs::create_dir_all(parent)
      .map_err(|err| JoyError::io(command, "creating registry index parent", parent, &err))?;
  }
  let mut raw = toml::to_string_pretty(index)
    .map_err(|err| JoyError::new(command, "registry_index_serialize_failed", err.to_string(), 1))?;
  if !raw.ends_with('\n') {
    raw.push('\n');
  }
  fs::write(index_path, raw)
    .map_err(|err| JoyError::io(command, "writing registry index", index_path, &err))
}

fn ensure_package_entry<'a>(
  index: &'a mut RegistryIndexFile,
  package_id: &str,
) -> &'a mut RegistryIndexPackageEntry {
  if let Some(existing) = index.packages.iter().position(|pkg| pkg.id == package_id) {
    return index.packages.get_mut(existing).expect("existing package entry");
  }
  index.packages.push(RegistryIndexPackageEntry {
    id: package_id.to_string(),
    owners: Vec::new(),
    versions: Vec::new(),
  });
  index.packages.sort_by(|a, b| a.id.cmp(&b.id));
  let idx =
    index.packages.iter().position(|pkg| pkg.id == package_id).expect("inserted package entry");
  index.packages.get_mut(idx).expect("inserted package entry mut")
}

fn sort_package_versions(package: &mut RegistryIndexPackageEntry) {
  package.versions.sort_by(|a, b| {
    let av = Version::parse(&a.version).ok();
    let bv = Version::parse(&b.version).ok();
    match (av, bv) {
      (Some(av), Some(bv)) => bv.cmp(&av),
      _ => b.version.cmp(&a.version),
    }
  });
}

fn commit_index_if_git_repo(
  command: &'static str,
  registry_root: &Path,
  message: &str,
) -> Result<bool, JoyError> {
  if !registry_root.join(".git").is_dir() {
    return Ok(false);
  }
  let status = git_ops::run_capture(
    Some(registry_root),
    ["status", "--porcelain", "--", "index.toml"],
    "checking registry index changes",
  )
  .map_err(|err| JoyError::new(command, "git_failed", err.to_string(), 1))?;
  if status.trim().is_empty() {
    return Ok(false);
  }

  git_ops::run(Some(registry_root), ["add", "index.toml"], "staging registry index update")
    .map_err(|err| JoyError::new(command, "git_failed", err.to_string(), 1))?;

  ensure_git_identity(command, registry_root)?;

  git_ops::run(Some(registry_root), ["commit", "-m", message], "committing registry index update")
    .map_err(|err| JoyError::new(command, "git_failed", err.to_string(), 1))?;
  Ok(true)
}

fn ensure_git_identity(command: &'static str, registry_root: &Path) -> Result<(), JoyError> {
  let has_name = git_ops::run_capture(
    Some(registry_root),
    ["config", "--get", "user.name"],
    "reading git user.name",
  )
  .is_ok();
  if !has_name {
    git_ops::run(
      Some(registry_root),
      ["config", "user.name", "Joy Publish"],
      "setting git user.name",
    )
    .map_err(|err| JoyError::new(command, "git_failed", err.to_string(), 1))?;
  }

  let has_email = git_ops::run_capture(
    Some(registry_root),
    ["config", "--get", "user.email"],
    "reading git user.email",
  )
  .is_ok();
  if !has_email {
    git_ops::run(
      Some(registry_root),
      ["config", "user.email", "joy-publish@example.local"],
      "setting git user.email",
    )
    .map_err(|err| JoyError::new(command, "git_failed", err.to_string(), 1))?;
  }

  Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryIndexFile {
  #[serde(default = "default_registry_index_version")]
  version: u32,
  #[serde(default)]
  packages: Vec<RegistryIndexPackageEntry>,
}

impl Default for RegistryIndexFile {
  fn default() -> Self {
    Self { version: 2, packages: Vec::new() }
  }
}

fn default_registry_index_version() -> u32 {
  2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryIndexPackageEntry {
  id: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  owners: Vec<String>,
  #[serde(default)]
  versions: Vec<RegistryIndexVersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryIndexVersionEntry {
  version: String,
  source: String,
  package: String,
  rev: String,
  #[serde(default)]
  yanked: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  manifest: Option<RegistryIndexManifestSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryIndexManifestSummary {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  digest: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  kind: Option<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  headers_include_roots: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  dependencies: Vec<RegistryIndexManifestDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryIndexManifestDependency {
  id: String,
  source: crate::manifest::DependencySource,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  rev: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  version: Option<String>,
}

#[cfg(test)]
mod tests {
  use tempfile::TempDir;

  use super::{RegistryIndexFile, save_registry_index};

  #[test]
  fn can_create_empty_registry_index() {
    let temp = TempDir::new().expect("tempdir");
    let index_path = temp.path().join("index.toml");
    save_registry_index("publish", &index_path, &RegistryIndexFile::default()).expect("save index");
    assert!(index_path.is_file());
  }
}
