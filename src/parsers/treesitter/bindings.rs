use tree_sitter::Language;

pub fn rust_language() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

pub fn python_language() -> Language {
    tree_sitter_python::LANGUAGE.into()
}

pub fn javascript_language() -> Language {
    tree_sitter_javascript::LANGUAGE.into()
}

pub fn typescript_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

pub fn tsx_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TSX.into()
}

pub fn cpp_language() -> Language {
    tree_sitter_cpp::LANGUAGE.into()
}

pub fn c_language() -> Language {
    tree_sitter_c::LANGUAGE.into()
}

pub fn java_language() -> Language {
    tree_sitter_java::LANGUAGE.into()
}

pub fn go_language() -> Language {
    tree_sitter_go::LANGUAGE.into()
}

pub fn ruby_language() -> Language {
    tree_sitter_ruby::LANGUAGE.into()
}

pub fn php_language() -> Language {
    tree_sitter_php::LANGUAGE_PHP_ONLY.into()
}
