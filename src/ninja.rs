use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::toolchain::CompilerKind;

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
  pub compiler_kind: CompilerKind,
  pub compiler_executable: String,
  pub cpp_standard: String,
  pub compile_units: Vec<NinjaCompileUnit>,
  pub binary_file: PathBuf,
  pub include_dirs: Vec<PathBuf>,
  pub link_dirs: Vec<PathBuf>,
  pub link_libs: Vec<String>,
  pub profile: BuildProfile,
}

/// A single source/object compilation edge in the generated Ninja graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NinjaCompileUnit {
  pub source_file: PathBuf,
  pub object_file: PathBuf,
}

/// Render a Ninja build file for a single `main.cpp` target.
pub fn render_build_ninja(spec: &NinjaBuildSpec) -> String {
  let cxxflags = build_cxxflags(spec);
  let ldflags = build_ldflags(spec);
  let binary = path_to_ninja(&spec.binary_file);
  let (compile_command, depfile_line, deps_line, msvc_deps_prefix_line, link_command) =
    match spec.compiler_kind {
      CompilerKind::Msvc => (
        "  command = $cxx $cxxflags /showIncludes /c $in /Fo$out".to_string(),
        None,
        Some("  deps = msvc".to_string()),
        Some("  msvc_deps_prefix = Note: including file:".to_string()),
        "  command = $cxx $in /Fe$out /link $ldflags".to_string(),
      ),
      CompilerKind::Clang | CompilerKind::Gcc => (
        "  command = $cxx $cxxflags -MMD -MF $out.d -c $in -o $out".to_string(),
        Some("  depfile = $out.d".to_string()),
        Some("  deps = gcc".to_string()),
        None,
        "  command = $cxx $in -o $out $ldflags".to_string(),
      ),
    };
  let mut lines = vec![
    "ninja_required_version = 1.3".to_string(),
    format!("cxx = {}", spec.compiler_executable),
    format!("cxxflags = {cxxflags}"),
    format!("ldflags = {ldflags}"),
    String::new(),
    "rule cxx_compile".to_string(),
    compile_command,
  ];
  if let Some(depfile) = depfile_line {
    lines.push(depfile);
  }
  if let Some(deps) = deps_line {
    lines.push(deps);
  }
  if let Some(msvc_prefix) = msvc_deps_prefix_line {
    lines.push(msvc_prefix);
  }
  lines.push(String::new());
  lines.push("rule cxx_link".to_string());
  lines.push(link_command);
  lines.push(String::new());

  let mut link_inputs = Vec::new();
  for unit in &spec.compile_units {
    let source = path_to_ninja(&unit.source_file);
    let object = path_to_ninja(&unit.object_file);
    link_inputs.push(object.clone());
    lines.push(format!("build {object}: cxx_compile {source}"));
  }

  let link_inputs = link_inputs.join(" ");
  lines.push(format!("build {binary}: cxx_link {link_inputs}"));
  lines.push(format!("default {binary}"));

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
  match spec.compiler_kind {
    CompilerKind::Msvc => {
      parts.push("/nologo".into());
      parts.push("/EHsc".into());
      parts.push(msvc_cpp_standard_flag(&spec.cpp_standard));
      match spec.profile {
        BuildProfile::Debug => {
          parts.push("/Od".into());
          parts.push("/Zi".into());
        },
        BuildProfile::Release => {
          parts.push("/O2".into());
          parts.push("/DNDEBUG".into());
        },
      }
      for include in &spec.include_dirs {
        parts.push(format!("/I{}", path_to_ninja(include)));
      }
    },
    CompilerKind::Clang | CompilerKind::Gcc => {
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
    },
  }
  parts.join(" ")
}

fn build_ldflags(spec: &NinjaBuildSpec) -> String {
  let mut parts = Vec::new();
  match spec.compiler_kind {
    CompilerKind::Msvc => {
      parts.push("/nologo".into());
      for dir in &spec.link_dirs {
        parts.push(format!("/LIBPATH:{}", path_to_ninja(dir)));
      }
      for lib in &spec.link_libs {
        parts.push(msvc_link_lib_name(lib));
      }
    },
    CompilerKind::Clang | CompilerKind::Gcc => {
      for dir in &spec.link_dirs {
        parts.push(format!("-L{}", path_to_ninja(dir)));
      }
      for lib in &spec.link_libs {
        parts.push(format!("-l{lib}"));
      }
    },
  }
  parts.join(" ")
}

