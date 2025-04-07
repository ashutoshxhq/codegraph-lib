pub const FUNCTION_QUERY: &str = "(function_definition) @node";

pub const CLASS_QUERY: &str = "[(class_specifier) (struct_specifier)] @node";

pub const VARIABLE_QUERY: &str = "
    (declaration) @node
";

pub const CALL_QUERY: &str = "
    (call_expression function: [
        (identifier) @func_name
        (field_expression field: (field_identifier) @func_name)
    ])
";

pub const REFERENCE_QUERY: &str = "
    (identifier) @reference
    (field_identifier) @reference
";

pub const IMPORT_QUERY: &str = "
    (preproc_include path: (_) @import_path)
";
