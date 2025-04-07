use crate::code_graph::{CodeNode, NodeType};
use log::{debug, trace, warn};
use std::collections::HashSet;
use std::path::Path;
use std::str;
use tree_sitter::{Language, Node, Query, QueryCursor, StreamingIterator, Tree};
use uuid::Uuid;

pub struct NodeExtractor {
    pub parsed_tree: Option<Tree>,
    pub source_code: String,
    pub language: String,
    pub file_path: String,
}

impl NodeExtractor {
    pub fn new(tree: Tree, source_code: String, language: String, file_path: &Path) -> Self {
        Self {
            parsed_tree: Some(tree),
            source_code,
            language,
            file_path: file_path.to_string_lossy().into_owned(),
        }
    }

    pub fn extract_code_units(&self) -> Vec<CodeNode> {
        let mut code_units = Vec::new();

        if self.parsed_tree.is_none() {
            warn!("No parsed tree available for extraction");
            return code_units;
        }

        debug!("Extracting code units from {}", self.file_path);

        let tree = self.parsed_tree.as_ref().unwrap();
        let language = tree.language();

        trace!("Extracting functions from {}", self.file_path);
        self.extract_nodes_by_query(
            tree.root_node(),
            &language,
            get_function_query(&self.language),
            NodeType::Function,
            &mut code_units,
        );

        trace!("Extracting classes from {}", self.file_path);
        self.extract_nodes_by_query(
            tree.root_node(),
            &language,
            get_class_query(&self.language),
            NodeType::Class,
            &mut code_units,
        );

        trace!("Extracting methods from {}", self.file_path);
        self.extract_nodes_by_query(
            tree.root_node(),
            &language,
            get_method_query(&self.language),
            NodeType::Method,
            &mut code_units,
        );

        trace!("Extracting variables from {}", self.file_path);
        self.extract_nodes_by_query(
            tree.root_node(),
            &language,
            get_variable_query(&self.language),
            NodeType::Variable,
            &mut code_units,
        );

        self.build_code_relationships(&mut code_units);

        debug!(
            "Extracted {} code units from {}",
            code_units.len(),
            self.file_path
        );
        code_units
    }

    fn build_code_relationships(&self, code_units: &mut Vec<CodeNode>) {
        let mut class_id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for node in code_units.iter() {
            if let NodeType::Class = node.node_type {
                class_id_map.insert(node.name.clone(), node.id.clone());
            }
        }

        for node in code_units.iter_mut() {
            if let NodeType::Method = node.node_type {
                if let Some(parent_class_name) = node.metadata.get("parent_class").cloned() {
                    if let Some(class_id) = class_id_map.get(&parent_class_name) {
                        node.add_metadata("parent_class_id".to_string(), class_id.clone());
                    }
                }
            }
        }
    }

