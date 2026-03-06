use serde_json::json;

use crate::cli::{CacheArgs, CacheSubcommand};
use crate::commands::CommandOutput;
use crate::error::JoyError;
use crate::fs_ops;
use crate::global_cache::GlobalCache;
use crate::output::HumanMessageBuilder;

pub fn handle(args: CacheArgs) -> Result<CommandOutput, JoyError> {
  match args.command {
    CacheSubcommand::Gc(gc) => handle_gc(gc.aggressive),
  }
}

fn handle_gc(aggressive: bool) -> Result<CommandOutput, JoyError> {
  let cache = GlobalCache::resolve()
    .map_err(|err| JoyError::new("cache", "cache_setup_failed", err.to_string(), 1))?;
  cache
    .ensure_layout()
    .map_err(|err| JoyError::new("cache", "cache_setup_failed", err.to_string(), 1))?;
  let mut removed = Vec::new();

  for path in [&cache.tmp_root] {
    if fs_ops::remove_path_if_exists(path)
      .map_err(|err| JoyError::io("cache", "removing cache path", path, &err))?
    {
      removed.push(path.display().to_string());
    }
  }
  if aggressive {
    for path in [&cache.src_root, &cache.archives_root] {
      if fs_ops::remove_path_if_exists(path)
        .map_err(|err| JoyError::io("cache", "removing cache path", path, &err))?
      {
        removed.push(path.display().to_string());
      }
    }
  }
  cache
    .ensure_layout()
    .map_err(|err| JoyError::new("cache", "cache_setup_failed", err.to_string(), 1))?;

  let human = HumanMessageBuilder::new("Garbage-collected joy cache")
    .kv("aggressive", aggressive.to_string())
    .kv("removed paths", removed.len().to_string())
    .build();
  Ok(CommandOutput::new(
    "cache",
    human,
    json!({
      "action": "gc",
      "aggressive": aggressive,
      "removed_paths": removed,
      "cache_root": cache.cache_root.display().to_string(),
    }),
  ))
}
