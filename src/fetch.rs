//! Source fetching backends and cache materialization helpers.
//!
//! `joy` currently prefers a git-mirror workflow (using the system `git` binary) for GitHub
//! shorthand dependencies so refs can be resolved locally and subsequent fetches can reuse the
//! mirror.

use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::fs_ops;
use crate::git_ops::{self, GitCommandError};
use crate::global_cache::{GlobalCache, GlobalCacheError};
use crate::output::progress_detail_tty;
use crate::package_id::PackageId;

const FETCH_FLAG_OFFLINE: u8 = 1 << 0;
const FETCH_FLAG_PROGRESS: u8 = 1 << 1;
const TRANSIENT_RETRY_ATTEMPTS: usize = 3;
static FETCH_RUNTIME_FLAGS: AtomicU8 = AtomicU8::new(0);

/// Runtime fetch behavior toggles derived from global CLI flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeOptions {
  pub offline: bool,
  pub progress: bool,
}

/// RAII guard that restores the previous fetch runtime options on drop.
pub struct RuntimeOptionsGuard {
  previous_flags: u8,
}

impl Drop for RuntimeOptionsGuard {
  fn drop(&mut self) {
    FETCH_RUNTIME_FLAGS.store(self.previous_flags, Ordering::SeqCst);
  }
}

/// Set process-wide fetch runtime options for the current command execution.
///
/// `joy` runs one command per process, so a lightweight global is sufficient and keeps the fetch
/// API surface stable while `--offline`/`--frozen` semantics are introduced.
pub fn push_runtime_options(options: RuntimeOptions) -> RuntimeOptionsGuard {
  let mut flags = 0u8;
  if options.offline {
    flags |= FETCH_FLAG_OFFLINE;
  }
  if options.progress {
    flags |= FETCH_FLAG_PROGRESS;
  }
  let previous_flags = FETCH_RUNTIME_FLAGS.swap(flags, Ordering::SeqCst);
  RuntimeOptionsGuard { previous_flags }
}

fn runtime_offline_enabled() -> bool {
  FETCH_RUNTIME_FLAGS.load(Ordering::SeqCst) & FETCH_FLAG_OFFLINE != 0
}

fn runtime_progress_enabled() -> bool {
  FETCH_RUNTIME_FLAGS.load(Ordering::SeqCst) & FETCH_FLAG_PROGRESS != 0
}

/// Read the current process-wide fetch runtime toggles (used by other transport/cache modules).
pub(crate) fn runtime_options() -> RuntimeOptions {
  RuntimeOptions { offline: runtime_offline_enabled(), progress: runtime_progress_enabled() }
}

/// Result of fetching a package source tree (or reusing an existing cached checkout).
#[derive(Debug, Clone)]
pub struct FetchResult {
  pub package: PackageId,
  pub requested_rev: String,
  pub requested_requirement: Option<String>,
  pub resolved_version: Option<String>,
  pub resolved_commit: String,
  pub source_dir: PathBuf,
  pub remote_url: String,
  pub cache_hit: bool,
}

/// Fetch a GitHub shorthand package using the default global cache.
pub fn fetch_github(package: &PackageId, requested_rev: &str) -> Result<FetchResult, FetchError> {
  let cache = GlobalCache::resolve()?;
  fetch_github_with_cache(package, requested_rev, &cache)
}

/// Resolve a semver requirement using git tags, then materialize the selected checkout.
pub fn fetch_github_semver(
  package: &PackageId,
  version_req: &str,
) -> Result<FetchResult, FetchError> {
  let cache = GlobalCache::resolve()?;
  fetch_github_semver_with_cache(package, version_req, &cache)
}

/// Fetch a generic git dependency using the global cache.
pub fn fetch_git(
  package: &PackageId,
  source_git: &str,
  requested_rev: &str,
) -> Result<FetchResult, FetchError> {
  let cache = GlobalCache::resolve()?;
  fetch_git_with_cache(package, source_git, requested_rev, &cache)
}

/// Materialize a path dependency into a deterministic cache snapshot.
pub fn fetch_path(package: &PackageId, source_path: &str) -> Result<FetchResult, FetchError> {
  let cache = GlobalCache::resolve()?;
  fetch_path_with_cache(package, source_path, &cache)
}

/// Materialize an archive dependency into the source cache after checksum validation.
pub fn fetch_archive(
  package: &PackageId,
  source_url: &str,
  expected_sha256: &str,
) -> Result<FetchResult, FetchError> {
  let cache = GlobalCache::resolve()?;
  fetch_archive_with_cache(package, source_url, expected_sha256, &cache)
}

/// Fetch a GitHub shorthand package using an explicit cache layout.
///
/// The requested ref is resolved to a concrete commit and the source checkout is materialized under
/// the commit-keyed source cache path.
pub fn fetch_github_with_cache(
  package: &PackageId,
  requested_rev: &str,
  cache: &GlobalCache,
) -> Result<FetchResult, FetchError> {
  cache.ensure_layout()?;

  let remote_url = github_remote_url(package);
  let mirror_dir = cache.git_mirror_dir(package);
  let requested = if requested_rev.trim().is_empty() { "HEAD" } else { requested_rev };
  if runtime_offline_enabled() && !mirror_dir.exists() {
    return Err(FetchError::OfflineCacheMiss {
      package: package.to_string(),
      requested_rev: requested.to_string(),
      mirror_dir,
    });
  }
  ensure_mirror(&remote_url, &mirror_dir)?;

  let resolved_commit = resolve_commit(&mirror_dir, requested)?;

  let source_dir = cache.source_checkout_dir(package, &resolved_commit);
  let cache_hit = source_dir.exists();
  if !cache_hit {
    materialize_checkout(&mirror_dir, &source_dir, &resolved_commit, cache.tmp_dir())?;
  }

  Ok(FetchResult {
    package: package.clone(),
    requested_rev: requested.to_string(),
    requested_requirement: None,
    resolved_version: None,
    resolved_commit,
    source_dir,
    remote_url,
    cache_hit,
  })
}

