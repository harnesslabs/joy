pub mod add;
pub mod build;
pub mod cache;
pub(crate) mod dependency_common;
pub mod doctor;
pub mod fetch;
pub(crate) mod graph_common;
pub mod info;
pub mod init;
pub mod metadata;
pub mod new;
pub mod outdated;
pub mod recipe_check;
pub mod registry_cmd;
pub mod remove;
pub mod run;
pub mod search;
pub mod sync;
pub mod tree;
pub mod update;
pub mod vendor;
pub mod verify;
pub mod version;
pub mod why;

use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{Commands, RuntimeFlags};
use crate::error::JoyError;
use crate::manifest::{Manifest, ManifestDocument, WorkspaceManifest};
use crate::templates;

/// Unified command result used by the output renderer for human and JSON modes.
#[derive(Debug, Clone)]
pub struct CommandOutput {
  pub command: &'static str,
  pub human_message: String,
  pub data: Value,
}

impl CommandOutput {
  /// Create a command output envelope payload.
  pub fn new(command: &'static str, human_message: impl Into<String>, data: Value) -> Self {
    Self { command, human_message: human_message.into(), data }
  }

  /// Create a command output payload from any serializable response type.
  pub fn from_data<T: Serialize>(
    command: &'static str,
    human_message: impl Into<String>,
    data: &T,
  ) -> Result<Self, JoyError> {
    let data = serde_json::to_value(data).map_err(|err| {
      JoyError::new(
        command,
        "output_serialize_failed",
        format!("failed to serialize output: {err}"),
        1,
      )
    })?;
    Ok(Self::new(command, human_message, data))
  }
}

/// Dispatch the parsed CLI subcommand to its handler.
pub fn dispatch(command: Commands, runtime: RuntimeFlags) -> Result<CommandOutput, JoyError> {
  match command {
    Commands::Version(args) => version::handle(args),
    Commands::New(args) => new::handle(args),
    Commands::Init(args) => init::handle(args),
    Commands::Add(args) => {
      dispatch_project_scoped("add", runtime, |runtime| add::handle(args, runtime))
    },
    Commands::Remove(args) => {
      dispatch_project_scoped("remove", runtime, |runtime| remove::handle(args, runtime))
    },
    Commands::Update(args) => {
      dispatch_project_scoped("update", runtime, |runtime| update::handle(args, runtime))
    },
    Commands::Tree(args) => {
      dispatch_project_scoped("tree", runtime, |runtime| tree::handle(args, runtime))
    },
    Commands::Why(args) => {
      dispatch_project_scoped("why", runtime, |runtime| why::handle(args, runtime))
    },
    Commands::Outdated(args) => {
      dispatch_project_scoped("outdated", runtime, |runtime| outdated::handle(args, runtime))
    },
    Commands::Registry(args) => registry_cmd::handle(args),
    Commands::Search(args) => search::handle(args),
    Commands::Info(args) => info::handle(args),
    Commands::Fetch(args) => {
      dispatch_project_scoped("fetch", runtime, |runtime| fetch::handle(args, runtime))
    },
    Commands::Vendor(args) => {
      dispatch_project_scoped("vendor", runtime, |runtime| vendor::handle(args, runtime))
    },
    Commands::Verify(args) => {
      dispatch_project_scoped("verify", runtime, |runtime| verify::handle(args, runtime))
    },
    Commands::Cache(args) => cache::handle(args),
    Commands::Metadata(args) => {
      dispatch_project_scoped("metadata", runtime, |runtime| metadata::handle(args, runtime))
    },
    Commands::RecipeCheck(args) => recipe_check::handle(args),
    Commands::Doctor(args) => doctor::handle(args),
    Commands::Build(args) => {
      dispatch_project_scoped("build", runtime, |runtime| build::handle(args, runtime))
    },
    Commands::Sync(args) => {
      dispatch_project_scoped("sync", runtime, |runtime| sync::handle(args, runtime))
    },
    Commands::Run(args) => {
      dispatch_project_scoped("run", runtime, |runtime| run::handle(args, runtime))
    },
  }
}

#[derive(Debug, Clone, Default)]
struct WorkspaceCommandContext {
  workspace_root: Option<PathBuf>,
  workspace_member: Option<String>,
  project_root_override: Option<PathBuf>,
}

