//! Curated recipe index and package recipe parsing.
//!
//! Recipes encode build metadata for third-party C++ libraries (headers, transitive deps, CMake
//! configure/build targets, and link metadata) so `joy` can build them reproducibly.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::package_id::{PackageId, PackageIdError};

/// Loaded curated recipe store rooted at `recipes/`.
#[derive(Debug, Clone)]
pub struct RecipeStore {
  root_dir: PathBuf,
  index: RecipeIndexFile,
  recipes_by_id: BTreeMap<String, PackageRecipe>,
}

impl RecipeStore {
  /// Load the repository-local recipe index bundled with `joy`.
  pub fn load_default() -> Result<Self, RecipeError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("recipes");
    Self::load_from_dir(&root)
  }

  /// Load a recipe store from an explicit directory (primarily used in tests).
  pub fn load_from_dir(root_dir: &Path) -> Result<Self, RecipeError> {
    let index_path = root_dir.join("index.toml");
    let raw = fs::read_to_string(&index_path).map_err(|source| RecipeError::Io {
      action: "reading recipe index".into(),
      path: index_path.clone(),
      source,
    })?;
    let index: RecipeIndexFile = toml::from_str(&raw).map_err(|source| RecipeError::Parse {
      path: index_path.clone(),
      source: Box::new(source),
    })?;

    if index.version != 1 {
      return Err(RecipeError::UnsupportedIndexVersion(index.version));
    }

    let mut seen_index_ids = BTreeSet::new();
    let mut seen_index_slugs = BTreeSet::new();
    let mut recipes_by_id = BTreeMap::new();
    for entry in &index.packages {
      if !seen_index_ids.insert(entry.id.clone()) {
        return Err(RecipeError::Validation(format!("duplicate recipe index id `{}`", entry.id)));
      }
      if !seen_index_slugs.insert(entry.slug.clone()) {
        return Err(RecipeError::Validation(format!(
          "duplicate recipe index slug `{}`",
          entry.slug
        )));
      }
      let recipe_path = entry
        .path
        .as_ref()
        .map(|p| root_dir.join(p))
        .unwrap_or_else(|| root_dir.join("packages").join(format!("{}.toml", entry.slug)));
      let recipe = load_recipe_file(&recipe_path)?;
      validate_recipe(&recipe, entry)?;
      if recipes_by_id.insert(recipe.id.clone(), recipe).is_some() {
        return Err(RecipeError::Validation(format!(
          "duplicate recipe definition for `{}`",
          entry.id
        )));
      }
    }

    Ok(Self { root_dir: root_dir.to_path_buf(), index, recipes_by_id })
  }

  pub fn root_dir(&self) -> &Path {
    &self.root_dir
  }

  /// Parsed `index.toml` backing this store.
  pub fn index(&self) -> &RecipeIndexFile {
    &self.index
  }

  /// Lookup a recipe by canonical package ID string.
  pub fn get_by_id(&self, id: &str) -> Option<&PackageRecipe> {
    self.recipes_by_id.get(id)
  }

  /// Lookup a recipe by parsed package ID.
  pub fn get(&self, package: &PackageId) -> Option<&PackageRecipe> {
    self.get_by_id(package.as_str())
  }

  /// Return whether a recipe exists for the given canonical package ID.
  pub fn contains(&self, id: &str) -> bool {
    self.recipes_by_id.contains_key(id)
  }
}

fn load_recipe_file(path: &Path) -> Result<PackageRecipe, RecipeError> {
  let raw = fs::read_to_string(path).map_err(|source| RecipeError::Io {
    action: "reading package recipe".into(),
    path: path.to_path_buf(),
    source,
  })?;
  let recipe: PackageRecipe = toml::from_str(&raw)
    .map_err(|source| RecipeError::Parse { path: path.to_path_buf(), source: Box::new(source) })?;
  Ok(recipe)
}

