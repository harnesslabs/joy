use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::global_cache::{GlobalCache, GlobalCacheError};
use crate::package_id::PackageId;

#[derive(Debug, Clone)]
pub struct FetchResult {
  pub package: PackageId,
  pub requested_rev: String,
  pub resolved_commit: String,
  pub source_dir: PathBuf,
  pub remote_url: String,
  pub cache_hit: bool,
}

pub fn fetch_github(package: &PackageId, requested_rev: &str) -> Result<FetchResult, FetchError> {
  let cache = GlobalCache::resolve()?;
  fetch_github_with_cache(package, requested_rev, &cache)
}

pub fn fetch_github_with_cache(
  package: &PackageId,
  requested_rev: &str,
  cache: &GlobalCache,
) -> Result<FetchResult, FetchError> {
  cache.ensure_layout()?;

  let remote_url = github_remote_url(package);
  let mirror_dir = cache.git_mirror_dir(package);
  ensure_mirror(&remote_url, &mirror_dir)?;

  let requested = if requested_rev.trim().is_empty() { "HEAD" } else { requested_rev };
  let resolved_commit = resolve_commit(&mirror_dir, requested)?;

  let source_dir = cache.source_checkout_dir(package, &resolved_commit);
  let cache_hit = source_dir.exists();
  if !cache_hit {
    materialize_checkout(&mirror_dir, &source_dir, &resolved_commit, cache.tmp_dir())?;
  }

  Ok(FetchResult {
    package: package.clone(),
    requested_rev: requested.to_string(),
    resolved_commit,
    source_dir,
    remote_url,
    cache_hit,
  })
}