    fn extract_nodes_by_query(
        &self,
        root_node: Node,
        language: &Language,
        query_str: Option<&'static str>,
        node_type: NodeType,
        code_units: &mut Vec<CodeNode>,
    ) {
        if let Some(query_str) = query_str {
            let simple_query_str = match (self.language.as_str(), &node_type) {
                ("rust", NodeType::Class) => "(struct_item) @node",
                ("rust", NodeType::Function) => "(function_item) @node",
                ("rust", NodeType::Method) => "(function_item) @node", // Simplified query
                ("rust", NodeType::Variable) => {
                    "[(let_declaration) (const_item) (static_item)] @node"
                }
                ("python", NodeType::Class) => "(class_definition) @node",
                ("python", NodeType::Function | NodeType::Method) => "(function_definition) @node",
                _ => query_str,
            };

            trace!("Using query for {:?}: {}", node_type, simple_query_str);

            match Query::new(language, simple_query_str) {
                Ok(query) => {
                    let mut query_cursor = QueryCursor::new();
                    let node_idx = query.capture_index_for_name("node").unwrap_or(0);
                    let mut matches =
                        query_cursor.matches(&query, root_node, self.source_code.as_bytes());

                    matches.advance();
                    while let Some(match_result) = matches.get() {
                        for capture in match_result.captures {
                            if capture.index == node_idx {
                                let captured_node = capture.node;

                                if let Some(name) = self.find_node_name(captured_node, &node_type) {
                                    let start_line = captured_node.start_position().row + 1;
                                    let end_line = captured_node.end_position().row + 1;
                                    let node_content = self.get_node_text(captured_node);

                                    // For methods and functions, determine the actual type
                                    let actual_node_type = if node_type == NodeType::Function
                                        || node_type == NodeType::Method
                                    {
                                        if self.is_inside_impl_block(captured_node) {
                                            NodeType::Method
                                        } else {
                                            NodeType::Function
                                        }
                                    } else {
                                        node_type.clone()
                                    };

                                    trace!(
                                        "Found {:?}: {} at lines {}-{}",
                                        actual_node_type, name, start_line, end_line
                                    );

                                    let mut metadata = std::collections::HashMap::new();

                                    if actual_node_type == NodeType::Method {
                                        if let Some(parent_class) =
                                            self.find_parent_class(captured_node)
                                        {
                                            metadata
                                                .insert("parent_class".to_string(), parent_class);
                                        }
                                    }

                                    if matches!(actual_node_type, NodeType::Variable) {
                                        if let Some(var_type) =
                                            self.find_variable_type(captured_node)
                                        {
                                            metadata.insert("type".to_string(), var_type);
                                        }

                                        if let Some(scope) = self.find_variable_scope(captured_node)
                                        {
                                            metadata.insert("scope".to_string(), scope);
                                        }
                                    }

                                    let mut code_node = CodeNode::new(
                                        Uuid::new_v4().to_string(),
                                        actual_node_type,
                                        name,
                                        self.file_path.clone(),
                                        (start_line, end_line),
                                        node_content,
                                    );

                                    for (key, value) in metadata {
                                        code_node.add_metadata(key, value);
                                    }

                                    code_units.push(code_node);
                                }
                            }
                        }
                        matches.advance();
                    }
                }
                Err(e) => {
                    warn!(
                        "Query failed for {:?} in {}: {}",
                        node_type, self.file_path, e
                    );
                    self.extract_nodes_by_traversal(root_node, node_type, code_units);
                }
            }
        } else {
            trace!("No query defined for {:?} in {}", node_type, self.language);
        }
    }

    fn is_inside_impl_block(&self, node: Node) -> bool {
        let mut current = node;

        // First check direct impl_item parent
        let mut parent_iter = current.parent();
        while let Some(parent) = parent_iter {
            match parent.kind() {
                "impl_item" => return true,
                // Also check for block inside impl_item
                "block" => {
                    if let Some(grandparent) = parent.parent() {
                        if grandparent.kind() == "impl_item" {
                            return true;
                        }
                    }
                }
                // Other method container indicators for different languages
                "class_definition" | "class_declaration" => {
                    if current.kind() == "function_definition"
                        || current.kind() == "method_definition"
                    {
                        return true;
                    }
                }
                _ => {}
            }

            current = parent;
            parent_iter = current.parent();
        }

        false
    }

