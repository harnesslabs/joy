use serde::Serialize;

use crate::cli::RecipeCheckArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::recipes::RecipeStore;

#[derive(Debug, Serialize)]
struct RecipeCheckResponse {
  recipes_root: String,
  recipe_count: usize,
  packages: Vec<String>,
}

pub fn handle(_args: RecipeCheckArgs) -> Result<CommandOutput, JoyError> {
  let store = RecipeStore::load_default()
    .map_err(|err| JoyError::new("recipe-check", "recipe_validation_failed", err.to_string(), 1))?;

  let mut packages =
    store.index().packages.iter().map(|entry| entry.id.clone()).collect::<Vec<_>>();
  packages.sort();

  CommandOutput::from_data(
    "recipe-check",
    format!("Validated {} bundled recipes", packages.len()),
    &RecipeCheckResponse {
      recipes_root: store.root_dir().display().to_string(),
      recipe_count: packages.len(),
      packages,
    },
  )
}
