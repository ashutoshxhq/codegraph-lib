pub const METHOD_QUERY: &str = "(method_declaration) @node";

pub const CLASS_QUERY: &str = "[(class_declaration) (interface_declaration)] @node";

pub const VARIABLE_QUERY: &str = "
    (variable_declarator) @node
    (field_declaration) @node
";

pub const CALL_QUERY: &str = "
    (method_invocation name: (identifier) @func_name)
    (method_reference name: (identifier) @func_name)
";

pub const REFERENCE_QUERY: &str = "
    (identifier) @reference
    (field_access field: (identifier) @reference)
";

pub const IMPORT_QUERY: &str = "
    (import_declaration name: (_) @import_path)
";