/// Resolve a semver requirement from cached/remote git tags and materialize the selected checkout.
pub fn fetch_github_semver_with_cache(
  package: &PackageId,
  version_req: &str,
  cache: &GlobalCache,
) -> Result<FetchResult, FetchError> {
  cache.ensure_layout()?;

  let remote_url = github_remote_url(package);
  let mirror_dir = cache.git_mirror_dir(package);
  if runtime_offline_enabled() && !mirror_dir.exists() {
    return Err(FetchError::OfflineCacheMiss {
      package: package.to_string(),
      requested_rev: format!("version:{version_req}"),
      mirror_dir,
    });
  }
  ensure_mirror(&remote_url, &mirror_dir)?;

  let selected = resolve_semver_tag(&mirror_dir, package, version_req)?;
  let source_dir = cache.source_checkout_dir(package, &selected.resolved_commit);
  let cache_hit = source_dir.exists();
  if !cache_hit {
    materialize_checkout(&mirror_dir, &source_dir, &selected.resolved_commit, cache.tmp_dir())?;
  }

  Ok(FetchResult {
    package: package.clone(),
    requested_rev: selected.resolved_tag,
    requested_requirement: Some(version_req.to_string()),
    resolved_version: Some(selected.resolved_version),
    resolved_commit: selected.resolved_commit,
    source_dir,
    remote_url,
    cache_hit,
  })
}

pub fn fetch_git_with_cache(
  package: &PackageId,
  source_git: &str,
  requested_rev: &str,
  cache: &GlobalCache,
) -> Result<FetchResult, FetchError> {
  cache.ensure_layout()?;

  let locator_key = source_locator_key(source_git);
  let mirror_dir = cache
    .git_root
    .join("git")
    .join(package.owner())
    .join(package.repo())
    .join(format!("{locator_key}.git"));
  let requested = if requested_rev.trim().is_empty() { "HEAD" } else { requested_rev };
  if runtime_offline_enabled() && !mirror_dir.exists() {
    return Err(FetchError::OfflineCacheMiss {
      package: package.to_string(),
      requested_rev: requested.to_string(),
      mirror_dir,
    });
  }
  ensure_mirror(source_git, &mirror_dir)?;

  let resolved_commit = resolve_commit(&mirror_dir, requested)?;
  let source_dir = cache
    .src_root
    .join("git")
    .join(package.owner())
    .join(package.repo())
    .join(locator_key)
    .join(&resolved_commit);
  let cache_hit = source_dir.exists();
  if !cache_hit {
    materialize_checkout(&mirror_dir, &source_dir, &resolved_commit, cache.tmp_dir())?;
  }

  Ok(FetchResult {
    package: package.clone(),
    requested_rev: requested.to_string(),
    requested_requirement: None,
    resolved_version: None,
    resolved_commit,
    source_dir,
    remote_url: source_git.to_string(),
    cache_hit,
  })
}

pub fn fetch_path_with_cache(
  package: &PackageId,
  source_path: &str,
  cache: &GlobalCache,
) -> Result<FetchResult, FetchError> {
  cache.ensure_layout()?;
  let source_dir = resolve_input_path(source_path)?;
  if !source_dir.is_dir() {
    return Err(FetchError::Io {
      action: "reading path dependency".into(),
      path: source_dir,
      source: std::io::Error::new(std::io::ErrorKind::NotFound, "path dependency is missing"),
    });
  }

  let digest = hash_path_sha256(&source_dir)?;
  let commit = format!("sha256:{digest}");
  let cached_dir =
    cache.src_root.join("path").join(package.owner()).join(package.repo()).join(&digest);
  let cache_hit = cached_dir.exists();
  if !cache_hit {
    mirror_path_dependency(&source_dir, &cached_dir, cache.tmp_dir())?;
  }

  Ok(FetchResult {
    package: package.clone(),
    requested_rev: source_dir.display().to_string(),
    requested_requirement: None,
    resolved_version: None,
    resolved_commit: commit,
    source_dir: cached_dir,
    remote_url: source_dir.display().to_string(),
    cache_hit,
  })
}

