use clap::{Args, Parser, Subcommand};

use crate::output::OutputMode;

#[derive(Debug, Parser)]
#[command(name = "joy", version, about = "Native C++ package and build manager")]
pub struct Cli {
  /// Emit machine-readable JSON output.
  #[arg(long, visible_alias = "machine", global = true)]
  pub json: bool,

  #[command(subcommand)]
  pub command: Commands,
}

impl Cli {
  pub fn output_mode(&self) -> OutputMode {
    if self.json { OutputMode::Json } else { OutputMode::Human }
  }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Create a new joy project in a new directory.
  New(NewArgs),
  /// Initialize a joy project in the current directory.
  Init(InitArgs),
  /// Add a package dependency to the current project.
  Add(AddArgs),
  /// Build the current project.
  Build(BuildArgs),
  /// Build and run the current project.
  Run(RunArgs),
}

#[derive(Debug, Args)]
pub struct NewArgs {
  pub name: String,
  #[arg(long)]
  pub force: bool,
}

#[derive(Debug, Args)]
pub struct InitArgs {
  #[arg(long)]
  pub force: bool,
}

#[derive(Debug, Args)]
pub struct AddArgs {
  pub package: String,
  #[arg(long)]
  pub rev: Option<String>,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
  #[arg(long)]
  pub release: bool,
  #[arg(long)]
  pub locked: bool,
  #[arg(long = "update-lock")]
  pub update_lock: bool,
}

#[derive(Debug, Args)]
pub struct RunArgs {
  #[arg(long)]
  pub release: bool,
  #[arg(long)]
  pub locked: bool,
  #[arg(long = "update-lock")]
  pub update_lock: bool,
  #[arg(last = true)]
  pub args: Vec<String>,
}

#[cfg(test)]
mod tests {
  use clap::Parser;

  use super::{Cli, Commands};

  #[test]
  fn requires_a_subcommand() {
    let err = Cli::try_parse_from(["joy"]).expect_err("expected clap parse error");
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand);
  }

  #[test]
  fn parses_new_command() {
    let cli = Cli::parse_from(["joy", "new", "demo"]);
    match cli.command {
      Commands::New(args) => {
        assert_eq!(args.name, "demo");
        assert!(!args.force);
      },
      other => panic!("expected new, got {other:?}"),
    }
    assert!(!cli.json);
  }

  #[test]
  fn parses_init_force_with_json_alias() {
    let cli = Cli::parse_from(["joy", "--machine", "init", "--force"]);
    match cli.command {
      Commands::Init(args) => assert!(args.force),
      other => panic!("expected init, got {other:?}"),
    }
    assert!(cli.json);
  }

  #[test]
  fn parses_add_with_rev() {
    let cli = Cli::parse_from(["joy", "add", "nlohmann/json", "--rev", "v3.11.3"]);
    match cli.command {
      Commands::Add(args) => {
        assert_eq!(args.package, "nlohmann/json");
        assert_eq!(args.rev.as_deref(), Some("v3.11.3"));
      },
      other => panic!("expected add, got {other:?}"),
    }
  }

  #[test]
  fn parses_build_flags() {
    let cli = Cli::parse_from(["joy", "build", "--release", "--locked", "--update-lock"]);
    match cli.command {
      Commands::Build(args) => {
        assert!(args.release);
        assert!(args.locked);
        assert!(args.update_lock);
      },
      other => panic!("expected build, got {other:?}"),
    }
  }

  #[test]
  fn parses_run_with_passthrough_args() {
    let cli = Cli::parse_from(["joy", "run", "--release", "--", "one", "two", "--flag"]);
    match cli.command {
      Commands::Run(args) => {
        assert!(args.release);
        assert_eq!(args.args, vec!["one", "two", "--flag"]);
      },
      other => panic!("expected run, got {other:?}"),
    }
  }
}
