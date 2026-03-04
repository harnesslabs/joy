use serde_json::{Value, json};
use std::env;
use std::path::PathBuf;

use crate::cli::DoctorArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::global_cache::GlobalCache;
use crate::output::HumanMessageBuilder;
use crate::project_probe;
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

  let probe = project_probe::probe(&cwd);
  if !probe.present {
    return json!({
      "ok": true,
      "project": {
        "present": false,
        "manifest_path": probe.manifest_path.display().to_string(),
      },
      "warnings": probe.warnings,
      "hints": probe.hints,
    });
  }

  let lockfile_json = if !probe.lockfile.present {
    json!({
      "present": false,
      "path": probe.lockfile.path.display().to_string(),
      "fresh": Value::Null,
      "package_count": Value::Null,
      "parse_error": Value::Null,
    })
  } else if let Some(parse_error) = &probe.lockfile.parse_error {
    json!({
      "present": true,
      "path": probe.lockfile.path.display().to_string(),
      "fresh": Value::Null,
      "package_count": Value::Null,
      "parse_error": parse_error,
    })
  } else {
    json!({
      "present": true,
      "path": probe.lockfile.path.display().to_string(),
      "fresh": probe.lockfile.fresh,
      "package_count": probe.lockfile.package_count,
      "parse_error": Value::Null,
    })
  };

  let dependency_metadata_json = if let Some(meta) = &probe.dependency_metadata {
    json!({
      "present": true,
      "package_count": meta.package_count,
      "metadata_source_counts": meta.metadata_source_counts,
      "declared_deps_source_counts": meta.declared_deps_source_counts,
      "package_manifest_count": meta.package_manifest_count,
      "registry_manifest_count": meta.registry_manifest_count,
    })
  } else {
    json!({ "present": false })
  };

  json!({
    "ok": probe.ok,
    "project": {
      "present": true,
      "manifest_path": probe.manifest_path.display().to_string(),
      "manifest_kind": probe.manifest_kind,
      "parse_error": probe.manifest_parse_error,
      "direct_dependency_count": probe.direct_dependency_count,
    },
    "artifacts": {
      "joy_root": probe.joy_root.display().to_string(),
      "state_dir": probe.state_dir.display().to_string(),
      "build_dir": probe.build_dir.display().to_string(),
      "dependency_graph": {
        "path": probe.dependency_graph.path.display().to_string(),
        "present": probe.dependency_graph.present,
        "parse_error": probe.dependency_graph.parse_error,
      },
      "root_compile_commands": {
        "path": probe.root_compile_commands.path.display().to_string(),
        "present": probe.root_compile_commands.present,
      },
      "target_compile_commands_count": probe.target_compile_commands.len(),
    },
    "lockfile": lockfile_json,
    "dependency_metadata": dependency_metadata_json,
    "warnings": probe.warnings,
    "hints": probe.hints,
  })
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