fn validate_recipe(
  recipe: &PackageRecipe,
  index_entry: &RecipeIndexEntry,
) -> Result<(), RecipeError> {
  let package_id = PackageId::parse(&recipe.id)?;
  if recipe.slug.trim().is_empty() {
    return Err(RecipeError::Validation(format!("recipe `{}` has empty slug", recipe.id)));
  }
  if recipe.id != index_entry.id {
    return Err(RecipeError::Validation(format!(
      "index entry id `{}` does not match recipe id `{}`",
      index_entry.id, recipe.id
    )));
  }
  if recipe.slug != index_entry.slug {
    return Err(RecipeError::Validation(format!(
      "index slug `{}` does not match recipe slug `{}` for `{}`",
      index_entry.slug, recipe.slug, recipe.id
    )));
  }

  for dep in recipe.dep_packages() {
    PackageId::parse(dep.id()).map_err(|_| {
      RecipeError::Validation(format!("recipe `{}` has invalid dep id `{}`", recipe.id, dep.id()))
    })?;
  }

  let _ = package_id;
  Ok(())
}

/// Top-level `recipes/index.toml` schema.
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeIndexFile {
  pub version: u32,
  #[serde(default)]
  pub packages: Vec<RecipeIndexEntry>,
}

/// Index entry describing where a package recipe file lives.
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeIndexEntry {
  pub id: String,
  pub slug: String,
  pub path: Option<String>,
}

/// Parsed package recipe schema.
#[derive(Debug, Clone, Deserialize)]
pub struct PackageRecipe {
  pub id: String,
  pub slug: String,
  pub source: RecipeSource,
  #[serde(default)]
  pub kind: Option<RecipeKind>,
  #[serde(default)]
  pub fetch: Option<FetchSection>,
  #[serde(default)]
  pub headers: Option<HeadersSection>,
  #[serde(default)]
  pub deps: Option<DepsSection>,
  #[serde(default)]
  pub cmake: Option<CmakeSection>,
  #[serde(default)]
  pub link: Option<LinkSection>,
}

impl PackageRecipe {
  /// Include roots exported by the package recipe.
  pub fn include_roots(&self) -> &[String] {
    self.headers.as_ref().map(|h| h.include_roots.as_slice()).unwrap_or(&[])
  }

  /// Transitive package dependencies declared by the recipe.
  pub fn dep_packages(&self) -> &[RecipeDependency] {
    self.deps.as_ref().map(|d| d.packages.as_slice()).unwrap_or(&[])
  }

  /// Whether the package should be treated as header-only by the resolver/build pipeline.
  pub fn is_header_only(&self) -> bool {
    match (&self.kind, &self.link) {
      (Some(RecipeKind::HeaderOnly), _) => true,
      (_, Some(link)) if !link.libs.is_empty() => false,
      (Some(RecipeKind::Cmake), _) => false,
      _ => true,
    }
  }
}

/// Supported recipe source backends.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecipeSource {
  Github,
}

/// Package build strategy described by a recipe.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecipeKind {
  HeaderOnly,
  Cmake,
}

/// Optional source fetch customization.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FetchSection {
  #[serde(default)]
  pub subdir: String,
}

/// Header export metadata.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HeadersSection {
  #[serde(default)]
  pub include_roots: Vec<String>,
}

/// Transitive dependency list for a recipe.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DepsSection {
  #[serde(default)]
  pub packages: Vec<RecipeDependency>,
}

/// Recipe dependency entry supporting short and detailed forms.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RecipeDependency {
  Id(String),
  Detailed { id: String, rev: Option<String> },
}

impl RecipeDependency {
  /// Canonical dependency package ID.
  pub fn id(&self) -> &str {
    match self {
      Self::Id(id) => id,
      Self::Detailed { id, .. } => id,
    }
  }

  /// Optional exact revision requested by this dependency edge.
  pub fn requested_rev(&self) -> Option<&str> {
    match self {
      Self::Id(_) => None,
      Self::Detailed { rev, .. } => rev.as_deref(),
    }
  }
}

/// CMake-specific build metadata used by the Phase 5 adapter.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CmakeSection {
  #[serde(default)]
  pub configure_args: Vec<String>,
  #[serde(default)]
  pub build_targets: Vec<String>,
}

