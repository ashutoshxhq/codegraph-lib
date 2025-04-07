use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Parser, Tree};

pub mod bindings;
pub mod languages;
pub mod queries;

pub struct TreeSitterParser {
    parser: Parser,
    language_parsers: HashMap<String, Language>,
}

impl TreeSitterParser {
    pub fn new() -> Self {
        let parser = Parser::new();
        let language_parsers = languages::get_language_parsers();

        Self {
            parser,
            language_parsers,
        }
    }

    pub fn parse_file(&mut self, file_path: &Path, content: &str) -> Option<(Tree, String)> {
        let language_name = self.detect_language(file_path)?;
        let language = self.language_parsers.get(&language_name)?.clone();

        self.parser.set_language(&language).ok()?;
        let tree = self.parser.parse(content.as_bytes(), None)?;

        Some((tree, language_name))
    }

    pub fn detect_language(&self, file_path: &Path) -> Option<String> {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            return languages::detect_language_from_extension(ext);
        }
        None
    }

    pub fn get_supported_extensions() -> Vec<&'static str> {
        languages::get_supported_extensions()
    }
}
