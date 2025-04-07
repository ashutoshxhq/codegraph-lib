pub const FUNCTION_QUERY: &str = "(function_definition) @node";

pub const CLASS_QUERY: &str = "(class_definition) @node";

pub const VARIABLE_QUERY: &str = "
    (assignment) @node
    (global_statement) @node
";

pub const CALL_QUERY: &str = "
    (call function: [
        (identifier) @func_name
        (attribute attribute: (identifier) @func_name)
    ])
";

pub const REFERENCE_QUERY: &str = "
    (identifier) @reference
    (attribute attribute: (identifier) @reference)
";

pub const IMPORT_QUERY: &str = "
    (import_statement name: (_) @import_path)
    (import_from_statement module_name: (_) @import_path)
";
