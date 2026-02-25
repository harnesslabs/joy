use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::DoctorArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::global_cache::GlobalCache;
use crate::lockfile;
use crate::manifest::ManifestDocument;
use crate::output::HumanMessageBuilder;
use crate::recipes::RecipeStore;
use crate::toolchain;

pub fn handle(_args: DoctorArgs) -> Result<CommandOutput, JoyError> {
  let joy_home_env = env::var("JOY_HOME").ok();
  let path_env_present = env::var_os("PATH").is_some();
  let cwd = env::current_dir().ok().map(|p| p.display().to_string());

  let git_status = which_status("git");
  let cmake_status = which_status("cmake");
  let ninja_primary = which_status("ninja");
  let ninja_alt = which_status("ninja-build");
  let clang_status = which_status("clang++");
  let gcc_status = which_status("g++");
  let clang_win_status = which_status("clang++.exe");
  let gcc_win_status = which_status("g++.exe");
  let cl_status = which_status("cl.exe");

  let toolchain_result = toolchain::discover();
  let toolchain_json = match &toolchain_result {
    Ok(tc) => json!({
      "ok": true,
      "compiler_kind": tc.compiler.kind.as_str(),
      "compiler_version": tc.compiler.version,
      "compiler_path": tc.compiler.path.display().to_string(),
      "ninja_path": tc.ninja.path.display().to_string(),
    }),
    Err(err) => json!({
      "ok": false,
      "error": err.to_string(),
    }),
  };

  let cache_json = match GlobalCache::resolve() {
    Ok(cache) => {
      let ensure = cache.ensure_layout();
      json!({
        "ok": ensure.is_ok(),
        "root": cache.joy_home.display().to_string(),
        "cache_dir": cache.cache_root.display().to_string(),
        "ensure_layout_error": ensure.err().map(|e| e.to_string()),
      })
    },
    Err(err) => json!({
      "ok": false,
      "error": err.to_string(),
    }),
  };

  let recipes_json = match RecipeStore::load_default() {
    Ok(store) => json!({
      "ok": true,
      "root": store.root_dir().display().to_string(),
      "recipe_count": store.index().packages.len(),
    }),
    Err(err) => json!({
      "ok": false,
      "error": err.to_string(),
    }),
  };

  let project_json = inspect_project_context();

  let toolchain_ok = toolchain_result.is_ok();
  let cache_ok = cache_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
  let recipes_ok = recipes_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
  let project_ok = project_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(true);
  let overall_ok = toolchain_ok && cache_ok && recipes_ok && git_status.ok && project_ok;

  let mut human_builder =
    HumanMessageBuilder::new(if overall_ok { "Doctor OK" } else { "Doctor reported issues" })
      .line("Summary")
      .kv("cwd", cwd.clone().unwrap_or_else(|| "<unavailable>".to_string()))
      .kv("PATH set", path_env_present.to_string())
      .kv("JOY_HOME", joy_home_env.clone().unwrap_or_else(|| "<default>".to_string()))
      .kv("git", status_human(&git_status))
      .kv("toolchain", status_word(toolchain_ok))
      .kv("cache", status_word(cache_ok))
      .kv("recipes", status_word(recipes_ok))
      .kv("project", status_word(project_ok))
      .line("Tools")
      .kv("cmake", status_human(&cmake_status))
      .kv(
        "ninja",
        if ninja_primary.ok {
          status_human(&ninja_primary)
        } else {
          format!("{} (alternate: {})", status_human(&ninja_primary), status_human(&ninja_alt))
        },
      )
      .kv("clang++", status_human(&clang_status))
      .kv("g++", status_human(&gcc_status))
      .kv("cl.exe", status_human(&cl_status));

  if let Some(project) = project_json.get("project")
    && project.get("present").and_then(|v| v.as_bool()) == Some(true)
  {
    human_builder = human_builder
      .line("Project")
      .kv(
        "manifest",
        project.get("manifest_path").and_then(|v| v.as_str()).unwrap_or("<unknown>").to_string(),
      )
      .kv(
        "manifest kind",
        project.get("manifest_kind").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
      )
      .kv(
        "direct dependencies",
        project
          .get("direct_dependency_count")
          .and_then(|v| v.as_u64())
          .unwrap_or_default()
          .to_string(),
      );
    if let Some(artifacts) = project_json.get("artifacts") {
      human_builder = human_builder
        .line("Artifacts")
        .kv(
          "graph artifact",
          artifact_status_human(artifacts.get("dependency_graph").unwrap_or(&Value::Null)),
        )
        .kv(
          "root compile db",
          artifact_status_human(artifacts.get("root_compile_commands").unwrap_or(&Value::Null)),
        )
        .kv(
          "target compile db files",
          artifacts
            .get("target_compile_commands_count")
            .and_then(|v| v.as_u64())
            .unwrap_or_default()
            .to_string(),
        );
    }
    if let Some(dep_meta) = project_json.get("dependency_metadata")
      && dep_meta.get("present").and_then(|v| v.as_bool()) == Some(true)
    {
      human_builder = human_builder
        .line("Dependency Metadata")
        .kv(
          "packages",
          dep_meta.get("package_count").and_then(|v| v.as_u64()).unwrap_or_default().to_string(),
        )
        .kv(
          "recipe metadata",
          dep_meta
            .get("metadata_source_counts")
            .and_then(|v| v.get("recipe"))
            .and_then(|v| v.as_u64())
            .unwrap_or_default()
            .to_string(),
        )
        .kv(
          "package manifests",
          dep_meta
            .get("metadata_source_counts")
            .and_then(|v| v.get("package_manifest"))
            .and_then(|v| v.as_u64())
            .unwrap_or_default()
            .to_string(),
        )
        .kv(
          "registry summaries",
          dep_meta
            .get("metadata_source_counts")
            .and_then(|v| v.get("registry_manifest"))
            .and_then(|v| v.as_u64())
            .unwrap_or_default()
            .to_string(),
        )
        .kv(
          "metadata none",
          dep_meta
            .get("metadata_source_counts")
            .and_then(|v| v.get("none"))
            .and_then(|v| v.as_u64())
            .unwrap_or_default()
            .to_string(),
        );
    }
  }

  if !git_status.ok {
    human_builder =
      human_builder.warning("`git` is missing; dependency fetch and updates will fail");
  }
  if !toolchain_ok {
    human_builder = human_builder
      .warning("No working local C++ toolchain + ninja combination was discovered")
      .hint("Install a compiler and `ninja`, then rerun `joy doctor`");
  }
  if !cache_ok {
    let cache_err = cache_json
      .get("ensure_layout_error")
      .or_else(|| cache_json.get("error"))
      .and_then(|v| v.as_str())
      .unwrap_or("cache setup failed");
    human_builder = human_builder
      .warning(format!("Cache check failed: {cache_err}"))
      .hint("Check filesystem permissions and `JOY_HOME`, then rerun `joy doctor`");
  }
  if !recipes_ok {
    let recipes_err =
      recipes_json.get("error").and_then(|v| v.as_str()).unwrap_or("recipe load failed");
    human_builder = human_builder
      .warning(format!("Bundled recipes failed to load: {recipes_err}"))
      .hint("Reinstall `joy` or verify the bundled `recipes/` directory is intact");
  }

  if let Some(project_warnings) = project_json.get("warnings").and_then(|v| v.as_array()) {
    for warning in project_warnings {
      if let Some(msg) = warning.as_str() {
        human_builder = human_builder.warning(msg.to_string());
      }
    }
  }
  if let Some(project_hints) = project_json.get("hints").and_then(|v| v.as_array()) {
    for hint in project_hints {
      if let Some(msg) = hint.as_str() {
        human_builder = human_builder.hint(msg.to_string());
      }
    }
  }

  let human = human_builder.build();

  Ok(CommandOutput::new(
    "doctor",
    human,
    json!({
      "ok": overall_ok,
      "cwd": cwd,
      "env": {
        "path_present": path_env_present,
        "joy_home": joy_home_env,
      },
      "tools": {
        "git": tool_status_json(&git_status),
        "cmake": tool_status_json(&cmake_status),
        "ninja": tool_status_json(&ninja_primary),
        "ninja_build": tool_status_json(&ninja_alt),
        "clangxx": tool_status_json(&clang_status),
        "gxx": tool_status_json(&gcc_status),
        "clangxx_exe": tool_status_json(&clang_win_status),
        "gxx_exe": tool_status_json(&gcc_win_status),
        "cl_exe": tool_status_json(&cl_status),
      },
      "toolchain": toolchain_json,
      "cache": cache_json,
      "recipes": recipes_json,
      "project": project_json.get("project").cloned().unwrap_or(Value::Null),
      "artifacts": project_json.get("artifacts").cloned().unwrap_or(Value::Null),
      "lockfile": project_json.get("lockfile").cloned().unwrap_or(Value::Null),
      "dependency_metadata": project_json
        .get("dependency_metadata")
        .cloned()
        .unwrap_or(json!({"present": false})),
      "project_warnings": project_json.get("warnings").cloned().unwrap_or(json!([])),
      "project_hints": project_json.get("hints").cloned().unwrap_or(json!([])),
    }),
  ))
}