pub fn fetch_archive_with_cache(
  package: &PackageId,
  source_url: &str,
  expected_sha256: &str,
  cache: &GlobalCache,
) -> Result<FetchResult, FetchError> {
  cache.ensure_layout()?;
  let normalized_checksum = normalize_sha256(expected_sha256)?;
  let locator_key = source_locator_key(source_url);
  let archive_file = cache
    .archives_root
    .join(package.owner())
    .join(package.repo())
    .join(format!("{locator_key}.tar.gz"));
  let extracted_dir = cache
    .src_root
    .join("archive")
    .join(package.owner())
    .join(package.repo())
    .join(&locator_key)
    .join(&normalized_checksum);
  let cache_hit = extracted_dir.exists();
  if !cache_hit {
    if runtime_offline_enabled() && !archive_file.exists() {
      return Err(FetchError::OfflineCacheMiss {
        package: package.to_string(),
        requested_rev: source_url.to_string(),
        mirror_dir: archive_file,
      });
    }
    materialize_archive_snapshot(
      package,
      source_url,
      &normalized_checksum,
      &archive_file,
      &extracted_dir,
      cache.tmp_dir(),
    )?;
  }

  let tree_hash = hash_path_sha256(&extracted_dir)?;
  Ok(FetchResult {
    package: package.clone(),
    requested_rev: source_url.to_string(),
    requested_requirement: None,
    resolved_version: None,
    resolved_commit: format!("sha256:{tree_hash}"),
    source_dir: extracted_dir,
    remote_url: source_url.to_string(),
    cache_hit,
  })
}

/// Download and extract a `.tar.gz` archive into `dest_dir`.
///
/// This API remains for backwards compatibility with older library consumers, but archive
/// transport is no longer used by the CLI dependency pipeline.
pub fn download_and_extract_tar_gz(url: &str, _dest_dir: &Path) -> Result<(), FetchError> {
  if runtime_offline_enabled() {
    return Err(FetchError::OfflineNetworkDisabled {
      action: "downloading archive".to_string(),
      url: url.to_string(),
    });
  }
  Err(FetchError::Runtime(std::io::Error::new(
    std::io::ErrorKind::Unsupported,
    "archive transport is not supported in this joy build",
  )))
}

/// Prefetch multiple GitHub packages in parallel while preserving the original request ordering.
pub fn prefetch_github_packages(
  requests: Vec<(PackageId, String)>,
) -> Result<Vec<FetchResult>, FetchError> {
  if requests.is_empty() {
    return Ok(Vec::new());
  }

  let total = requests.len();
  let worker_count =
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).max(1).min(total);
  let requests = std::sync::Arc::new(requests);
  let mut handles = Vec::with_capacity(worker_count);

  for worker_idx in 0..worker_count {
    let requests = std::sync::Arc::clone(&requests);
    handles.push(std::thread::spawn(move || {
      let mut partial = Vec::new();
      let mut idx = worker_idx;
      while idx < requests.len() {
        let (package, rev) = &requests[idx];
        partial.push((idx, fetch_github(package, rev)));
        idx += worker_count;
      }
      partial
    }));
  }

  let mut results: Vec<Option<FetchResult>> = vec![None; total];
  for handle in handles {
    let partial =
      handle.join().map_err(|_| FetchError::TaskJoin("prefetch worker thread panicked".into()))?;
    for (idx, fetched) in partial {
      results[idx] = Some(fetched?);
    }
  }

  results
    .into_iter()
    .map(|item| item.ok_or_else(|| FetchError::TaskJoin("missing prefetch result".into())))
    .collect::<Result<Vec<_>, _>>()
}

fn github_remote_url(package: &PackageId) -> String {
  let base = std::env::var("JOY_GITHUB_BASE").unwrap_or_else(|_| "https://github.com".to_string());
  let base = base.trim_end_matches('/');
  if base.contains("://") {
    format!("{base}/{}/{}.git", package.owner(), package.repo())
  } else {
    Path::new(base)
      .join(package.owner())
      .join(format!("{}.git", package.repo()))
      .display()
      .to_string()
  }
}

