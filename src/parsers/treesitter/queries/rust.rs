pub const FUNCTION_QUERY: &str = "(function_item) @node";

pub const CLASS_QUERY: &str = "(struct_item) @node";

pub const VARIABLE_QUERY: &str = "
    (let_declaration) @node
    (const_item) @node
    (static_item) @node
";

pub const CALL_QUERY: &str = "
    (call_expression
        function: [
            (identifier) @func_name
            (field_expression field: (field_identifier) @func_name)
            (scoped_identifier name: (identifier) @func_name)
        ]
    )
";

pub const REFERENCE_QUERY: &str = "
    (identifier) @reference
    (field_expression field: (field_identifier) @reference)
    (scoped_identifier name: (identifier) @reference)
";

pub const IMPORT_QUERY: &str = r#"
    (use_declaration
        path: (_) @import_path)
"#;
