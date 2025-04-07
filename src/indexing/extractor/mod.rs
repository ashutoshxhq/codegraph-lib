mod common;
mod cpp;
mod go;
mod java;
mod javascript;
mod python;
mod ruby;
mod rust;
mod typescript;

use crate::code_graph::CodeNode;
use log::{debug, error, trace, warn};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

// Trait defining common functionality for language-specific extractors
pub trait LanguageExtractor {
    fn extract_code_units(&self, content: &str, file_path: &Path) -> Vec<CodeNode>;
    fn extract_function_calls(
        &self,
        content: &str,
        func_range: (usize, usize),
        func_name: &str,
    ) -> Vec<String>;
    fn extract_variable_references(
        &self,
        content: &str,
        func_range: (usize, usize),
        var_name: &str,
    ) -> Vec<(usize, usize)>;
    fn extract_imported_modules(&self, content: &str) -> Vec<String>;
}

// Factory function to get the appropriate extractor for a language
pub fn get_extractor_for_language(language: &str) -> Option<Box<dyn LanguageExtractor>> {
    match language {
        "rust" => Some(Box::new(rust::RustExtractor::new())),
        "python" => Some(Box::new(python::PythonExtractor::new())),
        "javascript" => Some(Box::new(javascript::JavaScriptExtractor::new())),
        "typescript" | "tsx" => Some(Box::new(typescript::TypeScriptExtractor::new())),
        "java" => Some(Box::new(java::JavaExtractor::new())),
        "cpp" | "c" => Some(Box::new(cpp::CppExtractor::new())),
        "go" => Some(Box::new(go::GoExtractor::new())),
        "ruby" => Some(Box::new(ruby::RubyExtractor::new())),
        _ => None,
    }
}

// Main function to extract code units from a file
pub fn extract_code_units(file_path: &Path) -> io::Result<Vec<CodeNode>> {
    trace!("Extracting code units from: {:?}", file_path);

    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open file {:?}: {}", file_path, e);
            return Err(e);
        }
    };

    let mut content = String::new();
    if let Err(e) = file.read_to_string(&mut content) {
        error!("Failed to read file {:?}: {}", file_path, e);
        return Err(e);
    }

    // Detect language from file extension
    if let Some(language) = crate::parsers::detect_language(file_path) {
        if let Some(extractor) = get_extractor_for_language(&language) {
            let code_units = extractor.extract_code_units(&content, file_path);
            debug!(
                "Extracted {} code units from {:?}",
                code_units.len(),
                file_path
            );
            return Ok(code_units);
        }
    }

    warn!("Unsupported language for file: {:?}", file_path);
    Ok(Vec::new())
}
