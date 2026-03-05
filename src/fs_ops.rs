use std::fs;
use std::path::Path;

pub fn remove_path_if_exists(path: &Path) -> std::io::Result<bool> {
  match fs::symlink_metadata(path) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() || metadata.is_file() {
        match fs::remove_file(path) {
          Ok(()) => {},
          Err(err)
            if matches!(
              err.kind(),
              std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::IsADirectory
            ) =>
          {
            fs::remove_dir_all(path)?;
          },
          Err(err) => return Err(err),
        }
      } else if metadata.is_dir() {
        fs::remove_dir_all(path)?;
      } else {
        fs::remove_file(path)?;
      }
      Ok(true)
    },
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
    Err(err) => Err(err),
  }
}

#[cfg(test)]
mod tests {
  use tempfile::TempDir;

  use super::remove_path_if_exists;

  #[test]
  fn removes_file_path_when_present() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("file.txt");
    std::fs::write(&file, "data").expect("write file");

    let removed = remove_path_if_exists(&file).expect("remove file");
    assert!(removed);
    assert!(!file.exists());
  }

  #[test]
  fn removes_directory_path_when_present() {
    let temp = TempDir::new().expect("tempdir");
    let dir = temp.path().join("dir");
    std::fs::create_dir_all(dir.join("nested")).expect("create dir");
    std::fs::write(dir.join("nested").join("file.txt"), "data").expect("write nested file");

    let removed = remove_path_if_exists(&dir).expect("remove dir");
    assert!(removed);
    assert!(!dir.exists());
  }

  #[test]
  fn returns_false_for_missing_path() {
    let temp = TempDir::new().expect("tempdir");
    let missing = temp.path().join("missing");
    let removed = remove_path_if_exists(&missing).expect("remove missing");
    assert!(!removed);
  }
}
