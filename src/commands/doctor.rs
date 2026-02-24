use serde_json::json;
use std::env;
use std::path::PathBuf;

use crate::cli::DoctorArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::global_cache::GlobalCache;
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

  let overall_ok = toolchain_result.is_ok()
    && cache_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false)
    && recipes_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false)
    && git_status.ok;

  let toolchain_ok = toolchain_result.is_ok();
  let cache_ok = cache_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
  let recipes_ok = recipes_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
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
    }),
  ))
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
