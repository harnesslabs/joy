use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::package_id::{PackageId, PackageIdError};

#[derive(Debug, Clone)]
pub struct RecipeStore {
  root_dir: PathBuf,
  index: RecipeIndexFile,
  recipes_by_id: BTreeMap<String, PackageRecipe>,
}

impl RecipeStore {
  pub fn load_default() -> Result<Self, RecipeError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("recipes");
    Self::load_from_dir(&root)
  }

  pub fn load_from_dir(root_dir: &Path) -> Result<Self, RecipeError> {
    let index_path = root_dir.join("index.toml");
    let raw = fs::read_to_string(&index_path).map_err(|source| RecipeError::Io {
      action: "reading recipe index".into(),
      path: index_path.clone(),
      source,
    })?;
    let index: RecipeIndexFile = toml::from_str(&raw)
      .map_err(|source| RecipeError::Parse { path: index_path.clone(), source })?;

    if index.version != 1 {
      return Err(RecipeError::UnsupportedIndexVersion(index.version));
    }

    let mut recipes_by_id = BTreeMap::new();
    for entry in &index.packages {
      let recipe_path = entry
        .path
        .as_ref()
        .map(|p| root_dir.join(p))
        .unwrap_or_else(|| root_dir.join("packages").join(format!("{}.toml", entry.slug)));
      let recipe = load_recipe_file(&recipe_path)?;
      validate_recipe(&recipe, entry)?;
      recipes_by_id.insert(recipe.id.clone(), recipe);
    }

    Ok(Self { root_dir: root_dir.to_path_buf(), index, recipes_by_id })
  }

  pub fn root_dir(&self) -> &Path {
    &self.root_dir
  }

  pub fn index(&self) -> &RecipeIndexFile {
    &self.index
  }

  pub fn get_by_id(&self, id: &str) -> Option<&PackageRecipe> {
    self.recipes_by_id.get(id)
  }

  pub fn get(&self, package: &PackageId) -> Option<&PackageRecipe> {
    self.get_by_id(package.as_str())
  }

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
    .map_err(|source| RecipeError::Parse { path: path.to_path_buf(), source })?;
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

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeIndexFile {
  pub version: u32,
  #[serde(default)]
  pub packages: Vec<RecipeIndexEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeIndexEntry {
  pub id: String,
  pub slug: String,
  pub path: Option<String>,
}

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
  pub fn include_roots(&self) -> &[String] {
    self.headers.as_ref().map(|h| h.include_roots.as_slice()).unwrap_or(&[])
  }

  pub fn dep_packages(&self) -> &[RecipeDependency] {
    self.deps.as_ref().map(|d| d.packages.as_slice()).unwrap_or(&[])
  }

  pub fn is_header_only(&self) -> bool {
    match (&self.kind, &self.link) {
      (Some(RecipeKind::HeaderOnly), _) => true,
      (_, Some(link)) if !link.libs.is_empty() => false,
      (Some(RecipeKind::Cmake), _) => false,
      _ => true,
    }
  }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecipeSource {
  Github,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecipeKind {
  HeaderOnly,
  Cmake,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FetchSection {
  #[serde(default)]
  pub subdir: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HeadersSection {
  #[serde(default)]
  pub include_roots: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DepsSection {
  #[serde(default)]
  pub packages: Vec<RecipeDependency>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RecipeDependency {
  Id(String),
  Detailed { id: String, rev: Option<String> },
}

impl RecipeDependency {
  pub fn id(&self) -> &str {
    match self {
      Self::Id(id) => id,
      Self::Detailed { id, .. } => id,
    }
  }

  pub fn requested_rev(&self) -> Option<&str> {
    match self {
      Self::Id(_) => None,
      Self::Detailed { rev, .. } => rev.as_deref(),
    }
  }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CmakeSection {
  #[serde(default)]
  pub configure_args: Vec<String>,
  #[serde(default)]
  pub build_targets: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LinkSection {
  #[serde(default)]
  pub libs: Vec<String>,
  #[serde(default)]
  pub preferred_linkage: Option<Linkage>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Linkage {
  Static,
  Shared,
}

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
    source: toml::de::Error,
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
}
