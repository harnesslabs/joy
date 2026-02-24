use serde_json::json;

use crate::cli::RecipeCheckArgs;
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::recipes::RecipeStore;

pub fn handle(_args: RecipeCheckArgs) -> Result<CommandOutput, JoyError> {
  let store = RecipeStore::load_default()
    .map_err(|err| JoyError::new("recipe-check", "recipe_validation_failed", err.to_string(), 1))?;

  let mut packages =
    store.index().packages.iter().map(|entry| entry.id.clone()).collect::<Vec<_>>();
  packages.sort();

  Ok(CommandOutput::new(
    "recipe-check",
    format!("Validated {} bundled recipes", packages.len()),
    json!({
      "recipes_root": store.root_dir().display().to_string(),
      "recipe_count": packages.len(),
      "packages": packages,
    }),
  ))
}