pub fn download_and_extract_tar_gz(url: &str, dest_dir: &Path) -> Result<(), FetchError> {
  if let Some(parent) = dest_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| FetchError::Io {
      action: "creating archive extraction parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }

  if !dest_dir.exists() {
    fs::create_dir_all(dest_dir).map_err(|source| FetchError::Io {
      action: "creating archive extraction destination".into(),
      path: dest_dir.to_path_buf(),
      source,
    })?;
  }

  let client = reqwest::blocking::Client::new();
  let response = client
    .get(url)
    .send()
    .and_then(reqwest::blocking::Response::error_for_status)
    .map_err(FetchError::Http)?;

  extract_tar_gz_reader(response, dest_dir)
}

pub fn prefetch_github_packages(
  requests: Vec<(PackageId, String)>,
) -> Result<Vec<FetchResult>, FetchError> {
  if requests.is_empty() {
    return Ok(Vec::new());
  }

  let concurrency = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).max(1);
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(concurrency)
    .enable_all()
    .build()
    .map_err(FetchError::Runtime)?;

  runtime.block_on(async move {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let total = requests.len();
    let mut join_set = tokio::task::JoinSet::new();

    for (index, (package, rev)) in requests.into_iter().enumerate() {
      let semaphore = semaphore.clone();
      join_set.spawn(async move {
        let permit = semaphore
          .acquire_owned()
          .await
          .map_err(|_| FetchError::TaskJoin("prefetch semaphore closed".into()))?;
        let _permit = permit;
        tokio::task::spawn_blocking(move || fetch_github(&package, &rev))
          .await
          .map_err(|err| FetchError::TaskJoin(format!("prefetch task join failed: {err}")))?
          .map(|result| (index, result))
      });
    }

    let mut results: Vec<Option<FetchResult>> = vec![None; total];
    while let Some(task_result) = join_set.join_next().await {
      let (index, fetched) = task_result
        .map_err(|err| FetchError::TaskJoin(format!("prefetch task failed: {err}")))??;
      results[index] = Some(fetched);
    }

    results
      .into_iter()
      .map(|item| item.ok_or_else(|| FetchError::TaskJoin("missing prefetch result".into())))
      .collect::<Result<Vec<_>, _>>()
  })
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

fn extract_tar_gz_reader(reader: impl Read, dest_dir: &Path) -> Result<(), FetchError> {
  let decoder = flate2::read::GzDecoder::new(reader);
  let mut archive = tar::Archive::new(decoder);
  archive.unpack(dest_dir).map_err(|source| FetchError::Io {
    action: "extracting tar.gz archive".into(),
    path: dest_dir.to_path_buf(),
    source,
  })
}

fn ensure_mirror(remote_url: &str, mirror_dir: &Path) -> Result<(), FetchError> {
  if mirror_dir.exists() {
    run_git(Some(mirror_dir), ["fetch", "--all", "--tags", "--prune"], "fetching mirror")?;
    return Ok(());
  }

  if let Some(parent) = mirror_dir.parent() {
    fs::create_dir_all(parent).map_err(|source| FetchError::Io {
      action: "creating mirror parent".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }

  run_git_dynamic(
    None,
    vec![
      "clone".into(),
      "--mirror".into(),
      remote_url.into(),
      mirror_dir.as_os_str().to_os_string(),
    ],
    "cloning mirror",
  )?;
  Ok(())
}

fn resolve_commit(mirror_dir: &Path, requested_rev: &str) -> Result<String, FetchError> {
  let rev = if requested_rev == "HEAD" {
    "HEAD".to_string()
  } else {
    format!("{requested_rev}^{{commit}}")
  };
  let output =
    run_git_capture(Some(mirror_dir), ["rev-parse", rev.as_str()], "resolving revision")?;
  Ok(output.trim().to_string())
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
    remove_any(&tmp_dir).map_err(|source| FetchError::Io {
      action: "cleaning temp checkout".into(),
      path: tmp_dir.clone(),
      source,
    })?;
  }

  let result = (|| {
    run_git_dynamic(
      None,
      vec![
        "clone".into(),
        "--no-checkout".into(),
        mirror_dir.as_os_str().to_os_string(),
        tmp_dir.as_os_str().to_os_string(),
      ],
      "cloning cached checkout",
    )?;
    run_git(Some(&tmp_dir), ["checkout", "--detach", commit], "checking out resolved commit")?;
    if dest_dir.exists() {
      remove_any(dest_dir).map_err(|source| FetchError::Io {
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
    let _ = remove_any(&tmp_dir);
  }
  result
}

fn run_git_dynamic(
  cwd: Option<&Path>,
  args: Vec<OsString>,
  action: &str,
) -> Result<(), FetchError> {
  let output = git_output(cwd, args, action)?;
  if output.status.success() { Ok(()) } else { Err(git_failed_error(action, &output)) }
}

fn remove_any(path: &Path) -> std::io::Result<()> {
  match fs::symlink_metadata(path) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)
      } else if metadata.is_dir() {
        fs::remove_dir_all(path)
      } else {
        fs::remove_file(path)
      }
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(err) => Err(err),
  }
}

fn run_git<const N: usize>(
  cwd: Option<&Path>,
  args: [&str; N],
  action: &str,
) -> Result<(), FetchError> {
  let output = git_output(cwd, args.into_iter().map(OsString::from).collect(), action)?;
  if output.status.success() { Ok(()) } else { Err(git_failed_error(action, &output)) }
}

fn run_git_capture<const N: usize>(
  cwd: Option<&Path>,
  args: [&str; N],
  action: &str,
) -> Result<String, FetchError> {
  let output = git_output(cwd, args.into_iter().map(OsString::from).collect(), action)?;
  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(git_failed_error(action, &output))
  }
}

fn git_output(
  cwd: Option<&Path>,
  args: Vec<OsString>,
  action: &str,
) -> Result<std::process::Output, FetchError> {
  let mut cmd = Command::new("git");
  if let Some(dir) = cwd {
    cmd.arg("-C").arg(dir);
  }
  cmd.args(args);
  cmd.output().map_err(|source| FetchError::SpawnGit { action: action.into(), source })
}

fn git_failed_error(action: &str, output: &std::process::Output) -> FetchError {
  FetchError::GitFailed {
    action: action.into(),
    status: output.status.code(),
    stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
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
  #[error("failed to create tokio runtime for parallel fetch: {0}")]
  Runtime(std::io::Error),
  #[error("{0}")]
  TaskJoin(String),
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use flate2::Compression;
  use flate2::write::GzEncoder;
  use tempfile::TempDir;

  use super::download_and_extract_tar_gz;

  #[test]
  fn downloads_and_extracts_tar_gz_from_mock_http_server() {
    let mut server = mockito::Server::new();
    let archive_bytes = build_fixture_tar_gz();

    let mock = server.mock("GET", "/pkg.tar.gz").with_status(200).with_body(archive_bytes).create();

    let temp = TempDir::new().expect("tempdir");
    let url = format!("{}/pkg.tar.gz", server.url());
    download_and_extract_tar_gz(&url, temp.path()).expect("download+extract");

    mock.assert();
    assert!(temp.path().join("fixture").join("include").join("demo.hpp").is_file());
    assert!(temp.path().join("fixture").join("README.md").is_file());
  }

  fn build_fixture_tar_gz() -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    {
      let mut builder = tar::Builder::new(&mut tar_bytes);
      let files = [
        ("fixture/include/demo.hpp", b"// demo header\n".as_slice()),
        ("fixture/README.md", b"# fixture\n".as_slice()),
      ];

      for (path, contents) in files {
        let mut header = tar::Header::new_gnu();
        header.set_path(path).expect("tar path");
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, contents).expect("append tar entry");
      }
      builder.finish().expect("finish tar");
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_bytes).expect("write gzip payload");
    encoder.finish().expect("finish gzip")
  }
}
