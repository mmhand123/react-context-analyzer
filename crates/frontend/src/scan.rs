use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SUPPORTED_EXTENSIONS: [&str; 4] = ["js", "jsx", "ts", "tsx"];

#[derive(Debug)]
pub enum DiscoverFilesError {
    RootPathIsNotDirectory { root_path: PathBuf },
    ReadDirectoryFailed { path: PathBuf, source: io::Error },
    ReadDirectoryEntryFailed { path: PathBuf, source: io::Error },
}

impl std::fmt::Display for DiscoverFilesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootPathIsNotDirectory { root_path } => {
                write!(
                    formatter,
                    "root path is not a directory: {}",
                    root_path.display()
                )
            }
            Self::ReadDirectoryFailed { path, source } => write!(
                formatter,
                "failed to read directory {}: {}",
                path.display(),
                source
            ),
            Self::ReadDirectoryEntryFailed { path, source } => write!(
                formatter,
                "failed to read entry in directory {}: {}",
                path.display(),
                source
            ),
        }
    }
}

impl std::error::Error for DiscoverFilesError {}

pub fn discover_source_file_paths(root_path: &Path) -> Result<Vec<PathBuf>, DiscoverFilesError> {
    if !root_path.is_dir() {
        return Err(DiscoverFilesError::RootPathIsNotDirectory {
            root_path: root_path.to_path_buf(),
        });
    }

    let mut discovered_paths = Vec::new();
    collect_source_files_recursively(root_path, &mut discovered_paths)?;
    discovered_paths.sort();
    Ok(discovered_paths)
}

pub fn is_supported_source_file(path: &Path) -> bool {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) => SUPPORTED_EXTENSIONS
            .iter()
            .any(|supported_extension| extension.eq_ignore_ascii_case(supported_extension)),
        None => false,
    }
}

fn collect_source_files_recursively(
    directory_path: &Path,
    discovered_paths: &mut Vec<PathBuf>,
) -> Result<(), DiscoverFilesError> {
    let directory_entries = fs::read_dir(directory_path).map_err(|source_error| {
        DiscoverFilesError::ReadDirectoryFailed {
            path: directory_path.to_path_buf(),
            source: source_error,
        }
    })?;

    for directory_entry_result in directory_entries {
        let directory_entry = directory_entry_result.map_err(|source_error| {
            DiscoverFilesError::ReadDirectoryEntryFailed {
                path: directory_path.to_path_buf(),
                source: source_error,
            }
        })?;

        let entry_path = directory_entry.path();
        let file_type = directory_entry.file_type().map_err(|source_error| {
            DiscoverFilesError::ReadDirectoryEntryFailed {
                path: directory_path.to_path_buf(),
                source: source_error,
            }
        })?;

        if file_type.is_dir() {
            collect_source_files_recursively(&entry_path, discovered_paths)?;
            continue;
        }

        if file_type.is_file() && is_supported_source_file(&entry_path) {
            discovered_paths.push(entry_path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{discover_source_file_paths, is_supported_source_file};

    #[test]
    fn supported_extension_detection_handles_expected_source_types() {
        assert!(is_supported_source_file(Path::new("src/file.ts")));
        assert!(is_supported_source_file(Path::new("src/file.tsx")));
        assert!(is_supported_source_file(Path::new("src/file.js")));
        assert!(is_supported_source_file(Path::new("src/file.jsx")));
        assert!(is_supported_source_file(Path::new("src/file.TSX")));

        assert!(!is_supported_source_file(Path::new("src/file.rs")));
        assert!(!is_supported_source_file(Path::new("src/file.txt")));
        assert!(!is_supported_source_file(Path::new("src/file")));
    }

    #[test]
    fn discover_source_file_paths_returns_only_supported_files_in_stable_order() {
        let temporary_directory = tempdir().expect("temporary directory should be created");
        let fixture_root = temporary_directory.path();

        fs::create_dir_all(fixture_root.join("src/components"))
            .expect("fixture subdirectory should be created");

        fs::write(
            fixture_root.join("src/components/Profile.tsx"),
            "export function Profile() {}",
        )
        .expect("tsx fixture should be written");
        fs::write(
            fixture_root.join("src/components/Button.tsx"),
            "export function Button() {}",
        )
        .expect("tsx fixture should be written");
        fs::write(fixture_root.join("src/index.ts"), "export {}")
            .expect("ts fixture should be written");
        fs::write(fixture_root.join("README.md"), "not source")
            .expect("markdown fixture should be written");

        let discovered_paths =
            discover_source_file_paths(fixture_root).expect("file discovery should succeed");

        let relative_paths: Vec<String> = discovered_paths
            .iter()
            .map(|path| {
                path.strip_prefix(fixture_root)
                    .expect("path should be under fixture root")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();

        assert_eq!(
            relative_paths,
            vec![
                "src/components/Button.tsx".to_string(),
                "src/components/Profile.tsx".to_string(),
                "src/index.ts".to_string(),
            ]
        );
    }
}
