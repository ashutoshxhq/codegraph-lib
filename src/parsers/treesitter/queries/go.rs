pub const FUNCTION_QUERY: &str = "(function_declaration) @node";

pub const METHOD_QUERY: &str = "(method_declaration) @node";

pub const CLASS_QUERY: &str = "(type_spec type: (struct_type)) @node";

pub const VARIABLE_QUERY: &str = "
    (var_declaration) @node
    (const_declaration) @node
    (var_spec) @node
    (const_spec) @node
";

pub const CALL_QUERY: &str = "
    (call_expression function: [
        (identifier) @func_name
        (selector_expression field: (field_identifier) @func_name)
    ])
";

pub const REFERENCE_QUERY: &str = "
    (identifier) @reference
    (field_identifier) @reference
";

pub const IMPORT_QUERY: &str = "
    (import_spec path: (_) @import_path)
";
