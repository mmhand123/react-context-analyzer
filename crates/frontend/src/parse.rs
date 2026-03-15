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
