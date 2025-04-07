use crate::code_graph::{CodeNode, NodeType};
use crate::indexing::extractor::{LanguageExtractor, common};
use crate::parsers::treesitter::queries::javascript as queries;
use log::warn;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Node;

pub struct JavaScriptExtractor;

impl JavaScriptExtractor {
    pub fn new() -> Self {
        JavaScriptExtractor
    }

    fn find_node_name(&self, node: Node, source: &str, node_type: &NodeType) -> Option<String> {
        match node_type {
            NodeType::Function => {
                // Check for function name
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }

                // Handle anonymous function assigned to variable
                if let Some(parent) = node.parent() {
                    if parent.kind() == "variable_declarator"
                        || parent.kind() == "assignment_expression"
                    {
                        for i in 0..parent.named_child_count() {
                            if let Some(child) = parent.named_child(i) {
                                if child.kind() == "identifier"
                                    && child.start_byte() < node.start_byte()
                                {
                                    return Some(common::get_node_text(child, source));
                                }
                            }
                        }
                    }
                }

                // Handle exports
                if let Some(parent) = node.parent() {
                    if parent.kind() == "export_statement" {
                        return Some("exported_function".to_string());
                    }
                }

                // For anonymous functions
                return Some("anonymous".to_string());
            }
            NodeType::Method => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "property_identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }
            }
            NodeType::Class => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }

                // Handle anonymous class assigned to variable
                if let Some(parent) = node.parent() {
                    if parent.kind() == "variable_declarator"
                        || parent.kind() == "assignment_expression"
                    {
                        for i in 0..parent.named_child_count() {
                            if let Some(child) = parent.named_child(i) {
                                if child.kind() == "identifier"
                                    && child.start_byte() < node.start_byte()
                                {
                                    return Some(common::get_node_text(child, source));
                                }
                            }
                        }
                    }
                }

                return Some("AnonymousClass".to_string());
            }
            _ => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "identifier" || child.kind() == "property_identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }
            }
        }

        None
    }

    fn find_parent_class(&self, method_node: Node, source: &str) -> Option<String> {
        let mut current = method_node;
        let mut parent_iter = current.parent();

        while let Some(parent) = parent_iter {
            if parent.kind() == "class_body" {
                if let Some(class_node) = parent.parent() {
                    if class_node.kind() == "class_declaration" {
                        for i in 0..class_node.named_child_count() {
                            if let Some(child) = class_node.named_child(i) {
                                if child.kind() == "identifier" {
                                    return Some(common::get_node_text(child, source));
                                }
                            }
                        }
                    }
                }
            }

            current = parent;
            parent_iter = current.parent();
        }

        None
    }
}

impl LanguageExtractor for JavaScriptExtractor {
    fn extract_code_units(&self, content: &str, file_path: &Path) -> Vec<CodeNode> {
        let mut code_units = Vec::new();

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, file_path) {
            // Extract functions
            let function_nodes =
                common::execute_query(queries::FUNCTION_QUERY, &tree, content.as_bytes(), "node");

            for node in function_nodes {
                if let Some(name) = self.find_node_name(node, content, &NodeType::Function) {
                    let start_line = node.start_position().row + 1;
                    let end_line = node.end_position().row + 1;
                    let node_content = common::get_node_text(node, content);

                    let code_node = common::create_node(
                        NodeType::Function,
                        name,
                        file_path.to_str().unwrap_or(""),
                        (start_line, end_line),
                        node_content,
                    );

                    code_units.push(code_node);
                }
            }

            // Extract methods
            let method_nodes =
                common::execute_query(queries::METHOD_QUERY, &tree, content.as_bytes(), "node");

            for node in method_nodes {
                if let Some(name) = self.find_node_name(node, content, &NodeType::Method) {
                    let start_line = node.start_position().row + 1;
                    let end_line = node.end_position().row + 1;
                    let node_content = common::get_node_text(node, content);

                    let mut metadata = HashMap::new();

                    if let Some(parent_class) = self.find_parent_class(node, content) {
                        metadata.insert("parent_class".to_string(), parent_class);
                    }

                    let mut code_node = common::create_node(
                        NodeType::Method,
                        name,
                        file_path.to_str().unwrap_or(""),
                        (start_line, end_line),
                        node_content,
                    );

                    for (key, value) in metadata {
                        code_node.add_metadata(key, value);
                    }

                    code_units.push(code_node);
                }
            }

            // Extract classes
            let class_nodes =
                common::execute_query(queries::CLASS_QUERY, &tree, content.as_bytes(), "node");

            for node in class_nodes {
                if let Some(name) = self.find_node_name(node, content, &NodeType::Class) {
                    let start_line = node.start_position().row + 1;
                    let end_line = node.end_position().row + 1;
                    let node_content = common::get_node_text(node, content);

                    let code_node = common::create_node(
                        NodeType::Class,
                        name,
                        file_path.to_str().unwrap_or(""),
                        (start_line, end_line),
                        node_content,
                    );

                    code_units.push(code_node);
                }
            }
        } else {
            warn!("Failed to parse JavaScript file: {:?}", file_path);
        }

        code_units
    }

    fn extract_function_calls(
        &self,
        content: &str,
        func_range: (usize, usize),
        _func_name: &str,
    ) -> Vec<String> {
        let mut calls = Vec::new();

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.js")) {
            let call_nodes =
                common::execute_query(queries::CALL_QUERY, &tree, content.as_bytes(), "func_name");

            for node in call_nodes {
                let call_line = node.start_position().row + 1;

                // Check if call is within function range
                if call_line >= func_range.0 && call_line <= func_range.1 {
                    let call_name = common::get_node_text(node, content);
                    if !call_name.is_empty() {
                        calls.push(call_name);
                    }
                }
            }
        }

        calls
    }

    fn extract_variable_references(
        &self,
        content: &str,
        func_range: (usize, usize),
        var_name: &str,
    ) -> Vec<(usize, usize)> {
        let mut references = Vec::new();

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.js")) {
            let reference_nodes = common::execute_query(
                queries::REFERENCE_QUERY,
                &tree,
                content.as_bytes(),
                "reference",
            );

            for node in reference_nodes {
                let ref_line = node.start_position().row + 1;

                // Check if reference is within function range
                if ref_line >= func_range.0 && ref_line <= func_range.1 {
                    let ref_name = common::get_node_text(node, content);
                    if ref_name == var_name {
                        references.push((ref_line, node.end_position().row + 1));
                    }
                }
            }
        }
        references
    }

    fn extract_imported_modules(&self, content: &str) -> Vec<String> {
        let mut modules = Vec::new();

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.js")) {
            let import_nodes = common::execute_query(
                queries::IMPORT_QUERY,
                &tree,
                content.as_bytes(),
                "import_path",
            );

            for node in import_nodes {
                let import_text = common::get_node_text(node, content);
                let cleaned_text = import_text.trim_matches(|c| c == '"' || c == '\'' || c == '`');

                // Extract module name from path
                let parts: Vec<&str> = cleaned_text.split('/').collect();
                if let Some(last) = parts.last() {
                    let module_name = last.trim_end_matches(".js");
                    if !module_name.is_empty() {
                        modules.push(module_name.to_string());
                    }
                }
            }
        }

        modules
    }
}
