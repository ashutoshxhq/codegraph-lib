use crate::code_graph::{CodeNode, NodeType};
use crate::indexing::extractor::{LanguageExtractor, common};
use crate::parsers::treesitter::queries::go as queries;
use log::warn;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Node;

pub struct GoExtractor;

impl GoExtractor {
    pub fn new() -> Self {
        GoExtractor
    }

    fn find_node_name(&self, node: Node, source: &str, node_type: &NodeType) -> Option<String> {
        match node_type {
            NodeType::Function | NodeType::Method => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }
            }
            NodeType::Class => {
                // Go doesn't have classes, but we extract struct types
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "type_identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }
            }
            _ => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "identifier" || child.kind() == "type_identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }
            }
        }

        None
    }

    fn is_method(&self, node: Node) -> bool {
        node.kind() == "method_declaration"
    }

    fn find_receiver_type(&self, node: Node, source: &str) -> Option<String> {
        if !self.is_method(node) {
            return None;
        }

        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == "parameter_list" {
                    for j in 0..child.named_child_count() {
                        if let Some(param) = child.named_child(j) {
                            if param.kind() == "parameter_declaration" {
                                for k in 0..param.named_child_count() {
                                    if let Some(type_node) = param.named_child(k) {
                                        if type_node.kind() == "type_identifier"
                                            || type_node.kind() == "pointer_type"
                                        {
                                            let type_text =
                                                common::get_node_text(type_node, source);
                                            // Handle pointer receivers like (*T)
                                            return Some(
                                                type_text
                                                    .trim_start_matches('*')
                                                    .trim_matches('(')
                                                    .trim_matches(')')
                                                    .to_string(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }

        None
    }
}

impl LanguageExtractor for GoExtractor {
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

                    let node_type = NodeType::Function;

                    let code_node = common::create_node(
                        node_type,
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

                    if let Some(receiver_type) = self.find_receiver_type(node, content) {
                        metadata.insert("parent_class".to_string(), receiver_type);
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

            // Extract structs as "classes"
            let struct_nodes =
                common::execute_query(queries::CLASS_QUERY, &tree, content.as_bytes(), "node");

            for node in struct_nodes {
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
            warn!("Failed to parse Go file: {:?}", file_path);
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

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.go")) {
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

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.go")) {
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

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.go")) {
            let import_nodes = common::execute_query(
                queries::IMPORT_QUERY,
                &tree,
                content.as_bytes(),
                "import_path",
            );

            for node in import_nodes {
                let import_text = common::get_node_text(node, content);
                let module_name = common::extract_module_name_from_path(&import_text);

                if !module_name.is_empty() {
                    modules.push(module_name);
                }
            }
        }

        modules
    }
}