    fn find_parent_class(&self, node: Node) -> Option<String> {
        // Try to find parent impl block first
        let mut current = node;
        let mut parent_iter = current.parent();

        while let Some(parent) = parent_iter {
            if parent.kind() == "impl_item" {
                // First try to find the type directly
                for i in 0..parent.named_child_count() {
                    if let Some(child) = parent.named_child(i) {
                        if child.kind() == "type_identifier" {
                            return Some(self.get_node_text(child));
                        }
                    }
                }

                // If the above fails, check all children more broadly
                for i in 0..parent.child_count() {
                    if let Some(child) = parent.child(i) {
                        if child.kind() == "type_identifier" {
                            return Some(self.get_node_text(child));
                        }

                        // Check generic types
                        if child.kind() == "generic_type" {
                            for j in 0..child.named_child_count() {
                                if let Some(type_child) = child.named_child(j) {
                                    if type_child.kind() == "type_identifier" {
                                        return Some(self.get_node_text(type_child));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            current = parent;
            parent_iter = current.parent();
        }

        // If we can't find impl_item parent, try other class containers
        current = node;
        parent_iter = current.parent();

        while let Some(parent) = parent_iter {
            if parent.kind() == "struct_item"
                || parent.kind() == "class_definition"
                || parent.kind() == "class_declaration"
            {
                for i in 0..parent.named_child_count() {
                    if let Some(child) = parent.named_child(i) {
                        if child.kind() == "identifier" || child.kind() == "type_identifier" {
                            return Some(self.get_node_text(child));
                        }
                    }
                }
            }

            current = parent;
            parent_iter = current.parent();
        }

        None
    }

    fn find_variable_type(&self, node: Node) -> Option<String> {
        match node.kind() {
            "let_declaration" => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "type_annotation" && child.named_child_count() > 0 {
                            if let Some(type_node) = child.named_child(0) {
                                return Some(self.get_node_text(type_node));
                            }
                        }
                    }
                }

                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "=" && i + 1 < node.named_child_count() {
                            if let Some(value_node) = node.named_child(i + 1) {
                                return Some(format!("inferred from: {}", value_node.kind()));
                            }
                        }
                    }
                }
            }
            "const_item" | "static_item" => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == ":" && i + 1 < node.named_child_count() {
                            if let Some(type_node) = node.named_child(i + 1) {
                                return Some(self.get_node_text(type_node));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn find_variable_scope(&self, node: Node) -> Option<String> {
        let mut current = node;
        while let Some(parent) = current.parent() {
            match parent.kind() {
                "function_item" => {
                    for i in 0..parent.named_child_count() {
                        if let Some(child) = parent.named_child(i) {
                            if child.kind() == "identifier" {
                                return Some(format!("function:{}", self.get_node_text(child)));
                            }
                        }
                    }
                }
                "impl_item" => {
                    for i in 0..parent.named_child_count() {
                        if let Some(child) = parent.named_child(i) {
                            if child.kind() == "type_identifier" {
                                return Some(format!("impl:{}", self.get_node_text(child)));
                            }
                        }
                    }
                }
                "struct_item" => {
                    for i in 0..parent.named_child_count() {
                        if let Some(child) = parent.named_child(i) {
                            if child.kind() == "identifier" {
                                return Some(format!("struct:{}", self.get_node_text(child)));
                            }
                        }
                    }
                }
                "source_file" => {
                    return Some("global".to_string());
                }
                _ => {}
            }
            current = parent;
        }
        Some("unknown".to_string())
    }

    fn find_node_name(&self, node: Node, node_type: &NodeType) -> Option<String> {
        if matches!(node_type, NodeType::Variable) {
            match node.kind() {
                "let_declaration" => {
                    for i in 0..node.named_child_count() {
                        if let Some(child) = node.named_child(i) {
                            if child.kind() == "identifier" || child.kind() == "pattern" {
                                return Some(self.get_node_text(child));
                            }
                        }
                    }
                }
                "const_item" | "static_item" => {
                    for i in 0..node.named_child_count() {
                        if let Some(child) = node.named_child(i) {
                            if child.kind() == "identifier" {
                                return Some(self.get_node_text(child));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == "identifier"
                    || child.kind() == "type_identifier"
                    || child.kind() == "field_identifier"
                {
                    return Some(self.get_node_text(child));
                }
            }
        }

        self.find_name_recursively(node)
    }

    fn find_name_recursively(&self, node: Node) -> Option<String> {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == "identifier" || child.kind() == "type_identifier" {
                    return Some(self.get_node_text(child));
                }

                if i < 3 && child.named_child_count() > 0 {
                    if let Some(name) = self.find_name_recursively(child) {
                        return Some(name);
                    }
                }
            }
        }

        None
    }

    fn extract_nodes_by_traversal(
        &self,
        root_node: Node,
        node_type: NodeType,
        code_units: &mut Vec<CodeNode>,
    ) {
        warn!("Using direct traversal to find {:?} nodes", node_type);

        let target_kind = match (self.language.as_str(), &node_type) {
            ("rust", NodeType::Class) => "struct_item",
            ("rust", NodeType::Function) => "function_item",
            ("rust", NodeType::Method) => "function_item",
            ("rust", NodeType::Variable) => "let_declaration",
            ("python", NodeType::Class) => "class_definition",
            ("python", NodeType::Function | NodeType::Method) => "function_definition",
            _ => return,
        };

        let mut visit_count = 0;
        let mut found_count = 0;

        self.visit_node(
            root_node,
            target_kind,
            node_type,
            code_units,
            &mut visit_count,
            &mut found_count,
        );

        trace!(
            "Traversal visited {} nodes, found {} matching nodes",
            visit_count, found_count
        );
    }

    fn visit_node(
        &self,
        node: Node,
        target_kind: &str,
        node_type: NodeType,
        code_units: &mut Vec<CodeNode>,
        visit_count: &mut usize,
        found_count: &mut usize,
    ) {
        *visit_count += 1;

        if node.kind() == target_kind {
            *found_count += 1;

            if let Some(name) = self.find_node_name(node, &node_type) {
                let start_line = node.start_position().row + 1;
                let end_line = node.end_position().row + 1;
                let node_content = self.get_node_text(node);

                let actual_node_type =
                    if node_type == NodeType::Function && self.is_inside_impl_block(node) {
                        NodeType::Method
                    } else {
                        node_type.clone()
                    };

                let mut code_node = CodeNode::new(
                    Uuid::new_v4().to_string(),
                    actual_node_type.clone(),
                    name.clone(),
                    self.file_path.clone(),
                    (start_line, end_line),
                    node_content,
                );

                if actual_node_type == NodeType::Method {
                    if let Some(parent_class) = self.find_parent_class(node) {
                        code_node.add_metadata("parent_class".to_string(), parent_class);
                    }
                }

                trace!(
                    "Found {:?} '{}' at lines {}-{} via traversal",
                    actual_node_type, name, start_line, end_line
                );

                code_units.push(code_node);
            }
        }

        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                self.visit_node(
                    child,
                    target_kind,
                    node_type.clone(),
                    code_units,
                    visit_count,
                    found_count,
                );
            }
        }
    }

    pub fn get_node_text(&self, node: Node) -> String {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        if start_byte >= self.source_code.len() || end_byte > self.source_code.len() {
            return String::new();
        }

        self.source_code[start_byte..end_byte].to_string()
    }

    pub fn extract_function_calls(&self, target_name: &str) -> Vec<(usize, usize)> {
        let mut calls = Vec::new();

        if self.parsed_tree.is_none() {
            return calls;
        }

        let tree = self.parsed_tree.as_ref().unwrap();
        let language = tree.language();

        if let Some(call_query) = get_call_query(&self.language) {
            if let Ok(query) = Query::new(&language, call_query) {
                let mut query_cursor = QueryCursor::new();
                let function_name_idx = query.capture_index_for_name("function_name").unwrap_or(0);
                let mut matches =
                    query_cursor.matches(&query, tree.root_node(), self.source_code.as_bytes());

                matches.advance();
                while let Some(match_result) = matches.get() {
                    for capture in match_result.captures {
                        if capture.index == function_name_idx {
                            let node_text = self.get_node_text(capture.node);

                            if node_text == target_name {
                                let start_pos = capture.node.start_position();
                                let end_pos = capture.node.end_position();
                                calls.push((start_pos.row, end_pos.row));
                            }
                        }
                    }
                    matches.advance();
                }
            }
        }

        calls
    }

    pub fn extract_imports(&self, target_name: &str) -> Vec<(usize, usize)> {
        let mut imports = Vec::new();

        if self.parsed_tree.is_none() {
            return imports;
        }

        let tree = self.parsed_tree.as_ref().unwrap();
        let language = tree.language();

        if let Some(import_query) = get_import_query(&self.language) {
            if let Ok(query) = Query::new(&language, import_query) {
                let mut query_cursor = QueryCursor::new();
                let import_idx = query.capture_index_for_name("import").unwrap_or(0);
                let mut matches =
                    query_cursor.matches(&query, tree.root_node(), self.source_code.as_bytes());

                matches.advance();
                while let Some(match_result) = matches.get() {
                    for capture in match_result.captures {
                        if capture.index == import_idx {
                            let node_text = self.get_node_text(capture.node);

                            if node_text.contains(target_name) {
                                let start_pos = capture.node.start_position();
                                let end_pos = capture.node.end_position();
                                imports.push((start_pos.row, end_pos.row));
                            }
                        }
                    }
                    matches.advance();
                }
            }
        }

        imports
    }

    pub fn extract_references_with_context(
        &self,
        target_name: &str,
    ) -> Vec<(usize, usize, String)> {
        let mut references = Vec::new();

        if self.parsed_tree.is_none() {
            return references;
        }

        let tree = self.parsed_tree.as_ref().unwrap();
        let language = tree.language();

        if let Some(ref_query) = get_reference_query(&self.language) {
            if let Ok(query) = Query::new(&language, ref_query) {
                let mut query_cursor = QueryCursor::new();
                let ref_idx = query.capture_index_for_name("reference").unwrap_or(0);
                let mut matches =
                    query_cursor.matches(&query, tree.root_node(), self.source_code.as_bytes());

                matches.advance();
                while let Some(match_result) = matches.get() {
                    for capture in match_result.captures {
                        if capture.index == ref_idx {
                            let node_text = self.get_node_text(capture.node);

                            if node_text == target_name {
                                let start_pos = capture.node.start_position();
                                let end_pos = capture.node.end_position();

                                let context = if let Some(parent) = capture.node.parent() {
                                    self.determine_reference_context(parent)
                                } else {
                                    "Unknown".to_string()
                                };

                                references.push((start_pos.row, end_pos.row, context));
                            }
                        }
                    }
                    matches.advance();
                }
            }
        }

        references
    }

    pub fn find_explicit_function_calls(&self, function_name: &str) -> Vec<(usize, usize)> {
        let mut calls = Vec::new();

        if self.parsed_tree.is_none() {
            return calls;
        }

        let tree = self.parsed_tree.as_ref().unwrap();
        let language = tree.language();

        let query_str = match self.language.as_str() {
            "rust" => {
                format!(
                    r#"((call_expression
                        function: [
                            (identifier) @func_name (#eq? @func_name "{}")
                            (field_expression field: (field_identifier) @func_name (#eq? @func_name "{}"))
                            (scoped_identifier name: (identifier) @func_name (#eq? @func_name "{}"))
                        ]
                        arguments: (arguments)) @call)"#,
                    function_name, function_name, function_name
                )
            }
            "python" => {
                format!(
                    r#"((call
                        function: [
                            (identifier) @func_name (#eq? @func_name "{}")
                            (attribute attribute: (identifier) @func_name (#eq? @func_name "{}"))
                        ]) @call)"#,
                    function_name, function_name
                )
            }
            "javascript" | "typescript" | "tsx" => {
                format!(
                    r#"((call_expression
                        function: [
                            (identifier) @func_name (#eq? @func_name "{}")
                            (member_expression property: (property_identifier) @func_name (#eq? @func_name "{}"))
                        ]) @call)"#,
                    function_name, function_name
                )
            }
            "java" => {
                format!(
                    r#"((method_invocation
                        name: (identifier) @func_name (#eq? @func_name "{}")) @call)"#,
                    function_name
                )
            }
            "go" => {
                format!(
                    r#"((call_expression
                        function: [
                            (identifier) @func_name (#eq? @func_name "{}")
                            (selector_expression field: (field_identifier) @func_name (#eq? @func_name "{}"))
                        ]) @call)"#,
                    function_name, function_name
                )
            }
            "cpp" | "c" => {
                format!(
                    r#"((call_expression
                        function: [
                            (identifier) @func_name (#eq? @func_name "{}")
                            (field_expression field: (field_identifier) @func_name (#eq? @func_name "{}"))
                        ]) @call)"#,
                    function_name, function_name
                )
            }
            "ruby" => {
                format!(
                    r#"((call
                        method: (identifier) @func_name (#eq? @func_name "{}")) @call)"#,
                    function_name
                )
            }
            "php" => {
                format!(
                    r#"((function_call_expression
                        function: [
                            (name) @func_name (#eq? @func_name "{}")
                            (member_access_expression name: (name) @func_name (#eq? @func_name "{}"))
                        ]) @call)"#,
                    function_name, function_name
                )
            }
            _ => return calls,
        };

        if let Ok(query) = Query::new(&language, &query_str) {
            let mut query_cursor = QueryCursor::new();
            let call_idx = query.capture_index_for_name("call").unwrap_or(0);

            let mut matches =
                query_cursor.matches(&query, tree.root_node(), self.source_code.as_bytes());
            matches.advance();

            while let Some(match_result) = matches.get() {
                for capture in match_result.captures {
                    if capture.index == call_idx {
                        let start_pos = capture.node.start_position();
                        let end_pos = capture.node.end_position();
                        calls.push((start_pos.row, end_pos.row));
                    }
                }
                matches.advance();
            }
        }

        calls
    }

    pub fn extract_imported_modules(&self) -> HashSet<String> {
        let mut modules = HashSet::new();

        if self.parsed_tree.is_none() {
            return modules;
        }

        let tree = self.parsed_tree.as_ref().unwrap();
        let language = tree.language();

        let query_str = match self.language.as_str() {
            "rust" => {
                r#"
                (use_declaration
                    path: (_) @import_path
                )
                (use_declaration
                    (use_tree path: (_) @import_path)
                )
                "#
            }
            "python" => {
                r#"
                (import_statement
                    name: (_) @import_path)
                (import_from_statement
                    module_name: (_) @import_path)
                "#
            }
            "javascript" | "typescript" | "tsx" => r#"(import_statement source: (_) @import_path)"#,
            "java" => r#"(import_declaration name: (_) @import_path)"#,
            "go" => r#"(import_spec path: (_) @import_path)"#,
            "cpp" | "c" => r#"(preproc_include path: (_) @import_path)"#,
            "ruby" => {
                r#"(call
                    method: (identifier) @method (#eq? @method "require")
                    arguments: (argument_list (_) @import_path))"#
            }
            "php" => r#"(namespace_use_declaration (_) @import_path)"#,
            _ => return modules,
        };

        if let Ok(query) = Query::new(&language, query_str) {
            let mut query_cursor = QueryCursor::new();
            let path_idx = query.capture_index_for_name("import_path").unwrap_or(0);

            let mut matches =
                query_cursor.matches(&query, tree.root_node(), self.source_code.as_bytes());
            matches.advance();

            while let Some(match_result) = matches.get() {
                for capture in match_result.captures {
                    if capture.index == path_idx {
                        let path_text = self.get_node_text(capture.node);

                        if let Some(module_name) = extract_module_name_from_path(&path_text) {
                            if !module_name.is_empty() {
                                modules.insert(module_name);
                            }
                        }
                    }
                }
                matches.advance();
            }
        }

        modules
    }

    fn determine_reference_context(&self, node: Node) -> String {
        match node.kind() {
            "call_expression" => "Call".to_string(),
            "function_call" => "Call".to_string(),
            "method_invocation" => "Call".to_string(),
            "import_statement" => "Import".to_string(),
            "use_declaration" => "Import".to_string(),
            "assignment_expression" => "Assignment".to_string(),
            "variable_declarator" => "Declaration".to_string(),
            "parameter" => "Parameter".to_string(),
            "argument" => "Argument".to_string(),
            "member_expression" => "MemberAccess".to_string(),
            "field_expression" => "FieldAccess".to_string(),
            "binary_expression" => "BinaryOperation".to_string(),
            "return_statement" => "Return".to_string(),
            _ => node.kind().to_string(),
        }
    }
}
fn extract_module_name_from_path(path: &str) -> Option<String> {
    let path = path.trim_matches(|c| c == '"' || c == '\'' || c == '`');

    let components: Vec<&str> = path.split(&['/', '\\', '.', ':', ';']).collect();
    let last_component = components.last()?;

    let module_name: String = last_component
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if module_name.is_empty() {
        None
    } else {
        Some(module_name)
    }
}

fn get_function_query(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some("(function_item) @node"),
        "python" => Some("(function_definition) @node"),
        "javascript" | "typescript" | "tsx" => {
            Some("[(function_declaration) (method_definition)] @node")
        }
        "java" => Some("(method_declaration) @node"),
        "go" => Some("(function_declaration) @node"),
        "cpp" | "c" => Some("(function_definition) @node"),
        "ruby" => Some("(method) @node"),
        "php" => Some("(function_definition) @node"),
        _ => None,
    }
}

fn get_class_query(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some("(struct_item) @node"),
        "python" => Some("(class_definition) @node"),
        "javascript" | "typescript" | "tsx" => Some("(class_declaration) @node"),
        "java" => Some("(class_declaration) @node"),
        "go" => Some("(type_spec) @node"),
        "cpp" | "c" => Some("[(class_specifier) (struct_specifier)] @node"),
        "ruby" => Some("(class) @node"),
        "php" => Some("(class_declaration) @node"),
        _ => None,
    }
}

fn get_method_query(language: &str) -> Option<&'static str> {
    match language {
        // Simple query that just captures function nodes - we'll determine if they're methods later
        "rust" => Some("(function_item) @node"),
        "python" => Some("(function_definition) @node"),
        "javascript" | "typescript" | "tsx" => Some("(method_definition) @node"),
        "java" => Some("(method_declaration) @node"),
        "go" => Some("(method_declaration) @node"),
        "cpp" | "c" => Some("(function_definition) @node"),
        "ruby" => Some("(method) @node"),
        "php" => Some("(method_declaration) @node"),
        _ => None,
    }
}

fn get_variable_query(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some(
            "
            [(let_declaration pattern: (identifier) @node)
             (const_item name: (identifier) @node)
             (static_item name: (identifier) @node)]
        ",
        ),
        "python" => Some(
            "
            [(assignment left: (identifier) @node)
             (assignment left: (pattern_list (identifier) @node))]
        ",
        ),
        "javascript" | "typescript" | "tsx" => Some(
            "
            [(variable_declarator name: (identifier) @node)
             (lexical_declaration (variable_declarator name: (identifier) @node))]
        ",
        ),
        "java" => Some(
            "
            [(variable_declarator name: (identifier) @node)
             (field_declaration (variable_declarator name: (identifier) @node))]
        ",
        ),
        "go" => Some(
            "
            [(var_declaration (var_spec name: (identifier) @node))
             (const_declaration (const_spec name: (identifier) @node))]
        ",
        ),
        "cpp" | "c" => Some(
            "
            [(declaration declarator: (init_declarator declarator: (identifier) @node))
             (declaration declarator: (identifier) @node)]
        ",
        ),
        "ruby" => Some("(assignment left: (identifier) @node)"),
        "php" => Some("(variable_name) @node"),
        _ => None,
    }
}

fn get_call_query(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some(
            r#"
            (call_expression
                function: [
                    (identifier) @function_name
                    (field_expression field: (field_identifier) @function_name)
                    (scoped_identifier name: (identifier) @function_name)
                ]
            )
            "#,
        ),
        "python" => Some(
            "(call function: [
                (identifier) @function_name
                (attribute attribute: (identifier) @function_name)
             ])",
        ),
        "javascript" | "typescript" | "tsx" => Some(
            "(call_expression
                function: [
                    (identifier) @function_name
                    (member_expression property: (property_identifier) @function_name)
                ]
            )",
        ),
        "java" => Some(
            "(method_invocation name: (identifier) @function_name)
             (method_reference name: (identifier) @function_name)",
        ),
        "go" => Some(
            "(call_expression function: [
                (identifier) @function_name
                (selector_expression field: (field_identifier) @function_name)
             ])",
        ),
        "cpp" | "c" => Some(
            "(call_expression function: [
                (identifier) @function_name
                (field_expression field: (field_identifier) @function_name)
             ])",
        ),
        "ruby" => Some(
            "(call method: (identifier) @function_name)
             (method_call method: (identifier) @function_name)",
        ),
        "php" => Some(
            "(function_call_expression function: [
                (name) @function_name
                (member_access_expression name: (name) @function_name)
             ])",
        ),
        _ => None,
    }
}

fn get_import_query(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some(
            r#"
            (use_declaration path: (_) @import)
            (use_declaration (use_tree path: (_) @import))
            "#,
        ),
        "python" => Some(
            "(import_statement name: (_) @import)
             (import_from_statement module_name: (_) @import)",
        ),
        "javascript" | "typescript" | "tsx" => Some(
            "(import_statement source: (_) @import)
             (import_specifier name: (_) @import)",
        ),
        "java" => Some("(import_declaration name: (_) @import)"),
        "go" => Some("(import_spec path: (_) @import)"),
        "cpp" | "c" => Some("(preproc_include path: (_) @import)"),
        "ruby" => Some(
            "(call method: (identifier) @method (#eq? @method \"require\")
                arguments: (argument_list (_) @import))",
        ),
        "php" => Some(
            "(namespace_use_declaration (_) @import)
             (namespace_use_clause name: (_) @import)",
        ),
        _ => None,
    }
}

fn get_reference_query(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some(
            r#"
            (identifier) @reference
            (field_expression field: (field_identifier) @reference)
            (scoped_identifier name: (identifier) @reference)
            "#,
        ),
        "python" => Some(
            "(identifier) @reference
             (attribute attribute: (identifier) @reference)",
        ),
        "javascript" | "typescript" | "tsx" => Some(
            "(identifier) @reference
             (property_identifier) @reference",
        ),
        "java" => Some(
            "(identifier) @reference
             (field_access field: (identifier) @reference)",
        ),
        "go" => Some(
            "(identifier) @reference
             (field_identifier) @reference",
        ),
        "cpp" | "c" => Some(
            "(identifier) @reference
             (field_identifier) @reference",
        ),
        "ruby" => Some(
            "(identifier) @reference
             (constant) @reference",
        ),
        "php" => Some(
            "(name) @reference
             (variable_name) @reference",
        ),
        _ => None,
    }
}
