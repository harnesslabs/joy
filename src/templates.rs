pub fn joy_toml(project_name: &str) -> String {
  format!(
    r#"[project]
name = "{project_name}"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"
# extra_sources = ["src/lib.cpp", "src/feature/main.cpp"]
# include_dirs = ["include"]
# [[project.targets]]
# name = "tool"
# entry = "src/tool.cpp"
# extra_sources = ["src/shared.cpp"]
# include_dirs = ["tools/include"]

[dependencies]
"#
  )
}

pub fn main_cpp() -> &'static str {
  r#"#include <iostream>

int main() {
  std::cout << "Hello from joy!" << std::endl;
  return 0;
}
"#
}

pub fn gitignore() -> &'static str {
  r#".joy/
build/
*.o
*.obj
*.exe
"#
}

#[cfg(test)]
mod tests {
  use super::{gitignore, joy_toml, main_cpp};

  #[test]
  fn joy_toml_template_matches_expected_defaults() {
    let rendered = joy_toml("demo");
    assert_eq!(
      rendered,
      r#"[project]
name = "demo"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"
# extra_sources = ["src/lib.cpp", "src/feature/main.cpp"]
# include_dirs = ["include"]
# [[project.targets]]
# name = "tool"
# entry = "src/tool.cpp"
# extra_sources = ["src/shared.cpp"]
# include_dirs = ["tools/include"]

[dependencies]
"#
    );
  }

  #[test]
  fn main_cpp_template_contains_basic_program() {
    let template = main_cpp();
    assert!(template.contains("#include <iostream>"));
    assert!(template.contains("Hello from joy!"));
  }

  #[test]
  fn gitignore_template_includes_local_joy_dir() {
    let template = gitignore();
    assert!(template.lines().any(|line| line == ".joy/"));
  }
}
