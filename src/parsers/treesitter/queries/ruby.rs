pub const METHOD_QUERY: &str = "(method) @node";

pub const CLASS_QUERY: &str = "[(class) (module)] @node";

pub const VARIABLE_QUERY: &str = "
    (assignment) @node
    (instance_variable) @node
    (class_variable) @node
    (constant) @node
";

pub const CALL_QUERY: &str = "
    (call method: (identifier) @func_name)
    (method_call method: (identifier) @func_name)
";

pub const REFERENCE_QUERY: &str = "
    (identifier) @reference
    (constant) @reference
    (instance_variable) @reference
    (class_variable) @reference
";

pub const IMPORT_QUERY: &str = "
    (call
        method: (identifier)
        arguments: (argument_list (_) @import_path))
";
