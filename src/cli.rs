use clap::{Args, Parser, Subcommand, ValueEnum};
use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;

use crate::output::{
  ColorPreference, GlyphMode, GlyphPreference, HumanUiConfig, OutputMode, ProgressPreference,
};

const CLI_AFTER_HELP: &str = "\
Examples:
  joy new hello_cpp
  joy add nlohmann/json
  joy sync
  joy --frozen build
  joy --json doctor

Common workflow:
  1. `joy add <package>` to declare dependencies
  2. `joy sync` to materialize dependency + lockfile state
  3. `joy build` or `joy run` to compile and execute

Human UX controls:
  joy --color always --glyphs unicode doctor
  joy --progress never build

Docs:
  https://joy.harnesslabs.dev/
";

#[derive(Debug, Parser)]
#[command(
  name = "joy",
  version,
  about = "Native C++ package and build manager",
  after_help = CLI_AFTER_HELP
)]
pub struct Cli {
  /// Emit machine-readable JSON output.
  #[arg(long, visible_alias = "machine", global = true)]
  pub json: bool,

  /// Control ANSI color rendering in human output.
  #[arg(long, value_enum, global = true)]
  pub color: Option<CliColorArg>,

  /// Control progress/spinner rendering in human output.
  #[arg(long, value_enum, global = true)]
  pub progress: Option<CliProgressArg>,

  /// Control terminal glyph style in human output (Unicode vs ASCII fallbacks).
  #[arg(long, value_enum, global = true)]
  pub glyphs: Option<CliGlyphsArg>,

  /// Disable progress output (`--progress=never`).
  #[arg(long, hide = true, global = true, conflicts_with = "progress")]
  pub no_progress: bool,

  /// Force ASCII glyphs (`--glyphs=ascii`).
  #[arg(long, hide = true, global = true, conflicts_with = "glyphs")]
  pub ascii: bool,

