pub mod scan;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use scan::{DiscoverFilesError, discover_source_file_paths};

#[derive(Debug, Clone)]
pub struct SourceFileInput {
    pub path: PathBuf,
    pub source_text: String,
}

#[derive(Debug)]
pub enum LoadSourceFilesError {
    DiscoverFiles(DiscoverFilesError),
    ReadSourceFile { path: PathBuf, source: io::Error },
}

impl std::fmt::Display for LoadSourceFilesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DiscoverFiles(source_error) => {
                write!(formatter, "file discovery failed: {source_error}")
            }
            Self::ReadSourceFile { path, source } => {
                write!(
                    formatter,
                    "failed to read source file {}: {}",
                    path.display(),
                    source
                )
            }
        }
    }
}

impl std::error::Error for LoadSourceFilesError {}

pub fn load_source_files(root_path: &Path) -> Result<Vec<SourceFileInput>, LoadSourceFilesError> {
    let source_file_paths =
        discover_source_file_paths(root_path).map_err(LoadSourceFilesError::DiscoverFiles)?;

    let mut source_files = Vec::with_capacity(source_file_paths.len());

    for source_file_path in source_file_paths {
        let source_text = fs::read_to_string(&source_file_path).map_err(|source_error| {
            LoadSourceFilesError::ReadSourceFile {
                path: source_file_path.clone(),
                source: source_error,
            }
        })?;

        source_files.push(SourceFileInput {
            path: source_file_path,
            source_text,
        });
    }

    Ok(source_files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::load_source_files;

    #[test]
    fn load_source_files_discovers_and_reads_supported_files() {
        let temporary_directory = tempdir().expect("temporary directory should be created");
        let fixture_root = temporary_directory.path();

        fs::create_dir_all(fixture_root.join("src"))
            .expect("fixture src directory should be created");
        fs::write(
            fixture_root.join("src/app.tsx"),
            "export function App() { return <div />; }",
        )
        .expect("tsx file should be written");
        fs::write(
            fixture_root.join("src/helpers.ts"),
            "export const value = 1;",
        )
        .expect("ts file should be written");
        fs::write(fixture_root.join("README.md"), "not source")
            .expect("non-source file should be written");

        let source_files =
            load_source_files(fixture_root).expect("source file load should succeed");

        assert_eq!(source_files.len(), 2);
        assert!(
            source_files
                .iter()
                .all(|source_file| !source_file.source_text.is_empty())
        );
    }
}
