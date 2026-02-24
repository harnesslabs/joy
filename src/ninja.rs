use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Build profile flags for Phase 4 local builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfile {
  Debug,
  Release,
}

impl BuildProfile {
  pub fn from_release_flag(release: bool) -> Self {
    if release { Self::Release } else { Self::Debug }
  }
}

/// Inputs required to generate a single-target Ninja build file.
#[derive(Debug, Clone)]
pub struct NinjaBuildSpec {
  pub compiler_executable: String,
  pub cpp_standard: String,
  pub source_file: PathBuf,
  pub object_file: PathBuf,
  pub binary_file: PathBuf,
  pub include_dirs: Vec<PathBuf>,
  pub link_dirs: Vec<PathBuf>,
  pub link_libs: Vec<String>,
  pub profile: BuildProfile,
}

/// Render a Ninja build file for a single `main.cpp` target.
pub fn render_build_ninja(spec: &NinjaBuildSpec) -> String {
  let cxxflags = build_cxxflags(spec);
  let ldflags = build_ldflags(spec);
  let source = path_to_ninja(&spec.source_file);
  let object = path_to_ninja(&spec.object_file);
  let binary = path_to_ninja(&spec.binary_file);
  let lines = [
    "ninja_required_version = 1.3".to_string(),
    format!("cxx = {}", spec.compiler_executable),
    format!("cxxflags = {cxxflags}"),
    format!("ldflags = {ldflags}"),
    String::new(),
    "rule cxx_compile".to_string(),
    "  command = $cxx $cxxflags -MMD -MF $out.d -c $in -o $out".to_string(),
    "  depfile = $out.d".to_string(),
    "  deps = gcc".to_string(),
    String::new(),
    "rule cxx_link".to_string(),
    "  command = $cxx $in -o $out $ldflags".to_string(),
    String::new(),
    format!("build {object}: cxx_compile {source}"),
    format!("build {binary}: cxx_link {object}"),
    format!("default {binary}"),
  ];
  format!("{}\n", lines.join("\n"))
}

/// Write the Ninja build file to disk, creating parent directories if needed.
pub fn write_build_ninja(path: &Path, spec: &NinjaBuildSpec) -> Result<(), NinjaError> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|source| NinjaError::Io {
      action: "creating ninja file parent directory".into(),
      path: parent.to_path_buf(),
      source,
    })?;
  }
  let contents = render_build_ninja(spec);
  fs::write(path, contents).map_err(|source| NinjaError::Io {
    action: "writing build.ninja".into(),
    path: path.to_path_buf(),
    source,
  })
}

fn build_cxxflags(spec: &NinjaBuildSpec) -> String {
  let mut parts = Vec::new();
  parts.push(format!("-std={}", spec.cpp_standard));
  match spec.profile {
    BuildProfile::Debug => {
      parts.push("-O0".into());
      parts.push("-g".into());
    },
    BuildProfile::Release => {
      parts.push("-O3".into());
      parts.push("-DNDEBUG".into());
    },
  }
  for include in &spec.include_dirs {
    parts.push(format!("-I{}", path_to_ninja(include)));
  }
  parts.join(" ")
}

fn build_ldflags(spec: &NinjaBuildSpec) -> String {
  let mut parts = Vec::new();
  for dir in &spec.link_dirs {
    parts.push(format!("-L{}", path_to_ninja(dir)));
  }
  for lib in &spec.link_libs {
    parts.push(format!("-l{lib}"));
  }
  parts.join(" ")
}

fn path_to_ninja(path: &Path) -> String {
  let raw = path.to_string_lossy().replace('\\', "/");
  let mut escaped = String::with_capacity(raw.len());
  for ch in raw.chars() {
    match ch {
      '$' => escaped.push_str("$$"),
      ' ' => escaped.push_str("$ "),
      ':' => escaped.push_str("$:"),
      _ => escaped.push(ch),
    }
  }
  escaped
}

#[derive(Debug, Error)]
pub enum NinjaError {
  #[error("filesystem error while {action} at `{path}`: {source}")]
  Io {
    action: String,
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::{BuildProfile, NinjaBuildSpec, render_build_ninja};

  #[test]
  fn renders_expected_ninja_file() {
    let spec = NinjaBuildSpec {
      compiler_executable: "clang++".into(),
      cpp_standard: "c++20".into(),
      source_file: PathBuf::from("src/main.cpp"),
      object_file: PathBuf::from(".joy/build/obj/main.o"),
      binary_file: PathBuf::from(".joy/bin/demo"),
      include_dirs: vec![
        PathBuf::from(".joy/include/deps/nlohmann_json"),
        PathBuf::from(".joy/include/deps/fmt_fmt"),
      ],
      link_dirs: vec![PathBuf::from(".joy/lib")],
      link_libs: vec!["fmt".into()],
      profile: BuildProfile::Debug,
    };

    let rendered = render_build_ninja(&spec);
    assert!(rendered.contains("cxx = clang++"));
    assert!(rendered.contains(
      "cxxflags = -std=c++20 -O0 -g -I.joy/include/deps/nlohmann_json -I.joy/include/deps/fmt_fmt"
    ));
    assert!(rendered.contains("ldflags = -L.joy/lib -lfmt"));
    assert!(rendered.contains("\n  command = $cxx $cxxflags -MMD -MF $out.d -c $in -o $out\n"));
    assert!(rendered.contains("build .joy/build/obj/main.o: cxx_compile src/main.cpp"));
    assert!(rendered.contains("build .joy/bin/demo: cxx_link .joy/build/obj/main.o"));
  }

  #[test]
  fn escapes_spaces_in_paths() {
    let spec = NinjaBuildSpec {
      compiler_executable: "clang++".into(),
      cpp_standard: "c++20".into(),
      source_file: PathBuf::from("src/main.cpp"),
      object_file: PathBuf::from(".joy/build/obj/main.o"),
      binary_file: PathBuf::from(".joy/bin/space demo"),
      include_dirs: vec![],
      link_dirs: vec![],
      link_libs: vec![],
      profile: BuildProfile::Debug,
    };

    let rendered = render_build_ninja(&spec);
    assert!(rendered.contains("build .joy/bin/space$ demo: cxx_link .joy/build/obj/main.o"));
  }
}
