pub mod parse;
pub mod scan;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use parse::{ParseFileError, ParsedFile, parse_source_path};
use scan::{DiscoverFilesError, discover_source_file_paths};

#[derive(Debug, Clone)]
pub struct ParsedSourceFile {
    pub path: PathBuf,
    pub source_text: String,
    pub parsed_file: ParsedFile,
}

#[derive(Debug)]
pub enum ParseProjectError {
    DiscoverFiles(DiscoverFilesError),
    ReadSourceFile {
        path: PathBuf,
        source: io::Error,
    },
    ParseSourceFile {
        path: PathBuf,
        source: ParseFileError,
    },
}

impl std::fmt::Display for ParseProjectError {
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
            Self::ParseSourceFile { path, source } => write!(
                formatter,
                "failed to parse source file {}: {:?}",
                path.display(),
                source
            ),
        }
    }
}

impl std::error::Error for ParseProjectError {}

pub fn parse_project_source_files(
    root_path: &Path,
) -> Result<Vec<ParsedSourceFile>, ParseProjectError> {
    let source_file_paths =
        discover_source_file_paths(root_path).map_err(ParseProjectError::DiscoverFiles)?;

    let mut parsed_source_files = Vec::with_capacity(source_file_paths.len());

    for source_file_path in source_file_paths {
        let source_text = fs::read_to_string(&source_file_path).map_err(|source_error| {
            ParseProjectError::ReadSourceFile {
                path: source_file_path.clone(),
                source: source_error,
            }
        })?;

        let parsed_file =
            parse_source_path(&source_file_path, &source_text).map_err(|source_error| {
                ParseProjectError::ParseSourceFile {
                    path: source_file_path.clone(),
                    source: source_error,
                }
            })?;

        parsed_source_files.push(ParsedSourceFile {
            path: source_file_path,
            source_text,
            parsed_file,
        });
    }

    Ok(parsed_source_files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::parse_project_source_files;

    #[test]
    fn parse_project_source_files_discovers_and_parses_supported_files() {
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

        let parsed_source_files =
            parse_project_source_files(fixture_root).expect("project parse should succeed");

        assert_eq!(parsed_source_files.len(), 2);
        assert!(
            parsed_source_files
                .iter()
                .all(|parsed_source_file| parsed_source_file.parsed_file.parse_error_count == 0)
        );
    }
}
