use clap::{Args, Parser, Subcommand};

use crate::output::OutputMode;

#[derive(Debug, Parser)]
#[command(name = "joy", version, about = "Native C++ package and build manager")]
pub struct Cli {
  /// Emit machine-readable JSON output.
  #[arg(long, visible_alias = "machine", global = true)]
  pub json: bool,

  /// Resolve and build using only locally cached dependency data.
  #[arg(long, global = true)]
  pub offline: bool,

  /// CI-safe mode: no network access and no lockfile changes (`--offline` + `--locked`).
  #[arg(long, global = true)]
  pub frozen: bool,

  #[command(subcommand)]
  pub command: Commands,
}

impl Cli {
  pub fn output_mode(&self) -> OutputMode {
    if self.json { OutputMode::Json } else { OutputMode::Human }
  }

  pub fn runtime_flags(&self) -> RuntimeFlags {
    RuntimeFlags { offline: self.offline || self.frozen, frozen: self.frozen }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeFlags {
  pub offline: bool,
  pub frozen: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Create a new joy project in a new directory.
  New(NewArgs),
  /// Initialize a joy project in the current directory.
  Init(InitArgs),
  /// Add a package dependency to the current project.
  Add(AddArgs),
  /// Remove a package dependency from the current project.
  Remove(RemoveArgs),
  /// Refresh dependency sources and optionally update exact refs.
  Update(UpdateArgs),
  /// Show the resolved dependency graph.
  Tree(TreeArgs),
  /// Validate bundled recipe metadata (for local checks and CI).
  RecipeCheck(RecipeCheckArgs),
  /// Build the current project.
  Build(BuildArgs),
  /// Materialize dependencies and lockfile state without compiling the final binary.
  Sync(SyncArgs),
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
pub struct RemoveArgs {
  pub package: String,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
  pub package: Option<String>,
  #[arg(long)]
  pub rev: Option<String>,
}

#[derive(Debug, Args)]
pub struct TreeArgs {}

#[derive(Debug, Args)]
pub struct RecipeCheckArgs {}

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
pub struct SyncArgs {
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
  fn parses_global_offline_and_frozen_flags() {
    let cli = Cli::parse_from(["joy", "--offline", "--frozen", "sync"]);
    assert!(cli.offline);
    assert!(cli.frozen);
    assert!(cli.runtime_flags().offline);
    assert!(cli.runtime_flags().frozen);
    match cli.command {
      Commands::Sync(_) => {},
      other => panic!("expected sync, got {other:?}"),
    }
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
  fn parses_remove_command() {
    let cli = Cli::parse_from(["joy", "remove", "nlohmann/json"]);
    match cli.command {
      Commands::Remove(args) => assert_eq!(args.package, "nlohmann/json"),
      other => panic!("expected remove, got {other:?}"),
    }
  }

  #[test]
  fn parses_update_with_optional_package_and_rev() {
    let cli = Cli::parse_from(["joy", "update", "nlohmann/json", "--rev", "v1.2.3"]);
    match cli.command {
      Commands::Update(args) => {
        assert_eq!(args.package.as_deref(), Some("nlohmann/json"));
        assert_eq!(args.rev.as_deref(), Some("v1.2.3"));
      },
      other => panic!("expected update, got {other:?}"),
    }
  }

  #[test]
  fn parses_tree_command() {
    let cli = Cli::parse_from(["joy", "tree"]);
    match cli.command {
      Commands::Tree(_) => {},
      other => panic!("expected tree, got {other:?}"),
    }
  }

  #[test]
  fn parses_recipe_check_command() {
    let cli = Cli::parse_from(["joy", "recipe-check"]);
    match cli.command {
      Commands::RecipeCheck(_) => {},
      other => panic!("expected recipe-check, got {other:?}"),
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

  #[test]
  fn parses_sync_flags() {
    let cli = Cli::parse_from(["joy", "sync", "--release", "--locked", "--update-lock"]);
    match cli.command {
      Commands::Sync(args) => {
        assert!(args.release);
        assert!(args.locked);
        assert!(args.update_lock);
      },
      other => panic!("expected sync, got {other:?}"),
    }
  }
}
