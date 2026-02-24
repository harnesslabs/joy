use serde_json::json;
use std::env;

use crate::cli::AddArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::manifest::{DependencySource, DependencySpec, Manifest};
use crate::project_env;

pub fn handle(args: AddArgs) -> Result<CommandOutput, JoyError> {
  validate_package_id(&args.package)?;

  let cwd = env::current_dir().map_err(|err| {
    JoyError::new("add", "cwd_unavailable", format!("failed to get cwd: {err}"), 1)
  })?;
  let manifest_path = cwd.join("joy.toml");
  if !manifest_path.is_file() {
    return Err(JoyError::new(
      "add",
      "manifest_not_found",
      format!("no `joy.toml` found at {}", manifest_path.display()),
      1,
    ));
  }

  let mut manifest = Manifest::load(&manifest_path)
    .map_err(|err| JoyError::new("add", "manifest_parse_error", err.to_string(), 1))?;

  let rev = args.rev.unwrap_or_else(|| "HEAD".to_string());
  let changed = manifest.add_dependency(
    args.package.clone(),
    DependencySpec { source: DependencySource::Github, rev: rev.clone() },
  );
  manifest
    .save(&manifest_path)
    .map_err(|err| JoyError::new("add", "manifest_write_error", err.to_string(), 1))?;

  let env_layout = project_env::ensure_layout(&cwd)
    .map_err(|err| JoyError::new("add", "env_setup_failed", err.to_string(), 1))?;

  let lockfile_warning = cwd.join("joy.lock").is_file().then_some(
    "joy.lock exists and may be stale; future builds should refresh the lockfile".to_string(),
  );
  let mut human_message = if changed {
    format!("Added dependency `{}` (rev `{rev}`)", args.package)
  } else {
    format!("Dependency `{}` already present with rev `{rev}`", args.package)
  };
  if let Some(warning) = &lockfile_warning {
    human_message.push('\n');
    human_message.push_str("warning: ");
    human_message.push_str(warning);
  }

  let created_env_paths: Vec<String> =
    env_layout.created_paths.iter().map(|path| path.display().to_string()).collect();

  Ok(CommandOutput::new(
    "add",
    human_message,
    json!({
      "package": args.package,
      "rev": rev,
      "changed": changed,
      "manifest_path": manifest_path.display().to_string(),
      "project_root": cwd.display().to_string(),
      "created_env_paths": created_env_paths,
      "warnings": lockfile_warning.map(|w| vec![w]).unwrap_or_default(),
    }),
  ))
}

fn validate_package_id(package: &str) -> Result<(), JoyError> {
  let mut parts = package.split('/');
  let owner = parts.next().unwrap_or_default();
  let repo = parts.next().unwrap_or_default();
  let extra = parts.next();

  let valid = extra.is_none()
    && !owner.is_empty()
    && !repo.is_empty()
    && owner.chars().all(is_valid_package_char)
    && repo.chars().all(is_valid_package_char);

  if valid {
    Ok(())
  } else {
    Err(JoyError::new(
      "add",
      "invalid_package_id",
      format!("invalid package `{package}`; expected `owner/repo`"),
      1,
    ))
  }
}

fn is_valid_package_char(ch: char) -> bool {
  ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')
}

#[cfg(test)]
mod tests {
  use super::validate_package_id;

  #[test]
  fn validates_github_shorthand_package_ids() {
    for valid in ["nlohmann/json", "fmtlib/fmt", "owner/repo-name_1.2"] {
      validate_package_id(valid).expect("valid package");
    }
  }

  #[test]
  fn rejects_invalid_package_ids() {
    for invalid in ["", "owner", "/repo", "owner/", "owner/repo/extra", "ow ner/repo"] {
      let err = validate_package_id(invalid).expect_err("invalid package");
      assert_eq!(err.code, "invalid_package_id");
    }
  }
}