fn dispatch_project_scoped<F>(
  command: &'static str,
  runtime: RuntimeFlags,
  f: F,
) -> Result<CommandOutput, JoyError>
where
  F: FnOnce(RuntimeFlags) -> Result<CommandOutput, JoyError>,
{
  let ctx = resolve_workspace_context(command, runtime.workspace_package.as_deref())?;
  let mut scoped_runtime = runtime.clone();
  scoped_runtime.workspace_root = ctx.workspace_root.clone();
  scoped_runtime.workspace_member = ctx.workspace_member.clone();
  let mut result = if let Some(project_root) = &ctx.project_root_override {
    with_current_dir(project_root, || f(scoped_runtime.clone()))?
  } else {
    f(scoped_runtime.clone())?
  };
  attach_workspace_metadata(&mut result, &ctx);
  Ok(result)
}

fn resolve_workspace_context(
  command: &'static str,
  requested_member: Option<&str>,
) -> Result<WorkspaceCommandContext, JoyError> {
  let cwd = env::current_dir().map_err(|err| {
    JoyError::new(command, "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    if requested_member.is_some() {
      return Err(JoyError::new(
        command,
        "workspace_member_invalid",
        "cannot use `-p/--package` outside a workspace root; no `joy.toml` found in the current directory",
        1,
      ));
    }
    return Ok(WorkspaceCommandContext::default());
  }

  let doc = ManifestDocument::load(&manifest_path)
    .map_err(|err| JoyError::new(command, "manifest_parse_error", err.to_string(), 1))?;
  match doc {
    ManifestDocument::Project(_) => {
      if let Some(member) = requested_member {
        return Err(JoyError::new(
          command,
          "workspace_member_invalid",
          format!(
            "cannot use `-p/--package {member}` from a non-workspace project directory; rerun in a workspace root"
          ),
          1,
        ));
      }
      Ok(WorkspaceCommandContext::default())
    },
    ManifestDocument::Workspace(ws) => {
      resolve_workspace_member_context(command, &cwd, ws, requested_member)
    },
    ManifestDocument::Package(_) => Err(JoyError::new(
      command,
      "workspace_member_invalid",
      "current `joy.toml` is a reusable package manifest (`[package]`), not a project/workspace manifest",
      1,
    )),
  }
}

fn resolve_workspace_member_context(
  command: &'static str,
  workspace_root: &Path,
  ws: WorkspaceManifest,
  requested_member: Option<&str>,
) -> Result<WorkspaceCommandContext, JoyError> {
  let selected = requested_member
    .map(ToOwned::to_owned)
    .or(ws.workspace.default_member.clone())
    .ok_or_else(|| {
      JoyError::new(
        command,
        "workspace_member_required",
        "this command was run from a workspace root; select a member with `-p/--package <member>`",
        1,
      )
    })?;

  if !ws.workspace.members.iter().any(|m| m == &selected) {
    return Err(JoyError::new(
      command,
      "workspace_member_not_found",
      format!(
        "workspace member `{selected}` is not listed in `workspace.members`; available members: {}",
        ws.workspace.members.join(", ")
      ),
      1,
    ));
  }

  let member_root = workspace_root.join(&selected);
  let member_manifest_path = member_root.join("joy.toml");
  if !member_manifest_path.is_file() {
    return Err(JoyError::new(
      command,
      "workspace_member_not_found",
      format!(
        "workspace member `{selected}` does not contain a project manifest at `{}`",
        member_manifest_path.display()
      ),
      1,
    ));
  }
  let member_doc = ManifestDocument::load(&member_manifest_path)
    .map_err(|err| JoyError::new(command, "manifest_parse_error", err.to_string(), 1))?;
  match member_doc {
    ManifestDocument::Project(Manifest { .. }) => Ok(WorkspaceCommandContext {
      workspace_root: Some(workspace_root.to_path_buf()),
      workspace_member: Some(selected),
      project_root_override: Some(member_root),
    }),
    ManifestDocument::Workspace(_) => Err(JoyError::new(
      command,
      "workspace_member_not_found",
      format!(
        "workspace member `{selected}` points to another workspace root; nested workspace routing is not supported"
      ),
      1,
    )),
    ManifestDocument::Package(_) => Err(JoyError::new(
      command,
      "workspace_member_not_found",
      format!(
        "workspace member `{selected}` contains a reusable package manifest (`[package]`); workspace members must be project manifests"
      ),
      1,
    )),
  }
}

fn with_current_dir<T>(dir: &Path, f: impl FnOnce() -> Result<T, JoyError>) -> Result<T, JoyError> {
  let old = env::current_dir().map_err(|err| {
    JoyError::new("cli", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  env::set_current_dir(dir).map_err(|err| JoyError::io("cli", "changing directory", dir, &err))?;
  let result = f();
  let restore_result = env::set_current_dir(&old)
    .map_err(|err| JoyError::io("cli", "restoring directory", &old, &err));
  match (result, restore_result) {
    (Ok(value), Ok(())) => Ok(value),
    (Err(err), Ok(())) => Err(err),
    (Ok(_), Err(err)) => Err(err),
    (Err(primary), Err(_restore)) => Err(primary),
  }
}

fn attach_workspace_metadata(result: &mut CommandOutput, ctx: &WorkspaceCommandContext) {
  let Some(obj) = result.data.as_object_mut() else {
    return;
  };
  match (&ctx.workspace_root, &ctx.workspace_member) {
    (Some(root), Some(member)) => {
      obj.insert("workspace_root".into(), Value::String(root.display().to_string()));
      obj.insert("workspace_member".into(), Value::String(member.clone()));
    },
    _ => {
      obj.insert("workspace_root".into(), Value::Null);
      obj.insert("workspace_member".into(), Value::Null);
    },
  }
}

pub(crate) fn scaffold_files(
  command: &'static str,
  root: &Path,
  project_name: &str,
  force: bool,
) -> Result<ScaffoldWriteResult, JoyError> {
  // TODO(phase7): Move scaffolding path policy and force-overwrite semantics into a dedicated
  // `scaffold` module so `new` and `init` handlers remain thin wrappers.
  let mut created = Vec::new();
  let mut overwritten = Vec::new();

  if !root.exists() {
    fs::create_dir_all(root)
      .map_err(|err| JoyError::io(command, "creating directory", root, &err))?;
    created.push(root.to_path_buf());
  }

  let src_dir = root.join("src");
  if !src_dir.exists() {
    fs::create_dir_all(&src_dir)
      .map_err(|err| JoyError::io(command, "creating directory", &src_dir, &err))?;
    created.push(src_dir.clone());
  }

  let manifest_path = root.join("joy.toml");
  let main_cpp_path = src_dir.join("main.cpp");
  let gitignore_path = root.join(".gitignore");

  write_file(
    command,
    &manifest_path,
    &templates::joy_toml(project_name),
    force,
    &mut created,
    &mut overwritten,
  )?;
  write_file(
    command,
    &main_cpp_path,
    templates::main_cpp(),
    force,
    &mut created,
    &mut overwritten,
  )?;
  write_file(
    command,
    &gitignore_path,
    templates::gitignore(),
    force,
    &mut created,
    &mut overwritten,
  )?;

  Ok(ScaffoldWriteResult { root: root.to_path_buf(), created, overwritten })
}

pub(crate) fn dir_is_empty(path: &Path) -> Result<bool, JoyError> {
  let mut entries =
    fs::read_dir(path).map_err(|err| JoyError::io("new", "reading directory", path, &err))?;
  Ok(entries.next().is_none())
}

fn write_file(
  command: &'static str,
  path: &Path,
  contents: &str,
  force: bool,
  created: &mut Vec<PathBuf>,
  overwritten: &mut Vec<PathBuf>,
) -> Result<(), JoyError> {
  if path.exists() {
    if !force {
      return Err(JoyError::new(
        command,
        "path_exists",
        format!("refusing to overwrite existing path `{}` (use --force)", path.display()),
        1,
      ));
    }
    overwritten.push(path.to_path_buf());
  } else {
    created.push(path.to_path_buf());
  }

  fs::write(path, contents).map_err(|err| JoyError::io(command, "writing file", path, &err))
}

#[derive(Debug, Clone)]
pub(crate) struct ScaffoldWriteResult {
  pub root: PathBuf,
  pub created: Vec<PathBuf>,
  pub overwritten: Vec<PathBuf>,
}