fn inspect_project_context() -> Value {
  let cwd = match env::current_dir() {
    Ok(cwd) => cwd,
    Err(_) => {
      return json!({
        "ok": true,
        "project": { "present": false },
        "warnings": [],
        "hints": [],
      });
    },
  };
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return json!({
      "ok": true,
      "project": { "present": false, "manifest_path": manifest_path.display().to_string() },
      "warnings": [],
      "hints": [],
    });
  }

  let mut warnings = Vec::<String>::new();
  let mut hints = Vec::<String>::new();

  let doc = match ManifestDocument::load(&manifest_path) {
    Ok(doc) => doc,
    Err(err) => {
      return json!({
        "ok": false,
        "project": {
          "present": true,
          "manifest_path": manifest_path.display().to_string(),
          "manifest_kind": "unknown",
          "parse_error": err.to_string(),
          "direct_dependency_count": 0,
        },
        "warnings": [format!("Project manifest parse failed: {err}")],
        "hints": ["Fix `joy.toml` parse/validation errors, then rerun `joy doctor`"],
      });
    },
  };

  let (manifest_kind, direct_dependency_count) = match &doc {
    ManifestDocument::Project(m) => ("project", m.dependencies.len()),
    ManifestDocument::Workspace(ws) => ("workspace", ws.workspace.members.len()),
    ManifestDocument::Package(pkg) => ("package", pkg.dependencies.len()),
  };

  let joy_root = cwd.join(".joy");
  let state_dir = joy_root.join("state");
  let build_dir = joy_root.join("build");
  let graph_path = state_dir.join("dependency-graph.json");
  let root_compile_db = cwd.join("compile_commands.json");
  let lockfile_path = cwd.join("joy.lock");

  let graph_parse_error = if graph_path.is_file() {
    match fs::read(&graph_path)
      .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(invalid_json_io))
    {
      Ok(_) => None,
      Err(err) => Some(err.to_string()),
    }
  } else {
    None
  };

  let target_compile_commands_count = if build_dir.is_dir() {
    fs::read_dir(&build_dir)
      .ok()
      .into_iter()
      .flatten()
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .filter(|path| {
        path
          .file_name()
          .and_then(|n| n.to_str())
          .is_some_and(|n| n.starts_with("compile_commands.") && n.ends_with(".json"))
      })
      .count()
  } else {
    0
  };

  let mut lockfile_ok = true;
  let mut lockfile_json = json!({
    "present": false,
    "path": lockfile_path.display().to_string(),
    "fresh": Value::Null,
    "package_count": Value::Null,
    "parse_error": Value::Null,
  });
  let mut dependency_metadata_json = json!({ "present": false });

  if lockfile_path.is_file() {
    match lockfile::Lockfile::load(&lockfile_path) {
      Ok(lock) => {
        let manifest_hash = lockfile::compute_manifest_hash(&manifest_path).ok();
        let fresh = manifest_hash.as_deref().is_some_and(|hash| hash == lock.manifest_hash);
        if !fresh {
          lockfile_ok = false;
          warnings.push("joy.lock is stale (manifest hash mismatch)".into());
          hints.push(
            "Run `joy sync --update-lock` to refresh lockfile and graph/editor artifacts".into(),
          );
        }

        let mut metadata_source_counts = std::collections::BTreeMap::<String, u64>::new();
        let mut declared_deps_source_counts = std::collections::BTreeMap::<String, u64>::new();
        for pkg in &lock.packages {
          *metadata_source_counts
            .entry(pkg.metadata_source.clone().unwrap_or_else(|| "unknown".into()))
            .or_default() += 1;
          *declared_deps_source_counts
            .entry(pkg.declared_deps_source.clone().unwrap_or_else(|| "unknown".into()))
            .or_default() += 1;
        }
        let missing_metadata = metadata_source_counts.get("none").copied().unwrap_or_default();
        if missing_metadata > 0 {
          warnings.push(format!(
            "{missing_metadata} locked package(s) have no package metadata provenance (`metadata_source = none`)"
          ));
          hints.push(
            "Nested dependency expansion may rely on recipes or registry summaries for those packages".into(),
          );
        }

        let package_manifest_count =
          metadata_source_counts.get("package_manifest").copied().unwrap_or_default();
        let registry_manifest_count =
          metadata_source_counts.get("registry_manifest").copied().unwrap_or_default();

        lockfile_json = json!({
          "present": true,
          "path": lockfile_path.display().to_string(),
          "fresh": fresh,
          "package_count": lock.packages.len(),
          "parse_error": Value::Null,
        });
        dependency_metadata_json = json!({
          "present": true,
          "package_count": lock.packages.len(),
          "metadata_source_counts": metadata_source_counts,
          "declared_deps_source_counts": declared_deps_source_counts,
          "package_manifest_count": package_manifest_count,
          "registry_manifest_count": registry_manifest_count,
        });
      },
      Err(err) => {
        lockfile_ok = false;
        lockfile_json = json!({
          "present": true,
          "path": lockfile_path.display().to_string(),
          "fresh": Value::Null,
          "package_count": Value::Null,
          "parse_error": err.to_string(),
        });
        warnings.push(format!("joy.lock parse failed: {err}"));
        hints.push("Regenerate lockfile with `joy sync --update-lock`".into());
      },
    }
  }

  let root_compile_db_present = root_compile_db.is_file();
  let graph_present = graph_path.is_file();
  if manifest_kind == "project" && direct_dependency_count > 0 {
    if !graph_present {
      warnings
        .push("Dependency graph artifact is missing (`.joy/state/dependency-graph.json`)".into());
      hints.push("Run `joy sync` or `joy build` to materialize dependency state".into());
    }
    if !root_compile_db_present {
      warnings.push(
        "Root `compile_commands.json` is missing; editors may not resolve dependency includes"
          .into(),
      );
      hints.push(
        "Run `joy sync` or `joy build`; if a toolchain is missing, install a compiler + `ninja` so compile DB generation can run".into(),
      );
    }
  }
  if graph_parse_error.is_some() {
    warnings.push("Dependency graph artifact exists but failed to parse as JSON".into());
    hints.push("Rerun `joy sync` to regenerate `.joy/state/dependency-graph.json`".into());
  }

  let project_ok = lockfile_ok && graph_parse_error.is_none();
  json!({
    "ok": project_ok,
    "project": {
      "present": true,
      "manifest_path": manifest_path.display().to_string(),
      "manifest_kind": manifest_kind,
      "direct_dependency_count": direct_dependency_count,
    },
    "artifacts": {
      "joy_root": joy_root.display().to_string(),
      "state_dir": state_dir.display().to_string(),
      "build_dir": build_dir.display().to_string(),
      "dependency_graph": {
        "path": graph_path.display().to_string(),
        "present": graph_present,
        "parse_error": graph_parse_error,
      },
      "root_compile_commands": {
        "path": root_compile_db.display().to_string(),
        "present": root_compile_db_present,
      },
      "target_compile_commands_count": target_compile_commands_count,
    },
    "lockfile": lockfile_json,
    "dependency_metadata": dependency_metadata_json,
    "warnings": warnings,
    "hints": hints,
  })
}

