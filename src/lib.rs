//! `joy` is a native C++ package and build manager with a CLI-first API.
//!
//! The library crate exists primarily to keep the CLI testable: `src/main.rs` is a thin wrapper
//! around [`run`], while command dispatch, manifest parsing, dependency fetching, resolution, and
//! local build orchestration live in modules here.
//!
//! TODO(phase7): Narrow the public module surface to a stable, intentional library API (or mark
//! `joy` as CLI-only) once downstream usage expectations are finalized.

pub mod abi;
pub mod cli;
pub mod cmake;
pub mod commands;
pub mod error;
pub mod fetch;
pub mod global_cache;
pub mod install_index;
pub mod linking;
pub mod lockfile;
pub mod manifest;
pub mod ninja;
pub mod output;
pub mod package_id;
pub mod project_env;
pub mod recipes;
pub mod resolver;
pub mod templates;
pub mod toolchain;

use clap::Parser;
use std::ffi::OsString;
use std::process::ExitCode;

use crate::cli::Cli;
use crate::commands::dispatch;
use crate::output::{print_error, print_success};

/// Run the `joy` CLI using the current process arguments.
pub fn run() -> ExitCode {
  run_from(std::env::args_os())
}

/// Run the `joy` CLI using an explicit argument iterator.
///
/// This is used heavily by tests so command parsing and dispatch can be exercised without spawning
/// a subprocess.
pub fn run_from<I, T>(args: I) -> ExitCode
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let cli = match Cli::try_parse_from(args) {
    Ok(cli) => cli,
    Err(err) => {
      let code = err.exit_code();
      let _ = err.print();
      return to_exit_code(code);
    },
  };

  let mode = cli.output_mode();
  match dispatch(cli.command) {
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
