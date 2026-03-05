use std::collections::BTreeMap;
use std::fs;

use joy::manifest::{DependencySource, DependencySpec, Manifest, ProjectSection};
use joy::recipes::RecipeStore;
use joy::resolver;
use tempfile::TempDir;

#[test]
fn default_recipe_store_covers_representative_header_and_compiled_packages() {
  let store = RecipeStore::load_default().expect("load default recipes");

  for header_only in [
    "nlohmann/json",
    "cliutils/CLI11",
    "Neargye/magic_enum",
    "skypjack/entt",
    "jarro2783/cxxopts",
    "TartanLlama/expected",
    "gabime/spdlog",
  ] {
    let recipe =
      store.get_by_id(header_only).unwrap_or_else(|| panic!("missing recipe {header_only}"));
    assert!(recipe.is_header_only(), "{header_only} should be header-only");
    assert!(!recipe.include_roots().is_empty(), "{header_only} should declare include roots");
  }

  for compiled in ["fmtlib/fmt", "madler/zlib"] {
    let recipe = store.get_by_id(compiled).unwrap_or_else(|| panic!("missing recipe {compiled}"));
    assert!(!recipe.is_header_only(), "{compiled} should be compiled");
    assert!(recipe.link.as_ref().is_some_and(|link| !link.libs.is_empty()));
  }

  let spdlog = store.get_by_id("gabime/spdlog").expect("spdlog recipe");
  assert!(
    spdlog
      .dep_packages()
      .iter()
      .any(|dep| dep.id() == "fmtlib/fmt" && dep.requested_rev() == Some("11.0.2"))
  );
}

#[test]
fn local_recipe_fixture_smoke_resolves_transitive_chain_deterministically() {
  let temp = TempDir::new().expect("tempdir");
  fs::create_dir_all(temp.path().join("packages")).expect("packages dir");

  fs::write(
    temp.path().join("index.toml"),
    r#"version = 1

[[packages]]
id = "demo/root"
slug = "root"

[[packages]]
id = "demo/mid"
slug = "mid"

[[packages]]
id = "demo/leaf"
slug = "leaf"
"#,
  )
  .expect("write index");

  fs::write(
    temp.path().join("packages/root.toml"),
    r#"id = "demo/root"
slug = "root"
source = "github"
kind = "cmake"

[headers]
include_roots = ["include"]

[deps]
packages = [
  { id = "demo/mid", rev = "v1" },
]

[cmake]
build_targets = ["root"]

[link]
libs = ["root"]
"#,
  )
  .expect("write root recipe");
  fs::write(
    temp.path().join("packages/mid.toml"),
    r#"id = "demo/mid"
slug = "mid"
source = "github"
kind = "header_only"

[headers]
include_roots = ["include"]

[deps]
packages = [
  { id = "demo/leaf", rev = "v2" },
]
"#,
  )
  .expect("write mid recipe");
  fs::write(
    temp.path().join("packages/leaf.toml"),
    r#"id = "demo/leaf"
slug = "leaf"
source = "github"
kind = "cmake"

[headers]
include_roots = ["include"]

[cmake]
build_targets = ["leaf"]

[link]
libs = ["leaf"]
"#,
  )
  .expect("write leaf recipe");

  let store = RecipeStore::load_from_dir(temp.path()).expect("load recipe store");
  let mut deps = BTreeMap::new();
  deps.insert(
    "demo/root".to_string(),
    DependencySpec {
      source: DependencySource::Github,
      rev: "HEAD".to_string(),
      version: None,
      ..DependencySpec::default()
    },
  );
  let manifest = Manifest {
    project: ProjectSection {
      name: "demo".to_string(),
      version: "0.1.0".to_string(),
      cpp_standard: "c++20".to_string(),
      entry: "src/main.cpp".to_string(),
      extra_sources: Vec::new(),
      include_dirs: Vec::new(),
      targets: Vec::new(),
    },
    dependencies: deps,
  };

  let resolved = resolver::resolve_manifest_with(&manifest, &store, |pkg, rev| {
    Ok(format!("{}::{rev}", pkg.as_str()))
  })
  .expect("resolve");

  let order = resolved.build_order_ids().expect("build order");
  assert_eq!(order, vec!["demo/leaf", "demo/mid", "demo/root"]);
  let root = resolved.package("demo/root").expect("root");
  assert!(!root.header_only);
  let mid = resolved.package("demo/mid").expect("mid");
  assert!(mid.header_only);
  let leaf = resolved.package("demo/leaf").expect("leaf");
  assert!(!leaf.header_only);
}