fn invalid_json_io(err: serde_json::Error) -> std::io::Error {
  std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid json: {err}"))
}

fn artifact_status_human(value: &Value) -> String {
  let present = value.get("present").and_then(|v| v.as_bool()).unwrap_or(false);
  if !present {
    return "missing".into();
  }
  if let Some(err) = value.get("parse_error").and_then(|v| v.as_str()) {
    return format!("present (parse error: {err})");
  }
  "present".into()
}

#[derive(Debug, Clone)]
struct ToolStatus {
  program: &'static str,
  ok: bool,
  path: Option<PathBuf>,
}

fn which_status(program: &'static str) -> ToolStatus {
  match which::which(program) {
    Ok(path) => ToolStatus { program, ok: true, path: Some(path) },
    Err(_) => ToolStatus { program, ok: false, path: None },
  }
}

fn tool_status_json(status: &ToolStatus) -> serde_json::Value {
  json!({
    "program": status.program,
    "ok": status.ok,
    "path": status.path.as_ref().map(|p| p.display().to_string()),
  })
}

fn status_human(status: &ToolStatus) -> String {
  if status.ok {
    format!("ok ({})", status.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default())
  } else {
    "missing".to_string()
  }
}

fn status_word(ok: bool) -> &'static str {
  if ok { "ok" } else { "issue" }
}

#[allow(dead_code)]
fn _path_exists(path: &Path) -> bool {
  path.exists()
}
