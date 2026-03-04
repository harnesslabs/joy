use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Output};
use thiserror::Error;

/// Host C++ compiler family used for local builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerKind {
  Clang,
  Gcc,
  Msvc,
}

impl CompilerKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Clang => "clang",
      Self::Gcc => "gcc",
      Self::Msvc => "msvc",
    }
  }
}

/// Discovered C++ compiler executable and parsed version info.
#[derive(Debug, Clone)]
pub struct Compiler {
  pub kind: CompilerKind,
  pub executable_name: String,
  pub path: PathBuf,
  pub version: String,
}

/// Discovered Ninja executable.
#[derive(Debug, Clone)]
pub struct Ninja {
  pub path: PathBuf,
}

/// All required host build tools for Phase 4 local builds.
#[derive(Debug, Clone)]
pub struct Toolchain {
  pub compiler: Compiler,
  pub ninja: Ninja,
}

/// Discover Ninja and a supported C++ compiler on PATH.
pub fn discover() -> Result<Toolchain, ToolchainError> {
  let ninja = discover_ninja()?;
  let compiler = discover_compiler()?;
  Ok(Toolchain { compiler, ninja })
}

/// Discover a Ninja executable (`ninja`, then `ninja-build`).
pub fn discover_ninja() -> Result<Ninja, ToolchainError> {
  for candidate in ["ninja", "ninja-build"] {
    if let Ok(path) = which::which(candidate) {
      return Ok(Ninja { path });
    }
  }
  Err(ToolchainError::NinjaNotFound)
}

/// Discover a supported C++ compiler using the roadmap priority order.
pub fn discover_compiler() -> Result<Compiler, ToolchainError> {
  let candidates: &[(&str, CompilerKind)] = if cfg!(windows) {
    &[
      ("g++.exe", CompilerKind::Gcc),
      ("clang++.exe", CompilerKind::Clang),
      ("cl.exe", CompilerKind::Msvc),
    ]
  } else {
    &[("clang++", CompilerKind::Clang), ("g++", CompilerKind::Gcc)]
  };

  for (name, kind) in candidates {
    let Ok(path) = which::which(name) else {
      continue;
    };
    let version = probe_compiler_version(*kind, &path)?;
    return Ok(Compiler { kind: *kind, executable_name: (*name).to_string(), path, version });
  }

  Err(ToolchainError::CompilerNotFound)
}

fn probe_compiler_version(
  kind: CompilerKind,
  path: &std::path::Path,
) -> Result<String, ToolchainError> {
  match kind {
    CompilerKind::Msvc => {
      // `cl.exe` prints its banner/version to stderr and may exit non-zero when invoked without
      // input files. Treat a parsable banner as a successful probe.
      let output = run_capture_output(path, Vec::<OsString>::new())?;
      let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
      );
      if let Some(version) = parse_compiler_version(kind, &combined) {
        return Ok(version);
      }
      if output.status.success() {
        return Ok("unknown".to_string());
      }
      Err(ToolchainError::CommandFailed {
        program: path.display().to_string(),
        status: output.status.code(),
        stderr: combined.trim().to_string(),
      })
    },
    CompilerKind::Clang | CompilerKind::Gcc => {
      let version_output = run_capture(path, [OsString::from("--version")])?;
      Ok(parse_compiler_version(kind, &version_output).unwrap_or_else(|| "unknown".to_string()))
    },
  }
}

/// Parse compiler version text into a concise version string for diagnostics.
pub fn parse_compiler_version(kind: CompilerKind, text: &str) -> Option<String> {
  match kind {
    CompilerKind::Clang => parse_after_marker(text, "clang version "),
    CompilerKind::Gcc => parse_gcc_version(text),
    CompilerKind::Msvc => parse_after_marker(text, "Version "),
  }
}

fn parse_after_marker(text: &str, marker: &str) -> Option<String> {
  text
    .lines()
    .find_map(|line| line.split_once(marker).map(|(_, tail)| tail))
    .and_then(|tail| tail.split_whitespace().next())
    .map(ToString::to_string)
}

fn parse_gcc_version(text: &str) -> Option<String> {
  for line in text.lines() {
    let Some(token) = line.split_whitespace().last() else {
      continue;
    };
    if token.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
      && token.chars().any(|ch| ch.is_ascii_digit())
    {
      return Some(token.to_string());
    }
  }
  None
}

fn run_capture<const N: usize>(
  path: &std::path::Path,
  args: [OsString; N],
) -> Result<String, ToolchainError> {
  let output = run_capture_output(path, args)?;
  if !output.status.success() {
    return Err(ToolchainError::CommandFailed {
      program: path.display().to_string(),
      status: output.status.code(),
      stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    });
  }
  Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_capture_output(
  path: &std::path::Path,
  args: impl IntoIterator<Item = OsString>,
) -> Result<Output, ToolchainError> {
  Command::new(path)
    .args(args)
    .output()
    .map_err(|source| ToolchainError::Spawn { program: path.display().to_string(), source })
}

#[derive(Debug, Error)]
pub enum ToolchainError {
  #[error("ninja executable not found on PATH (looked for `ninja` and `ninja-build`)")]
  NinjaNotFound,
  #[error("no supported C++ compiler found on PATH (looked for clang++/g++ and cl.exe on Windows)")]
  CompilerNotFound,
  #[error("failed to spawn `{program}`: {source}")]
  Spawn {
    program: String,
    #[source]
    source: std::io::Error,
  },
  #[error("`{program}` failed (status {status:?}): {stderr}")]
  CommandFailed { program: String, status: Option<i32>, stderr: String },
}

#[cfg(test)]
mod tests {
  use super::{CompilerKind, parse_compiler_version};

  #[test]
  fn parses_clang_version_output() {
    let text = "Apple clang version 16.0.0 (clang-1600.0.26.6)\nTarget: arm64-apple-darwin";
    let parsed = parse_compiler_version(CompilerKind::Clang, text).expect("clang version");
    assert_eq!(parsed, "16.0.0");
  }

  #[test]
  fn parses_gcc_version_output() {
    let text = "g++ (Homebrew GCC 14.2.0) 14.2.0\nCopyright (C) ...";
    let parsed = parse_compiler_version(CompilerKind::Gcc, text).expect("gcc version");
    assert_eq!(parsed, "14.2.0");
  }

  #[test]
  fn parses_msvc_version_output_with_crlf_banner() {
    let text = "Microsoft (R) C/C++ Optimizing Compiler Version 19.38.33135 for x64\r\nCopyright (C) Microsoft Corporation.  All rights reserved.\r\n";
    let parsed = parse_compiler_version(CompilerKind::Msvc, text).expect("msvc version");
    assert_eq!(parsed, "19.38.33135");
  }
}