  /// Workspace member package to operate on when running from a workspace root.
  #[arg(long, short = 'p', global = true)]
  pub workspace_package: Option<String>,

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
    let ui = self.resolve_human_ui();
    let progress_enabled =
      !self.json && !matches!(resolved_progress_preference(self), ProgressPreference::Never);
    RuntimeFlags {
      offline: self.offline || self.frozen,
      frozen: self.frozen,
      progress: progress_enabled,
      ui,
      workspace_package: self.workspace_package.clone(),
      workspace_root: None,
      workspace_member: None,
    }
  }

  fn resolve_human_ui(&self) -> HumanUiConfig {
    if self.json {
      return HumanUiConfig::default();
    }

    let stderr_is_tty = std::io::stderr().is_terminal();
    let term_dumb = env::var("TERM").ok().map(|v| v.eq_ignore_ascii_case("dumb")).unwrap_or(false);
    let ci = env::var_os("CI").is_some();

    let color_pref = resolved_color_preference(self);
    let progress_pref = resolved_progress_preference(self);
    let glyph_pref = resolved_glyph_preference(self);

    let color_enabled = match color_pref {
      ColorPreference::Always => true,
      ColorPreference::Never => false,
      ColorPreference::Auto => stderr_is_tty && !term_dumb,
    };
    let progress_enabled = match progress_pref {
      ProgressPreference::Always => true,
      ProgressPreference::Never => false,
      ProgressPreference::Auto => stderr_is_tty && !term_dumb && !ci,
    };
    let glyph_mode = match glyph_pref {
      GlyphPreference::Unicode => GlyphMode::Unicode,
      GlyphPreference::Ascii => GlyphMode::Ascii,
      GlyphPreference::Auto => {
        if stderr_is_tty && !term_dumb {
          GlyphMode::Unicode
        } else {
          GlyphMode::Ascii
        }
      },
    };

    HumanUiConfig { color_enabled, progress_enabled, glyph_mode, stderr_is_tty }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFlags {
  pub offline: bool,
  pub frozen: bool,
  pub progress: bool,
  pub ui: HumanUiConfig,
  pub workspace_package: Option<String>,
  pub workspace_root: Option<PathBuf>,
  pub workspace_member: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliColorArg {
  Auto,
  Always,
  Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliProgressArg {
  Auto,
  Always,
  Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliGlyphsArg {
  Auto,
  Unicode,
  Ascii,
}

fn resolved_color_preference(cli: &Cli) -> ColorPreference {
  if let Some(value) = cli.color {
    return map_color_arg(value);
  }
  if let Some(value) = parse_color_env(env::var("JOY_COLOR").ok().as_deref()) {
    return value;
  }
  if env::var_os("NO_COLOR").is_some() {
    return ColorPreference::Never;
  }
  if env_var_truthy("CLICOLOR_FORCE") {
    return ColorPreference::Always;
  }
  if env::var("CLICOLOR").ok().as_deref() == Some("0") {
    return ColorPreference::Never;
  }
  ColorPreference::Auto
}

fn resolved_progress_preference(cli: &Cli) -> ProgressPreference {
  if cli.no_progress {
    return ProgressPreference::Never;
  }
  if let Some(value) = cli.progress {
    return map_progress_arg(value);
  }
  if let Some(value) = parse_progress_env(env::var("JOY_PROGRESS").ok().as_deref()) {
    return value;
  }
  ProgressPreference::Auto
}

fn resolved_glyph_preference(cli: &Cli) -> GlyphPreference {
  if cli.ascii {
    return GlyphPreference::Ascii;
  }
  if let Some(value) = cli.glyphs {
    return map_glyphs_arg(value);
  }
  if let Some(value) = parse_glyphs_env(env::var("JOY_GLYPHS").ok().as_deref()) {
    return value;
  }
  GlyphPreference::Auto
}

fn map_color_arg(value: CliColorArg) -> ColorPreference {
  match value {
    CliColorArg::Auto => ColorPreference::Auto,
    CliColorArg::Always => ColorPreference::Always,
    CliColorArg::Never => ColorPreference::Never,
  }
}

fn map_progress_arg(value: CliProgressArg) -> ProgressPreference {
  match value {
    CliProgressArg::Auto => ProgressPreference::Auto,
    CliProgressArg::Always => ProgressPreference::Always,
    CliProgressArg::Never => ProgressPreference::Never,
  }
}

fn map_glyphs_arg(value: CliGlyphsArg) -> GlyphPreference {
  match value {
    CliGlyphsArg::Auto => GlyphPreference::Auto,
    CliGlyphsArg::Unicode => GlyphPreference::Unicode,
    CliGlyphsArg::Ascii => GlyphPreference::Ascii,
  }
}

fn parse_color_env(value: Option<&str>) -> Option<ColorPreference> {
  match value?.trim().to_ascii_lowercase().as_str() {
    "auto" => Some(ColorPreference::Auto),
    "always" | "1" | "true" | "on" => Some(ColorPreference::Always),
    "never" | "0" | "false" | "off" => Some(ColorPreference::Never),
    _ => None,
  }
}

fn parse_progress_env(value: Option<&str>) -> Option<ProgressPreference> {
  match value?.trim().to_ascii_lowercase().as_str() {
    "auto" => Some(ProgressPreference::Auto),
    "always" | "1" | "true" | "on" => Some(ProgressPreference::Always),
    "never" | "0" | "false" | "off" => Some(ProgressPreference::Never),
    _ => None,
  }
}

fn parse_glyphs_env(value: Option<&str>) -> Option<GlyphPreference> {
  match value?.trim().to_ascii_lowercase().as_str() {
    "auto" => Some(GlyphPreference::Auto),
    "unicode" | "utf8" | "utf-8" | "1" | "true" => Some(GlyphPreference::Unicode),
    "ascii" | "0" | "false" => Some(GlyphPreference::Ascii),
    _ => None,
  }
}

fn env_var_truthy(name: &str) -> bool {
  env::var(name)
    .ok()
    .map(|v| {
      let trimmed = v.trim();
      !trimmed.is_empty() && trimmed != "0"
    })
    .unwrap_or(false)
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Print CLI/build version information.
  #[command(after_help = "Examples:\n  joy version\n  joy --json version")]
  Version(VersionArgs),
  /// Create a new joy project in a new directory.
  New(NewArgs),
  /// Initialize a joy project in the current directory.
  Init(InitArgs),
  /// Add a package dependency to the current project.
  #[command(after_help = "Example:\n  joy add nlohmann/json\n  joy add fmtlib/fmt --rev 11.0.2")]
  Add(AddArgs),
  /// Remove a package dependency from the current project.
  Remove(RemoveArgs),
  /// Refresh dependency sources and optionally update exact refs.
  #[command(after_help = "Examples:\n  joy update\n  joy update fmtlib/fmt --rev 11.1.0")]
  Update(UpdateArgs),
  /// Show the resolved dependency graph.
  #[command(after_help = "Examples:\n  joy tree\n  joy --json tree")]
  Tree(TreeArgs),
  /// Explain why a dependency is present in the resolved graph.
  #[command(after_help = "Examples:\n  joy why nlohmann/json\n  joy why fmtlib/fmt --locked")]
  Why(WhyArgs),
  /// Report available updates for direct and transitive dependencies.
  #[command(after_help = "Examples:\n  joy outdated\n  joy --json outdated")]
  Outdated(OutdatedArgs),
  /// Manage registry configuration.
  Registry(RegistryArgs),
  /// Search package metadata in a configured registry.
  Search(SearchArgs),
  /// Show package metadata for a configured registry package.
  Info(InfoArgs),
  /// Warm dependency cache state without building.
  Fetch(FetchArgs),
  /// Vendor dependencies into a project-local directory.
  Vendor(VendorArgs),
  /// Manage global cache lifecycle.
  Cache(CacheArgs),
  /// Emit machine-oriented project/dependency/editor metadata.
  #[command(after_help = "Examples:\n  joy metadata\n  joy --json metadata")]
  Metadata(MetadataArgs),
  /// Validate bundled recipe metadata (for local checks and CI).
  RecipeCheck(RecipeCheckArgs),
  /// Diagnose local toolchain, cache, and recipe environment health.
  #[command(after_help = "Examples:\n  joy doctor\n  joy --json doctor")]
  Doctor(DoctorArgs),
  /// Build the current project.
  #[command(after_help = "Examples:\n  joy build\n  joy build --locked\n  joy --offline build")]
  Build(BuildArgs),
  /// Materialize dependencies and lockfile state without compiling the final binary.
  #[command(after_help = "Examples:\n  joy sync\n  joy sync --update-lock\n  joy --frozen sync")]
  Sync(SyncArgs),
  /// Build and run the current project.
  #[command(after_help = "Examples:\n  joy run\n  joy run -- --app-arg")]
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
  #[arg(long = "as")]
  pub as_name: Option<String>,
  #[arg(long)]
  pub rev: Option<String>,
  #[arg(long)]
  pub version: Option<String>,
  #[arg(long)]
  pub registry: Option<String>,
  #[arg(long)]
  pub sha256: Option<String>,
  #[arg(long)]
  pub no_sync: bool,
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
  #[arg(long)]
  pub version: Option<String>,
  #[arg(long)]
  pub registry: Option<String>,
  #[arg(long)]
  pub sha256: Option<String>,
}

#[derive(Debug, Args)]
pub struct RegistryArgs {
  #[command(subcommand)]
  pub command: RegistrySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum RegistrySubcommand {
  List(RegistryListArgs),
  Add(RegistryAddArgs),
  Remove(RegistryRemoveArgs),
  SetDefault(RegistrySetDefaultArgs),
}

#[derive(Debug, Args)]
pub struct RegistryListArgs {
  #[arg(long)]
  pub project: bool,
}

#[derive(Debug, Args)]
pub struct RegistryAddArgs {
  pub name: String,
  pub index: String,
  #[arg(long)]
  pub default: bool,
  #[arg(long)]
  pub project: bool,
}

#[derive(Debug, Args)]
pub struct RegistryRemoveArgs {
  pub name: String,
  #[arg(long)]
  pub project: bool,
}

#[derive(Debug, Args)]
pub struct RegistrySetDefaultArgs {
  pub name: String,
  #[arg(long)]
  pub project: bool,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
  pub query: String,
  #[arg(long)]
  pub registry: Option<String>,
  #[arg(long, default_value_t = 20)]
  pub limit: usize,
}

#[derive(Debug, Args)]
pub struct InfoArgs {
  pub package: String,
  #[arg(long)]
  pub registry: Option<String>,
}

#[derive(Debug, Args)]
pub struct FetchArgs {}

#[derive(Debug, Args)]
pub struct VendorArgs {
  #[arg(long)]
  pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct CacheArgs {
  #[command(subcommand)]
  pub command: CacheSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CacheSubcommand {
  Gc(CacheGcArgs),
}

#[derive(Debug, Args)]
pub struct CacheGcArgs {
  #[arg(long)]
  pub aggressive: bool,
}

#[derive(Debug, Args)]
pub struct TreeArgs {
  #[arg(long)]
  pub locked: bool,
}

#[derive(Debug, Args)]
pub struct WhyArgs {
  pub package: String,
  #[arg(long)]
  pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutdatedSourceArg {
  All,
  Registry,
  Github,
}

#[derive(Debug, Args)]
pub struct OutdatedArgs {
  /// Restrict update checks to a dependency source subset.
  #[arg(long, value_enum, default_value_t = OutdatedSourceArg::All)]
  pub sources: OutdatedSourceArg,
}

#[derive(Debug, Args)]
pub struct VersionArgs {}

#[derive(Debug, Args)]
pub struct MetadataArgs {}

#[derive(Debug, Args)]
pub struct RecipeCheckArgs {}

#[derive(Debug, Args)]
pub struct DoctorArgs {}

#[derive(Debug, Args)]
pub struct BuildArgs {
  #[arg(long)]
  pub release: bool,
  #[arg(long)]
  pub target: Option<String>,
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
  pub target: Option<String>,
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

  use super::{
    CacheSubcommand, Cli, CliColorArg, CliGlyphsArg, CliProgressArg, Commands, OutdatedSourceArg,
    RegistrySubcommand,
  };

  #[test]
  fn parses_version_command() {
    let cli = Cli::parse_from(["joy", "version"]);
    match cli.command {
      Commands::Version(_) => {},
      other => panic!("expected version, got {other:?}"),
    }
  }

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
      Commands::Init(ref args) => assert!(args.force),
      other => panic!("expected init, got {other:?}"),
    }
    assert!(cli.json);
    assert!(!cli.runtime_flags().progress);
  }

  #[test]
  fn parses_global_offline_and_frozen_flags() {
    let cli = Cli::parse_from(["joy", "-p", "app", "--offline", "--frozen", "sync"]);
    assert_eq!(cli.workspace_package.as_deref(), Some("app"));
    assert!(cli.offline);
    assert!(cli.frozen);
    assert!(cli.runtime_flags().offline);
    assert!(cli.runtime_flags().frozen);
    assert_eq!(cli.runtime_flags().workspace_package.as_deref(), Some("app"));
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
        assert_eq!(args.version, None);
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
        assert_eq!(args.version, None);
      },
      other => panic!("expected update, got {other:?}"),
    }
  }

  #[test]
  fn parses_add_with_semver_version() {
    let cli = Cli::parse_from(["joy", "add", "fmtlib/fmt", "--version", "^11"]);
    match cli.command {
      Commands::Add(args) => {
        assert_eq!(args.package, "fmtlib/fmt");
        assert_eq!(args.version.as_deref(), Some("^11"));
        assert_eq!(args.rev, None);
        assert!(!args.no_sync);
      },
      other => panic!("expected add, got {other:?}"),
    }
  }

  #[test]
  fn parses_add_with_no_sync_flag() {
    let cli = Cli::parse_from(["joy", "add", "nlohmann/json", "--no-sync"]);
    match cli.command {
      Commands::Add(args) => assert!(args.no_sync),
      other => panic!("expected add, got {other:?}"),
    }
  }

  #[test]
  fn parses_add_with_alias_registry_and_sha256() {
    let cli = Cli::parse_from([
      "joy",
      "add",
      "archive:https://example.com/lib.tar.gz",
      "--as",
      "archive_dep",
      "--registry",
      "corp",
      "--sha256",
      "deadbeef",
    ]);
    match cli.command {
      Commands::Add(args) => {
        assert_eq!(args.as_name.as_deref(), Some("archive_dep"));
        assert_eq!(args.registry.as_deref(), Some("corp"));
        assert_eq!(args.sha256.as_deref(), Some("deadbeef"));
      },
      other => panic!("expected add, got {other:?}"),
    }
  }

  #[test]
  fn parses_tree_command() {
    let cli = Cli::parse_from(["joy", "tree"]);
    match cli.command {
      Commands::Tree(args) => assert!(!args.locked),
      other => panic!("expected tree, got {other:?}"),
    }
  }

  #[test]
  fn parses_tree_locked_flag() {
    let cli = Cli::parse_from(["joy", "tree", "--locked"]);
    match cli.command {
      Commands::Tree(args) => assert!(args.locked),
      other => panic!("expected tree, got {other:?}"),
    }
  }

  #[test]
  fn parses_why_command() {
    let cli = Cli::parse_from(["joy", "why", "nlohmann/json", "--locked"]);
    match cli.command {
      Commands::Why(args) => {
        assert_eq!(args.package, "nlohmann/json");
        assert!(args.locked);
      },
      other => panic!("expected why, got {other:?}"),
    }
  }

  #[test]
  fn parses_outdated_command() {
    let cli = Cli::parse_from(["joy", "outdated"]);
    match cli.command {
      Commands::Outdated(args) => assert_eq!(args.sources, OutdatedSourceArg::All),
      other => panic!("expected outdated, got {other:?}"),
    }
  }

  #[test]
  fn parses_outdated_sources_filter() {
    let cli = Cli::parse_from(["joy", "outdated", "--sources", "github"]);
    match cli.command {
      Commands::Outdated(args) => assert_eq!(args.sources, OutdatedSourceArg::Github),
      other => panic!("expected outdated, got {other:?}"),
    }
  }

  #[test]
  fn parses_metadata_command() {
    let cli = Cli::parse_from(["joy", "metadata"]);
    match cli.command {
      Commands::Metadata(_) => {},
      other => panic!("expected metadata, got {other:?}"),
    }
  }

  #[test]
  fn parses_registry_add_and_set_default_commands() {
    let add = Cli::parse_from([
      "joy",
      "registry",
      "add",
      "corp",
      "https://example.com/registry.git",
      "--project",
      "--default",
    ]);
    match add.command {
      Commands::Registry(args) => match args.command {
        RegistrySubcommand::Add(sub) => {
          assert_eq!(sub.name, "corp");
          assert_eq!(sub.index, "https://example.com/registry.git");
          assert!(sub.project);
          assert!(sub.default);
        },
        other => panic!("expected registry add, got {other:?}"),
      },
      other => panic!("expected registry, got {other:?}"),
    }

    let set_default = Cli::parse_from(["joy", "registry", "set-default", "corp"]);
    match set_default.command {
      Commands::Registry(args) => match args.command {
        RegistrySubcommand::SetDefault(sub) => {
          assert_eq!(sub.name, "corp");
          assert!(!sub.project);
        },
        other => panic!("expected registry set-default, got {other:?}"),
      },
      other => panic!("expected registry, got {other:?}"),
    }
  }

  #[test]
  fn parses_search_and_info_commands() {
    let search = Cli::parse_from(["joy", "search", "fmt", "--registry", "corp", "--limit", "7"]);
    match search.command {
      Commands::Search(args) => {
        assert_eq!(args.query, "fmt");
        assert_eq!(args.registry.as_deref(), Some("corp"));
        assert_eq!(args.limit, 7);
      },
      other => panic!("expected search, got {other:?}"),
    }

    let info = Cli::parse_from(["joy", "info", "fmtlib/fmt", "--registry", "default"]);
    match info.command {
      Commands::Info(args) => {
        assert_eq!(args.package, "fmtlib/fmt");
        assert_eq!(args.registry.as_deref(), Some("default"));
      },
      other => panic!("expected info, got {other:?}"),
    }
  }

  #[test]
  fn parses_fetch_vendor_and_cache_gc_commands() {
    let fetch = Cli::parse_from(["joy", "fetch"]);
    match fetch.command {
      Commands::Fetch(_) => {},
      other => panic!("expected fetch, got {other:?}"),
    }

    let vendor = Cli::parse_from(["joy", "vendor", "--output", "third_party"]);
    match vendor.command {
      Commands::Vendor(args) => assert_eq!(args.output.as_deref(), Some("third_party")),
      other => panic!("expected vendor, got {other:?}"),
    }

    let cache = Cli::parse_from(["joy", "cache", "gc", "--aggressive"]);
    match cache.command {
      Commands::Cache(args) => match args.command {
        CacheSubcommand::Gc(gc) => assert!(gc.aggressive),
      },
      other => panic!("expected cache, got {other:?}"),
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
  fn parses_doctor_command() {
    let cli = Cli::parse_from(["joy", "doctor"]);
    match cli.command {
      Commands::Doctor(_) => {},
      other => panic!("expected doctor, got {other:?}"),
    }
  }

  #[test]
  fn parses_build_flags() {
    let cli = Cli::parse_from([
      "joy",
      "build",
      "--release",
      "--target",
      "tool",
      "--locked",
      "--update-lock",
    ]);
    match cli.command {
      Commands::Build(args) => {
        assert!(args.release);
        assert_eq!(args.target.as_deref(), Some("tool"));
        assert!(args.locked);
        assert!(args.update_lock);
      },
      other => panic!("expected build, got {other:?}"),
    }
  }

  #[test]
  fn parses_run_with_passthrough_args() {
    let cli = Cli::parse_from([
      "joy",
      "run",
      "--release",
      "--target",
      "tool",
      "--",
      "one",
      "two",
      "--flag",
    ]);
    match cli.command {
      Commands::Run(args) => {
        assert!(args.release);
        assert_eq!(args.target.as_deref(), Some("tool"));
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

  #[test]
  fn parses_global_ui_flags() {
    let cli = Cli::parse_from([
      "joy",
      "--color",
      "always",
      "--progress",
      "never",
      "--glyphs",
      "ascii",
      "doctor",
    ]);
    assert_eq!(cli.color, Some(CliColorArg::Always));
    assert_eq!(cli.progress, Some(CliProgressArg::Never));
    assert_eq!(cli.glyphs, Some(CliGlyphsArg::Ascii));
    assert!(!cli.no_progress);
    assert!(!cli.ascii);
    let runtime = cli.runtime_flags();
    assert!(!runtime.progress);
    assert!(runtime.ui.color_enabled);
  }

  #[test]
  fn parses_hidden_ui_alias_flags() {
    let cli = Cli::parse_from(["joy", "--no-progress", "--ascii", "doctor"]);
    assert!(cli.no_progress);
    assert!(cli.ascii);
    let runtime = cli.runtime_flags();
    assert!(!runtime.progress);
    assert_eq!(runtime.ui.glyph_mode, crate::output::GlyphMode::Ascii);
  }
}
