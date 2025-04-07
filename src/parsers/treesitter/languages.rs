use std::collections::HashMap;
use tree_sitter::Language;

use super::bindings;

pub fn get_language_parsers() -> HashMap<String, Language> {
    let mut parsers = HashMap::new();

    parsers.insert("rust".to_string(), bindings::rust_language());
    parsers.insert("python".to_string(), bindings::python_language());
    parsers.insert("javascript".to_string(), bindings::javascript_language());
    parsers.insert("typescript".to_string(), bindings::typescript_language());
    parsers.insert("tsx".to_string(), bindings::tsx_language());
    parsers.insert("cpp".to_string(), bindings::cpp_language());
    parsers.insert("c".to_string(), bindings::c_language());
    parsers.insert("java".to_string(), bindings::java_language());
    parsers.insert("go".to_string(), bindings::go_language());
    parsers.insert("ruby".to_string(), bindings::ruby_language());
    parsers.insert("php".to_string(), bindings::php_language());

    parsers
}

pub fn detect_language_from_extension(extension: &str) -> Option<String> {
    match extension {
        "rs" => Some("rust".to_string()),
        "py" => Some("python".to_string()),
        "js" => Some("javascript".to_string()),
        "jsx" => Some("javascript".to_string()),
        "ts" => Some("typescript".to_string()),
        "tsx" => Some("tsx".to_string()),
        "java" => Some("java".to_string()),
        "c" => Some("c".to_string()),
        "h" => Some("c".to_string()),
        "cpp" | "cc" | "cxx" => Some("cpp".to_string()),
        "hpp" => Some("cpp".to_string()),
        "go" => Some("go".to_string()),
        "rb" => Some("ruby".to_string()),
        "php" => Some("php".to_string()),
        _ => None,
    }
}

pub fn get_supported_extensions() -> Vec<&'static str> {
    vec![
        "rs", "py", "js", "jsx", "ts", "tsx", "java", "c", "cpp", "cc", "cxx", "hpp", "h", "go",
        "rb", "php",
    ]
}
