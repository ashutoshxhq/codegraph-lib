pub const FUNCTION_QUERY: &str = "
    (function_declaration) @node
    (function) @node
    (arrow_function) @node
";

pub const METHOD_QUERY: &str = "(method_definition) @node";

pub const CLASS_QUERY: &str = "(class_declaration) @node";

pub const INTERFACE_QUERY: &str = "(interface_declaration) @node";

pub const TYPE_QUERY: &str = "(type_alias_declaration) @node";

pub const VARIABLE_QUERY: &str = "
    (variable_declarator) @node
    (lexical_declaration) @node
    (variable_declaration) @node
";

pub const CALL_QUERY: &str = "
    (call_expression
        function: [
            (identifier) @func_name
            (member_expression property: (property_identifier) @func_name)
        ]
    )
";

pub const REFERENCE_QUERY: &str = "
    (identifier) @reference
    (property_identifier) @reference
    (type_identifier) @reference
";

pub const IMPORT_QUERY: &str = "
    (import_statement source: (_) @import_path)
";
