//! `joy` is a native C++ package and build manager with a CLI-first API.
//!
//! The library crate exists primarily to keep the CLI testable: `src/main.rs` is a thin wrapper
//! around [`run`], while command dispatch, manifest parsing, dependency fetching, resolution, and
//! local build orchestration live in modules here.
//!
mod abi;
mod cli;
mod cmake;
mod commands;
mod error;
mod fetch;
mod fs_ops;
mod git_ops;
mod global_cache;
mod install_index;
mod linking;
mod lockfile;
pub mod manifest;
mod ninja;
mod output;
mod package_id;
mod project_env;
mod project_probe;
pub mod recipes;
mod registry;
pub mod resolver;
mod templates;
mod toolchain;

use clap::Parser;
use std::ffi::OsString;
use std::process::ExitCode;

use crate::cli::Cli;
use crate::commands::dispatch;
use crate::output::{configure_ui, print_error, print_success};

/// Run the `joy` CLI using the current process arguments.
pub fn run() -> ExitCode {
  run_from(std::env::args_os())
}

/// Run the `joy` CLI using an explicit argument iterator.
///
/// This is used heavily by tests so command parsing and dispatch can be exercised without spawning
/// a subprocess.
fn run_from<I, T>(args: I) -> ExitCode
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let argv: Vec<OsString> = args.into_iter().map(Into::into).collect();
  let requested_json = args_request_json_mode(&argv);

  let cli = match Cli::try_parse_from(argv) {
    Ok(cli) => cli,
    Err(err) => {
      let code = err.exit_code();
      if requested_json && code != 0 {
        let joy_err = crate::error::JoyError::new("cli", "cli_parse_error", err.to_string(), 2);
        let _ = crate::output::print_error(crate::output::OutputMode::Json, "cli", &joy_err);
      } else {
        let _ = err.print();
      }
      return to_exit_code(code);
    },
  };

  let mode = cli.output_mode();
  let runtime = cli.runtime_flags();
  configure_ui(mode, runtime.ui);
  match dispatch(cli.command, runtime) {
    Ok(result) => {
      if let Err(err) = print_success(mode, &result) {
        eprintln!("failed to write output: {err}");
        return ExitCode::from(1);
      }
      ExitCode::SUCCESS
    },
    Err(err) => {
      if let Err(write_err) = print_error(mode, err.command, &err) {
        eprintln!("failed to write error output: {write_err}");
        return ExitCode::from(1);
      }
      ExitCode::from(err.exit_code)
    },
  }
}

fn to_exit_code(code: i32) -> ExitCode {
  let bounded = code.clamp(0, u8::MAX as i32) as u8;
  ExitCode::from(bounded)
}

fn args_request_json_mode(args: &[OsString]) -> bool {
  args.iter().skip(1).any(|arg| {
    matches!(
      arg.to_str(),
      Some("--json") | Some("--machine") | Some("-j") // future-proof if added
    )
  })
}
