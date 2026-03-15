use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub source_type: SourceType,
    pub parse_error_count: usize,
}

impl ParsedFile {
    pub fn parsed_successfully(&self) -> bool {
        self.parse_error_count == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseFileError {
    UnsupportedExtension,
}

pub fn parse_source_text(source_text: &str, source_type: SourceType) -> ParsedFile {
    let allocator = Allocator::default();
    let parser_output = Parser::new(&allocator, source_text, source_type).parse();

    ParsedFile {
        source_type,
        parse_error_count: parser_output.errors.len(),
    }
}

pub fn parse_source_path(path: &Path, source_text: &str) -> Result<ParsedFile, ParseFileError> {
    let source_type =
        SourceType::from_path(path).map_err(|_| ParseFileError::UnsupportedExtension)?;
    Ok(parse_source_text(source_text, source_type))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ParseFileError, parse_source_path, parse_source_text};
    use oxc_span::SourceType;

    #[test]
    fn parse_source_path_maps_unknown_extensions_to_unsupported_extension_error() {
        let path = Path::new("src/not_supported.txt");

        let error = parse_source_path(path, "const value = 1;")
            .expect_err("unsupported extensions should return an explicit error");

        assert_eq!(error, ParseFileError::UnsupportedExtension);
    }

    #[test]
    fn parse_source_text_reports_errors_for_invalid_syntax() {
        let parsed_file = parse_source_text("const = ;", SourceType::ts());

        assert!(!parsed_file.parsed_successfully());
        assert!(parsed_file.parse_error_count > 0);
    }

    #[test]
    fn parse_source_text_reports_zero_errors_for_valid_syntax() {
        let parsed_file = parse_source_text("const value = 1;", SourceType::ts());

        assert!(parsed_file.parsed_successfully());
        assert_eq!(parsed_file.parse_error_count, 0);
    }
}
