use std::path::Path;

pub fn detect_language(file_path: &Path) -> Option<String> {
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        return crate::parsers::treesitter::languages::detect_language_from_extension(ext);
    }
    None
}

pub fn get_supported_extensions() -> Vec<&'static str> {
    vec![
        "py", "js", "ts", "jsx", "tsx", "java", "c", "cpp", "cc", "cxx", "hpp", "h", "rs", "go",
        "rb", "php", "swift", "cs", "kt", "kts",
    ]
}