/// Linker metadata for the final project build.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LinkSection {
  #[serde(default)]
  pub libs: Vec<String>,
  #[serde(default)]
  pub preferred_linkage: Option<Linkage>,
}

/// Preferred linkage mode for recipe-built libraries.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Linkage {
  Static,
  Shared,
}

/// Errors produced while loading or validating recipe metadata.
#[derive(Debug, Error)]
pub enum RecipeError {
  #[error("filesystem error while {action} at `{path}`: {source}")]
  Io {
    action: String,
    path: PathBuf,
    #[source]
    source: std::io::Error,
  },
  #[error("failed to parse recipe TOML `{path}`: {source}")]
  Parse {
    path: PathBuf,
    #[source]
    source: Box<toml::de::Error>,
  },
  #[error("unsupported recipe index version `{0}`")]
  UnsupportedIndexVersion(u32),
  #[error("invalid recipe: {0}")]
  Validation(String),
  #[error(transparent)]
  PackageId(#[from] PackageIdError),
}

#[cfg(test)]
mod tests {
  use std::fs;

  use tempfile::TempDir;

  use super::{RecipeKind, RecipeStore};

  #[test]
  fn loads_recipe_store_and_package_recipes() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(temp.path().join("packages")).expect("packages dir");

    fs::write(
      temp.path().join("index.toml"),
      r#"version = 1

[[packages]]
id = "fmtlib/fmt"
slug = "fmt"
"#,
    )
    .expect("write index");

    fs::write(
      temp.path().join("packages/fmt.toml"),
      r#"id = "fmtlib/fmt"
slug = "fmt"
source = "github"
kind = "cmake"

[headers]
include_roots = ["include"]

[deps]
packages = [
  { id = "zlib-ng/zlib-ng", rev = "2.1.6" },
]

[cmake]
configure_args = ["-DBUILD_SHARED_LIBS=OFF"]
build_targets = ["fmt"]

[link]
libs = ["fmt"]
preferred_linkage = "static"
"#,
    )
    .expect("write recipe");

    let store = RecipeStore::load_from_dir(temp.path()).expect("load store");
    let recipe = store.get_by_id("fmtlib/fmt").expect("fmt recipe");

    assert_eq!(recipe.slug, "fmt");
    assert_eq!(recipe.kind, Some(RecipeKind::Cmake));
    assert_eq!(recipe.include_roots(), ["include"]);
    assert_eq!(recipe.dep_packages().len(), 1);
    assert!(!recipe.is_header_only());
  }

  #[test]
  fn validates_index_entry_against_recipe() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(temp.path().join("packages")).expect("packages dir");
    fs::write(
      temp.path().join("index.toml"),
      r#"version = 1

[[packages]]
id = "nlohmann/json"
slug = "nlohmann_json"
"#,
    )
    .expect("write index");
    fs::write(
      temp.path().join("packages/nlohmann_json.toml"),
      r#"id = "nlohmann/json"
slug = "json"
source = "github"
"#,
    )
    .expect("write recipe");

    let err = RecipeStore::load_from_dir(temp.path()).expect_err("mismatch should fail");
    assert!(err.to_string().contains("index slug"));
  }

  #[test]
  fn rejects_duplicate_index_ids_and_slugs() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(temp.path().join("packages")).expect("packages dir");
    fs::write(
      temp.path().join("index.toml"),
      r#"version = 1

[[packages]]
id = "nlohmann/json"
slug = "nlohmann_json"

[[packages]]
id = "nlohmann/json"
slug = "nlohmann_json_alt"
"#,
    )
    .expect("write index");
    fs::write(
      temp.path().join("packages/nlohmann_json.toml"),
      r#"id = "nlohmann/json"
slug = "nlohmann_json"
source = "github"
"#,
    )
    .expect("write recipe");
    fs::write(
      temp.path().join("packages/nlohmann_json_alt.toml"),
      r#"id = "nlohmann/json"
slug = "nlohmann_json_alt"
source = "github"
"#,
    )
    .expect("write alt recipe");

    let err = RecipeStore::load_from_dir(temp.path()).expect_err("duplicate id should fail");
    assert!(err.to_string().contains("duplicate recipe index id"));
  }
}