fn ensure_mirror(remote_url: &str, mirror_dir: &Path) -> Result<(), FetchError> {
  if mirror_dir.exists() {
    if runtime_offline_enabled() {
      emit_progress(&format!(
        "Using cached source mirror for `{remote_url}` in offline mode ({})",
        mirror_dir.display()
      ));
      return Ok(());
    }
    emit_progress(&format!(
      "Refreshing cached source mirror from `{remote_url}` ({})",
      mirror_dir.display()
    ));
    retry_transient_network("fetching mirror", || {
      git_ops::run(Some(mirror_dir), ["fetch", "--all", "--tags", "--prune"], "fetching mirror")
        .map_err(map_git_error)
    })?;
    return Ok(());
  }

  if let Some(parent) = mirror_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| FetchError::Io {
      action: "creating mirror parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }

  emit_progress(&format!(
    "Cloning source mirror from `{remote_url}` into {}",
    mirror_dir.display()
  ));
  retry_transient_network("cloning mirror", || {
    git_ops::run_dynamic(
      None,
      vec![
        "clone".into(),
        "--mirror".into(),
        remote_url.into(),
        mirror_dir.as_os_str().to_os_string(),
      ],
      "cloning mirror",
    )
    .map_err(map_git_error)
  })?;
  Ok(())
}

fn resolve_commit(mirror_dir: &Path, requested_rev: &str) -> Result<String, FetchError> {
  let rev = if requested_rev == "HEAD" {
    "HEAD".to_string()
  } else {
    format!("{requested_rev}^{{commit}}")
  };
  let output =
    match git_ops::run_capture(Some(mirror_dir), ["rev-parse", rev.as_str()], "resolving revision")
      .map_err(map_git_error)
    {
      Ok(output) => output,
      Err(err) if runtime_offline_enabled() => {
        return Err(FetchError::OfflineRevisionUnavailable {
          requested_rev: requested_rev.to_string(),
          mirror_dir: mirror_dir.to_path_buf(),
          source: Box::new(err),
        });
      },
      Err(err) => return Err(err),
    };
  Ok(output.trim().to_string())
}

#[derive(Debug, Clone)]
struct SemverTagSelection {
  resolved_tag: String,
  resolved_version: String,
  resolved_commit: String,
}

fn resolve_semver_tag(
  mirror_dir: &Path,
  package: &PackageId,
  version_req: &str,
) -> Result<SemverTagSelection, FetchError> {
  let requirement = VersionReq::parse(version_req).map_err(|source| {
    FetchError::InvalidVersionReq { requirement: version_req.to_string(), source }
  })?;
  let tags = list_tags(mirror_dir)?;

  let mut best: Option<(Version, String)> = None;
  for tag in tags {
    let Some(version) = parse_tag_semver(&tag) else {
      continue;
    };
    if !requirement.matches(&version) {
      continue;
    }
    match &best {
      Some((best_version, best_tag)) if (&version, &tag) <= (best_version, best_tag) => {},
      _ => best = Some((version, tag)),
    }
  }

  let Some((resolved_version, resolved_tag)) = best else {
    return Err(FetchError::VersionNotFound {
      package: package.to_string(),
      requested_requirement: version_req.to_string(),
      mirror_dir: mirror_dir.to_path_buf(),
    });
  };
  let resolved_commit = resolve_commit(mirror_dir, &resolved_tag)?;
  Ok(SemverTagSelection {
    resolved_tag,
    resolved_version: resolved_version.to_string(),
    resolved_commit,
  })
}

fn list_tags(mirror_dir: &Path) -> Result<Vec<String>, FetchError> {
  let raw = git_ops::run_capture(Some(mirror_dir), ["tag", "--list"], "listing tags")
    .map_err(map_git_error)?;
  Ok(raw.lines().map(str::trim).filter(|line| !line.is_empty()).map(ToOwned::to_owned).collect())
}

fn parse_tag_semver(tag: &str) -> Option<Version> {
  let normalized = tag.strip_prefix('v').unwrap_or(tag);
  Version::parse(normalized).ok()
}

fn materialize_checkout(
  mirror_dir: &Path,
  dest_dir: &Path,
  commit: &str,
  tmp_root: &Path,
) -> Result<(), FetchError> {
  if let Some(parent) = dest_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| FetchError::Io {
      action: "creating source cache parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }
  fs::create_dir_all(tmp_root).map_err(|source| FetchError::Io {
    action: "creating cache tmp dir".into(),
    path: tmp_root.to_path_buf(),
    source,
  })?;

  let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map(|dur| dur.as_nanos()).unwrap_or(0);
  let tmp_dir = tmp_root.join(format!(
    "{}-{}-{}-{}",
    dest_dir.file_name().and_then(|name| name.to_str()).unwrap_or("checkout"),
    std::process::id(),
    nonce,
    "tmp"
  ));

  if tmp_dir.exists() {
    fs_ops::remove_path_if_exists(&tmp_dir).map_err(|source| FetchError::Io {
      action: "cleaning temp checkout".into(),
      path: tmp_dir.clone(),
      source,
    })?;
  }

  let result = (|| {
    emit_progress(&format!(
      "Materializing source checkout `{commit}` into cache ({})",
      dest_dir.display()
    ));
    git_ops::run_dynamic(
      None,
      vec![
        "clone".into(),
        "--no-checkout".into(),
        mirror_dir.as_os_str().to_os_string(),
        tmp_dir.as_os_str().to_os_string(),
      ],
      "cloning cached checkout",
    )
    .map_err(map_git_error)?;
    git_ops::run(Some(&tmp_dir), ["checkout", "--detach", commit], "checking out resolved commit")
      .map_err(map_git_error)?;
    if dest_dir.exists() {
      fs_ops::remove_path_if_exists(dest_dir).map_err(|source| FetchError::Io {
        action: "removing stale destination checkout".into(),
        path: dest_dir.to_path_buf(),
        source,
      })?;
    }
    fs::rename(&tmp_dir, dest_dir).map_err(|source| FetchError::Io {
      action: "moving checkout into cache".into(),
      path: dest_dir.to_path_buf(),
      source,
    })?;
    Ok::<(), FetchError>(())
  })();

  if result.is_err() && tmp_dir.exists() {
    let _ = fs_ops::remove_path_if_exists(&tmp_dir);
  }
  result
}

fn source_locator_key(raw: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(raw.as_bytes());
  let digest = hasher.finalize();
  let hex = format!("{digest:x}");
  hex.chars().take(24).collect()
}

fn resolve_input_path(raw: &str) -> Result<PathBuf, FetchError> {
  let candidate = Path::new(raw);
  let path = if candidate.is_absolute() {
    candidate.to_path_buf()
  } else {
    std::env::current_dir().map_err(FetchError::Runtime)?.join(candidate)
  };
  Ok(path.canonicalize().unwrap_or(path))
}

fn normalize_sha256(raw: &str) -> Result<String, FetchError> {
  let normalized = raw.trim().to_ascii_lowercase();
  let valid = normalized.len() == 64 && normalized.chars().all(|ch| ch.is_ascii_hexdigit());
  if !valid {
    return Err(FetchError::InvalidChecksum {
      checksum: raw.to_string(),
      reason: "expected 64 lowercase/uppercase hex characters".to_string(),
    });
  }
  Ok(normalized)
}

fn materialize_archive_snapshot(
  package: &PackageId,
  source_url: &str,
  expected_sha256: &str,
  archive_file: &Path,
  dest_dir: &Path,
  tmp_root: &Path,
) -> Result<(), FetchError> {
  if let Some(parent) = archive_file.parent() {
    fs::create_dir_all(parent).map_err(|source| FetchError::Io {
      action: "creating archive cache parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }
  if let Some(parent) = dest_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| FetchError::Io {
      action: "creating archive source cache parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }
  fs::create_dir_all(tmp_root).map_err(|source| FetchError::Io {
    action: "creating cache tmp dir".into(),
    path: tmp_root.to_path_buf(),
    source,
  })?;

  if archive_file.exists() {
    let existing = hash_file_sha256(archive_file)?;
    if existing != expected_sha256 {
      if runtime_offline_enabled() {
        return Err(FetchError::ChecksumMismatch {
          package: package.to_string(),
          expected: expected_sha256.to_string(),
          actual: existing,
        });
      }
      fs::remove_file(archive_file).map_err(|source| FetchError::Io {
        action: "removing stale archive cache file".into(),
        path: archive_file.to_path_buf(),
        source,
      })?;
    }
  }

  if !archive_file.exists() {
    if runtime_offline_enabled() {
      return Err(FetchError::OfflineNetworkDisabled {
        action: format!("downloading archive for {}", package.as_str()),
        url: source_url.to_string(),
      });
    }
    download_archive_to_file(source_url, archive_file)?;
  }

  let actual_checksum = hash_file_sha256(archive_file)?;
  if actual_checksum != expected_sha256 {
    return Err(FetchError::ChecksumMismatch {
      package: package.to_string(),
      expected: expected_sha256.to_string(),
      actual: actual_checksum,
    });
  }

  let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map(|dur| dur.as_nanos()).unwrap_or(0);
  let tmp_extract =
    tmp_root.join(format!("archive-{}-{}-{nonce}", std::process::id(), package.repo()));
  if tmp_extract.exists() {
    fs_ops::remove_path_if_exists(&tmp_extract).map_err(|source| FetchError::Io {
      action: "cleaning archive temp directory".into(),
      path: tmp_extract.clone(),
      source,
    })?;
  }
  fs::create_dir_all(&tmp_extract).map_err(|source| FetchError::Io {
    action: "creating archive temp directory".into(),
    path: tmp_extract.clone(),
    source,
  })?;

  let result = (|| {
    emit_progress(&format!(
      "Extracting archive dependency `{}` into cache ({})",
      package.as_str(),
      dest_dir.display()
    ));
    extract_archive_file(archive_file, &tmp_extract)?;
    let extracted_root = collapse_archive_root(&tmp_extract)?;
    if dest_dir.exists() {
      fs_ops::remove_path_if_exists(dest_dir).map_err(|source| FetchError::Io {
        action: "removing stale extracted archive".into(),
        path: dest_dir.to_path_buf(),
        source,
      })?;
    }
    fs::rename(extracted_root, dest_dir).map_err(|source| FetchError::Io {
      action: "moving extracted archive into cache".into(),
      path: dest_dir.to_path_buf(),
      source,
    })?;
    Ok::<(), FetchError>(())
  })();

  if tmp_extract.exists() {
    let _ = fs_ops::remove_path_if_exists(&tmp_extract);
  }
  result
}

fn download_archive_to_file(source_url: &str, archive_file: &Path) -> Result<(), FetchError> {
  emit_progress(&format!(
    "Downloading archive source from `{source_url}` into {}",
    archive_file.display()
  ));
  if source_url.starts_with("file://") {
    let local = PathBuf::from(source_url.trim_start_matches("file://"));
    let bytes = fs::read(&local).map_err(|source| FetchError::Io {
      action: "reading local archive file".into(),
      path: local.clone(),
      source,
    })?;
    fs::write(archive_file, bytes).map_err(|source| FetchError::Io {
      action: "writing archive cache file".into(),
      path: archive_file.to_path_buf(),
      source,
    })?;
    return Ok(());
  }
  let local = Path::new(source_url);
  if local.exists() {
    let bytes = fs::read(local).map_err(|source| FetchError::Io {
      action: "reading local archive file".into(),
      path: local.to_path_buf(),
      source,
    })?;
    fs::write(archive_file, bytes).map_err(|source| FetchError::Io {
      action: "writing archive cache file".into(),
      path: archive_file.to_path_buf(),
      source,
    })?;
    return Ok(());
  }

  let response = reqwest::blocking::get(source_url)?.error_for_status()?;
  let bytes = response.bytes()?;
  fs::write(archive_file, &bytes).map_err(|source| FetchError::Io {
    action: "writing archive cache file".into(),
    path: archive_file.to_path_buf(),
    source,
  })?;
  Ok(())
}

fn extract_archive_file(archive_file: &Path, out_dir: &Path) -> Result<(), FetchError> {
  let name =
    archive_file.file_name().and_then(|v| v.to_str()).unwrap_or_default().to_ascii_lowercase();
  if !(name.ends_with(".tar.gz") || name.ends_with(".tgz")) {
    return Err(FetchError::UnsupportedArchiveFormat {
      archive: archive_file.to_path_buf(),
      expected: ".tar.gz or .tgz".to_string(),
    });
  }
  let file = fs::File::open(archive_file).map_err(|source| FetchError::Io {
    action: "opening archive cache file".into(),
    path: archive_file.to_path_buf(),
    source,
  })?;
  let decoder = flate2::read::GzDecoder::new(file);
  let mut archive = tar::Archive::new(decoder);
  archive.unpack(out_dir).map_err(|source| FetchError::Io {
    action: "extracting archive cache file".into(),
    path: out_dir.to_path_buf(),
    source,
  })
}

fn collapse_archive_root(extract_dir: &Path) -> Result<PathBuf, FetchError> {
  let mut entries = fs::read_dir(extract_dir)
    .map_err(|source| FetchError::Io {
      action: "scanning extracted archive root".into(),
      path: extract_dir.to_path_buf(),
      source,
    })?
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .collect::<Vec<_>>();
  entries.sort();
  if entries.len() == 1 && entries[0].is_dir() {
    Ok(entries.remove(0))
  } else {
    Ok(extract_dir.to_path_buf())
  }
}

fn mirror_path_dependency(
  source_dir: &Path,
  dest_dir: &Path,
  tmp_root: &Path,
) -> Result<(), FetchError> {
  if let Some(parent) = dest_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| FetchError::Io {
      action: "creating path source cache parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }
  fs::create_dir_all(tmp_root).map_err(|source| FetchError::Io {
    action: "creating cache tmp dir".into(),
    path: tmp_root.to_path_buf(),
    source,
  })?;

  let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map(|dur| dur.as_nanos()).unwrap_or(0);
  let tmp_dir = tmp_root.join(format!(
    "path-mirror-{}-{}-{nonce}",
    std::process::id(),
    source_dir.file_name().and_then(|v| v.to_str()).unwrap_or("src")
  ));
  if tmp_dir.exists() {
    fs_ops::remove_path_if_exists(&tmp_dir).map_err(|source| FetchError::Io {
      action: "cleaning path mirror temp dir".into(),
      path: tmp_dir.clone(),
      source,
    })?;
  }

  let result = (|| {
    copy_dir_recursive(source_dir, &tmp_dir)?;
    if dest_dir.exists() {
      fs_ops::remove_path_if_exists(dest_dir).map_err(|source| FetchError::Io {
        action: "removing stale path mirror".into(),
        path: dest_dir.to_path_buf(),
        source,
      })?;
    }
    fs::rename(&tmp_dir, dest_dir).map_err(|source| FetchError::Io {
      action: "moving path mirror into cache".into(),
      path: dest_dir.to_path_buf(),
      source,
    })?;
    Ok::<(), FetchError>(())
  })();

  if result.is_err() && tmp_dir.exists() {
    let _ = fs_ops::remove_path_if_exists(&tmp_dir);
  }
  result
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), FetchError> {
  fs::create_dir_all(dst).map_err(|source| FetchError::Io {
    action: "creating mirrored directory".into(),
    path: dst.to_path_buf(),
    source,
  })?;
  for entry in fs::read_dir(src).map_err(|source| FetchError::Io {
    action: "reading source directory".into(),
    path: src.to_path_buf(),
    source,
  })? {
    let entry = entry.map_err(|source| FetchError::Io {
      action: "reading source directory entry".into(),
      path: src.to_path_buf(),
      source,
    })?;
    let path = entry.path();
    let dest = dst.join(entry.file_name());
    let metadata = entry.metadata().map_err(|source| FetchError::Io {
      action: "reading source metadata".into(),
      path: path.clone(),
      source,
    })?;
    if metadata.is_dir() {
      copy_dir_recursive(&path, &dest)?;
    } else if metadata.is_file() {
      fs::copy(&path, &dest).map_err(|source| FetchError::Io {
        action: "copying source file".into(),
        path: dest.clone(),
        source,
      })?;
    }
  }
  Ok(())
}

fn hash_file_sha256(path: &Path) -> Result<String, FetchError> {
  let mut file = fs::File::open(path).map_err(|source| FetchError::Io {
    action: "opening file for hashing".into(),
    path: path.to_path_buf(),
    source,
  })?;
  let mut hasher = Sha256::new();
  let mut buf = [0u8; 64 * 1024];
  loop {
    let read = file.read(&mut buf).map_err(|source| FetchError::Io {
      action: "reading file for hashing".into(),
      path: path.to_path_buf(),
      source,
    })?;
    if read == 0 {
      break;
    }
    hasher.update(&buf[..read]);
  }
  Ok(format!("{:x}", hasher.finalize()))
}

fn hash_path_sha256(path: &Path) -> Result<String, FetchError> {
  let mut files = Vec::<String>::new();
  collect_files_recursive(path, path, &mut files)?;
  files.sort();
  let mut hasher = Sha256::new();
  for rel in files {
    let abs = path.join(&rel);
    let bytes = fs::read(&abs).map_err(|source| FetchError::Io {
      action: "reading source file for hashing".into(),
      path: abs.clone(),
      source,
    })?;
    hasher.update(rel.as_bytes());
    hasher.update([0u8]);
    hasher.update(bytes);
    hasher.update([0u8]);
  }
  Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files_recursive(
  root: &Path,
  current: &Path,
  out: &mut Vec<String>,
) -> Result<(), FetchError> {
  for entry in fs::read_dir(current).map_err(|source| FetchError::Io {
    action: "reading source directory for hashing".into(),
    path: current.to_path_buf(),
    source,
  })? {
    let entry = entry.map_err(|source| FetchError::Io {
      action: "reading directory entry for hashing".into(),
      path: current.to_path_buf(),
      source,
    })?;
    let path = entry.path();
    let metadata = entry.metadata().map_err(|source| FetchError::Io {
      action: "reading metadata for hashing".into(),
      path: path.clone(),
      source,
    })?;
    if metadata.is_dir() {
      collect_files_recursive(root, &path, out)?;
    } else if metadata.is_file() {
      let rel =
        path.strip_prefix(root).unwrap_or(path.as_path()).to_string_lossy().replace('\\', "/");
      out.push(rel);
    }
  }
  Ok(())
}

fn emit_progress(message: &str) {
  if runtime_progress_enabled() {
    progress_detail_tty(message);
  }
}

fn retry_transient_network<T, F>(action: &str, mut op: F) -> Result<T, FetchError>
where
  F: FnMut() -> Result<T, FetchError>,
{
  retry_transient_network_with_sleep(action, &mut op, std::thread::sleep)
}

fn retry_transient_network_with_sleep<T, F, S>(
  action: &str,
  op: &mut F,
  mut sleep_fn: S,
) -> Result<T, FetchError>
where
  F: FnMut() -> Result<T, FetchError>,
  S: FnMut(Duration),
{
  for attempt in 1..=TRANSIENT_RETRY_ATTEMPTS {
    match op() {
      Ok(value) => return Ok(value),
      Err(err) if err.is_transient_network() && attempt < TRANSIENT_RETRY_ATTEMPTS => {
        emit_progress(&format!(
          "Transient fetch failure while {action}; retrying ({attempt}/{TRANSIENT_RETRY_ATTEMPTS})"
        ));
        sleep_fn(transient_retry_delay(attempt));
      },
      Err(err) if err.is_transient_network() => {
        return Err(FetchError::TransientRetriesExhausted {
          action: action.to_string(),
          attempts: TRANSIENT_RETRY_ATTEMPTS,
          source: Box::new(err),
        });
      },
      Err(err) => return Err(err),
    }
  }

  unreachable!("retry loop must return on success or terminal failure")
}

fn transient_retry_delay(attempt: usize) -> Duration {
  let millis = match attempt {
    0 | 1 => 100,
    2 => 250,
    _ => 500,
  };
  Duration::from_millis(millis)
}

fn map_git_error(err: GitCommandError) -> FetchError {
  match err {
    GitCommandError::Spawn { action, source } => FetchError::SpawnGit { action, source },
    GitCommandError::Failed { action, status, stdout, stderr } => {
      FetchError::GitFailed { action, status, stdout, stderr }
    },
  }
}

#[derive(Debug, Error)]
pub enum FetchError {
  #[error(transparent)]
  GlobalCache(#[from] GlobalCacheError),
  #[error("http error while downloading archive: {0}")]
  Http(#[from] reqwest::Error),
  #[error("failed to run git while {action}: {source}")]
  SpawnGit {
    action: String,
    #[source]
    source: std::io::Error,
  },
  #[error("git failed while {action} (status {status:?})\nstdout: {stdout}\nstderr: {stderr}")]
  GitFailed { action: String, status: Option<i32>, stdout: String, stderr: String },
  #[error("filesystem error while {action} at `{path}`: {source}")]
  Io {
    action: String,
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("invalid sha256 checksum `{checksum}`: {reason}")]
  InvalidChecksum { checksum: String, reason: String },
  #[error("archive/source checksum mismatch for `{package}`: expected {expected}, actual {actual}")]
  ChecksumMismatch { package: String, expected: String, actual: String },
  #[error("unsupported archive format for `{archive}` (expected {expected})")]
  UnsupportedArchiveFormat { archive: PathBuf, expected: String },
  #[error("failed to create tokio runtime for parallel fetch: {0}")]
  Runtime(std::io::Error),
  #[error("{0}")]
  TaskJoin(String),
  #[error("invalid semver requirement `{requirement}`: {source}")]
  InvalidVersionReq {
    requirement: String,
    #[source]
    source: semver::Error,
  },
  #[error(
    "no git tag matching `{requested_requirement}` found for `{package}` in cached mirror `{}`",
    .mirror_dir.display()
  )]
  VersionNotFound { package: String, requested_requirement: String, mirror_dir: PathBuf },
  #[error(
    "offline mode requires a cached source mirror for `{package}` at rev `{requested_rev}` (missing `{}`)",
    .mirror_dir.display()
  )]
  OfflineCacheMiss { package: String, requested_rev: String, mirror_dir: PathBuf },
  #[error(
    "offline mode could not resolve rev `{requested_rev}` from cached mirror `{}`; refresh online first",
    .mirror_dir.display()
  )]
  OfflineRevisionUnavailable {
    requested_rev: String,
    mirror_dir: PathBuf,
    #[source]
    source: Box<FetchError>,
  },
  #[error("offline mode blocks {action} from `{url}`")]
  OfflineNetworkDisabled { action: String, url: String },
  #[error("transient network failures while {action} after {attempts} attempts: {source}")]
  TransientRetriesExhausted {
    action: String,
    attempts: usize,
    #[source]
    source: Box<FetchError>,
  },
}

impl FetchError {
  pub fn is_offline_cache_miss(&self) -> bool {
    matches!(self, Self::OfflineCacheMiss { .. } | Self::OfflineRevisionUnavailable { .. })
  }

  pub fn is_offline_network_disabled(&self) -> bool {
    matches!(self, Self::OfflineNetworkDisabled { .. })
  }

  pub fn is_invalid_version_requirement(&self) -> bool {
    matches!(self, Self::InvalidVersionReq { .. })
  }

  pub fn is_version_not_found(&self) -> bool {
    matches!(self, Self::VersionNotFound { .. })
  }

  pub fn is_invalid_checksum(&self) -> bool {
    matches!(self, Self::InvalidChecksum { .. })
  }

  pub fn is_checksum_mismatch(&self) -> bool {
    matches!(self, Self::ChecksumMismatch { .. })
  }

  pub fn is_unsupported_archive_format(&self) -> bool {
    matches!(self, Self::UnsupportedArchiveFormat { .. })
  }

  pub fn is_transient_network(&self) -> bool {
    match self {
      Self::Http(err) => err.is_timeout() || err.is_connect() || err.is_request(),
      Self::SpawnGit { source, .. } => matches!(
        source.kind(),
        std::io::ErrorKind::TimedOut
          | std::io::ErrorKind::ConnectionAborted
          | std::io::ErrorKind::ConnectionReset
          | std::io::ErrorKind::NotConnected
          | std::io::ErrorKind::Interrupted
          | std::io::ErrorKind::UnexpectedEof
          | std::io::ErrorKind::WouldBlock
          | std::io::ErrorKind::BrokenPipe
      ),
      Self::GitFailed { stderr, .. } => {
        let stderr = stderr.to_ascii_lowercase();
        [
          "connection reset",
          "timed out",
          "timeout",
          "could not resolve host",
          "failed to connect",
          "network is unreachable",
          "remote end hung up unexpectedly",
          "early eof",
          "connection was forcibly closed",
          "tls",
        ]
        .iter()
        .any(|needle| stderr.contains(needle))
      },
      Self::TransientRetriesExhausted { .. } => false,
      Self::OfflineCacheMiss { .. }
      | Self::OfflineRevisionUnavailable { .. }
      | Self::OfflineNetworkDisabled { .. }
      | Self::InvalidVersionReq { .. }
      | Self::VersionNotFound { .. }
      | Self::InvalidChecksum { .. }
      | Self::ChecksumMismatch { .. }
      | Self::UnsupportedArchiveFormat { .. }
      | Self::GlobalCache(_)
      | Self::Io { .. }
      | Self::Runtime(_)
      | Self::TaskJoin(_) => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{FetchError, parse_tag_semver, retry_transient_network_with_sleep};

  #[test]
  fn retry_policy_retries_transient_errors_and_succeeds() {
    let mut attempts = 0usize;
    let value = retry_transient_network_with_sleep(
      "fetching mirror",
      &mut || {
        attempts += 1;
        if attempts < 3 {
          Err(FetchError::GitFailed {
            action: "fetching mirror".to_string(),
            status: Some(128),
            stdout: String::new(),
            stderr: "fatal: Connection reset by peer".to_string(),
          })
        } else {
          Ok("ok")
        }
      },
      |_| {},
    )
    .expect("retry should succeed");

    assert_eq!(value, "ok");
    assert_eq!(attempts, 3);
  }

  #[test]
  fn retry_policy_does_not_retry_non_transient_errors() {
    let mut attempts = 0usize;
    let err = retry_transient_network_with_sleep(
      "cloning mirror",
      &mut || {
        attempts += 1;
        Err::<(), _>(FetchError::GitFailed {
          action: "cloning mirror".to_string(),
          status: Some(128),
          stdout: String::new(),
          stderr: "fatal: repository not found".to_string(),
        })
      },
      |_| {},
    )
    .expect_err("non-transient error should not retry");

    assert!(matches!(err, FetchError::GitFailed { .. }));
    assert_eq!(attempts, 1);
  }

  #[test]
  fn retry_policy_returns_stable_error_when_retries_exhausted() {
    let mut attempts = 0usize;
    let err = retry_transient_network_with_sleep(
      "fetching mirror",
      &mut || {
        attempts += 1;
        Err::<(), _>(FetchError::GitFailed {
          action: "fetching mirror".to_string(),
          status: Some(128),
          stdout: String::new(),
          stderr: "fatal: operation timed out".to_string(),
        })
      },
      |_| {},
    )
    .expect_err("expected retry exhaustion");

    match err {
      FetchError::TransientRetriesExhausted { attempts: got_attempts, .. } => {
        assert_eq!(got_attempts, 3)
      },
      other => panic!("unexpected error variant: {other}"),
    }
    assert_eq!(attempts, 3);
    assert!(err.to_string().contains("after 3 attempts"));
  }

  #[test]
  fn parses_semver_from_plain_and_v_prefixed_tags() {
    assert_eq!(parse_tag_semver("v1.2.3").map(|v| v.to_string()).as_deref(), Some("1.2.3"));
    assert_eq!(parse_tag_semver("2.0.0").map(|v| v.to_string()).as_deref(), Some("2.0.0"));
    assert!(parse_tag_semver("release-1.2.3").is_none());
  }
}
