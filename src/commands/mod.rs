pub mod add;
pub mod build;
pub mod init;
pub mod new;
pub mod run;

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Commands;
use crate::error::JoyError;
use crate::templates;

#[derive(Debug, Clone)]
pub struct CommandOutput {
  pub command: &'static str,
  pub human_message: String,
  pub data: Value,
}

impl CommandOutput {
  pub fn new(command: &'static str, human_message: impl Into<String>, data: Value) -> Self {
    Self { command, human_message: human_message.into(), data }
  }
}

pub fn dispatch(command: Commands) -> Result<CommandOutput, JoyError> {
  match command {
    Commands::New(args) => new::handle(args),
    Commands::Init(args) => init::handle(args),
    Commands::Add(args) => add::handle(args),
    Commands::Build(args) => build::handle(args),
    Commands::Run(args) => run::handle(args),
  }
}

pub(crate) fn not_implemented(command: &'static str) -> Result<CommandOutput, JoyError> {
  Err(JoyError::not_implemented(command))
}

pub(crate) fn scaffold_files(
  command: &'static str,
  root: &Path,
  project_name: &str,
  force: bool,
) -> Result<ScaffoldWriteResult, JoyError> {
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