fn msvc_cpp_standard_flag(cpp_standard: &str) -> String {
  format!("/std:{cpp_standard}")
}

fn msvc_link_lib_name(lib: &str) -> String {
  if lib.to_ascii_lowercase().ends_with(".lib") { lib.to_string() } else { format!("{lib}.lib") }
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

  use crate::toolchain::CompilerKind;

  use super::{BuildProfile, NinjaBuildSpec, NinjaCompileUnit, render_build_ninja};

  #[test]
  fn renders_expected_ninja_file() {
    let spec = NinjaBuildSpec {
      compiler_kind: CompilerKind::Clang,
      compiler_executable: "clang++".into(),
      cpp_standard: "c++20".into(),
      compile_units: vec![
        NinjaCompileUnit {
          source_file: PathBuf::from("src/main.cpp"),
          object_file: PathBuf::from(".joy/build/obj/main-a1b2c3.o"),
        },
        NinjaCompileUnit {
          source_file: PathBuf::from("src/dup/main.cpp"),
          object_file: PathBuf::from(".joy/build/obj/main-d4e5f6.o"),
        },
      ],
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
    assert!(rendered.contains("build .joy/build/obj/main-a1b2c3.o: cxx_compile src/main.cpp"));
    assert!(rendered.contains("build .joy/build/obj/main-d4e5f6.o: cxx_compile src/dup/main.cpp"));
    assert!(rendered.contains(
      "build .joy/bin/demo: cxx_link .joy/build/obj/main-a1b2c3.o .joy/build/obj/main-d4e5f6.o"
    ));
  }

  #[test]
  fn escapes_spaces_in_paths() {
    let spec = NinjaBuildSpec {
      compiler_kind: CompilerKind::Clang,
      compiler_executable: "clang++".into(),
      cpp_standard: "c++20".into(),
      compile_units: vec![NinjaCompileUnit {
        source_file: PathBuf::from("src/main.cpp"),
        object_file: PathBuf::from(".joy/build/obj/main.o"),
      }],
      binary_file: PathBuf::from(".joy/bin/space demo"),
      include_dirs: vec![],
      link_dirs: vec![],
      link_libs: vec![],
      profile: BuildProfile::Debug,
    };

    let rendered = render_build_ninja(&spec);
    assert!(rendered.contains("build .joy/bin/space$ demo: cxx_link .joy/build/obj/main.o"));
  }

  #[test]
  fn renders_msvc_compile_and_link_rules() {
    let spec = NinjaBuildSpec {
      compiler_kind: CompilerKind::Msvc,
      compiler_executable: "cl.exe".into(),
      cpp_standard: "c++20".into(),
      compile_units: vec![NinjaCompileUnit {
        source_file: PathBuf::from("src/main.cpp"),
        object_file: PathBuf::from(".joy/build/obj/main-abc123.o"),
      }],
      binary_file: PathBuf::from(".joy/bin/demo.exe"),
      include_dirs: vec![PathBuf::from(".joy/include/deps/fmt_fmt")],
      link_dirs: vec![PathBuf::from(".joy/lib")],
      link_libs: vec!["fmt".into(), "user32.lib".into()],
      profile: BuildProfile::Release,
    };

    let rendered = render_build_ninja(&spec);
    assert!(rendered.contains("cxx = cl.exe"));
    assert!(
      rendered
        .contains("cxxflags = /nologo /EHsc /std:c++20 /O2 /DNDEBUG /I.joy/include/deps/fmt_fmt")
    );
    assert!(rendered.contains("ldflags = /nologo /LIBPATH:.joy/lib fmt.lib user32.lib"));
    assert!(rendered.contains("\n  command = $cxx $cxxflags /showIncludes /c $in /Fo$out\n"));
    assert!(rendered.contains("\n  deps = msvc\n"));
    assert!(rendered.contains("\n  msvc_deps_prefix = Note: including file:\n"));
    assert!(rendered.contains("\n  command = $cxx $in /Fe$out /link $ldflags\n"));
  }
}
