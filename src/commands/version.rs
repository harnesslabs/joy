use serde::Serialize;

use crate::cli::VersionArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::output::HumanMessageBuilder;

#[derive(Debug, Serialize)]
struct VersionResponse {
  joy_version: String,
  schema_version: String,
  build_target: String,
  build_profile: String,
  git_commit: String,
}

pub fn handle(_args: VersionArgs) -> Result<CommandOutput, JoyError> {
  let payload = VersionResponse {
    joy_version: env!("CARGO_PKG_VERSION").to_string(),
    schema_version: "1".to_string(),
    build_target: compile_target(),
    build_profile: if cfg!(debug_assertions) { "debug".into() } else { "release".into() },
    git_commit: build_commit(),
  };

  CommandOutput::from_data(
    "version",
    HumanMessageBuilder::new(format!("joy {}", payload.joy_version))
      .kv("schema", payload.schema_version.clone())
      .kv("target", payload.build_target.clone())
      .kv("profile", payload.build_profile.clone())
      .kv("commit", payload.git_commit.clone())
      .build(),
    &payload,
  )
}

fn compile_target() -> String {
  option_env!("TARGET")
    .map(ToOwned::to_owned)
    .unwrap_or_else(|| format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS))
}

fn build_commit() -> String {
  option_env!("JOY_BUILD_COMMIT")
    .or_else(|| option_env!("VERGEN_GIT_SHA"))
    .unwrap_or("unknown")
    .to_string()
}

#[cfg(test)]
mod tests {
  use super::{build_commit, compile_target};

  #[test]
  fn target_is_non_empty() {
    assert!(!compile_target().trim().is_empty());
  }

  #[test]
  fn commit_is_non_empty() {
    assert!(!build_commit().trim().is_empty());
  }
}
