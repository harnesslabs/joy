pub mod add;
pub mod build;
pub mod init;
pub mod new;
pub mod run;

use serde_json::Value;

use crate::cli::Commands;
use crate::error::JoyError;

#[derive(Debug, Clone)]
pub struct CommandOutput {
  pub command: &'static str,
  pub human_message: String,
  pub data: Value,
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
