use std::{fs, path::Path};

pub fn normalize_file_path_string(path: &str) -> String {
    normalize_file_path_from_path(Path::new(path))
}

pub fn normalize_file_path_from_path(path: &Path) -> String {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical_path.to_string_lossy().replace('\\', "/")
}
