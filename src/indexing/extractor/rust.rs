use crate::code_graph::{CodeNode, NodeType};
use crate::indexing::extractor::{LanguageExtractor, common};
use crate::parsers::treesitter::queries::rust as queries;
use log::warn;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, StreamingIterator};

pub struct RustExtractor;

impl RustExtractor {
    pub fn new() -> Self {
        RustExtractor
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
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "type_identifier" {
                            return Some(common::get_node_text(child, source));
                        }
                    }
                }
            }
            _ => {
                // Generic name finder
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

    fn is_inside_impl_block(&self, node: Node) -> bool {
        let mut current = node;
        let mut parent_iter = current.parent();

        while let Some(parent) = parent_iter {
            if parent.kind() == "impl_item" {
                return true;
            }

            current = parent;
            parent_iter = current.parent();
        }

        false
    }

    fn find_parent_class(&self, node: Node, source: &str) -> Option<String> {
        let mut current = node;
        let mut parent_iter = current.parent();

        while let Some(parent) = parent_iter {
            if parent.kind() == "impl_item" {
                for i in 0..parent.named_child_count() {
                    if let Some(child) = parent.named_child(i) {
                        if child.kind() == "type_identifier" {
                            return Some(common::get_node_text(child, source));
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

impl LanguageExtractor for RustExtractor {
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

                    let is_method = self.is_inside_impl_block(node);
                    let node_type = if is_method {
                        NodeType::Method
                    } else {
                        NodeType::Function
                    };

                    let mut metadata = HashMap::new();

                    if is_method {
                        if let Some(parent_class) = self.find_parent_class(node, content) {
                            metadata.insert("parent_class".to_string(), parent_class);
                        }
                    }

                    let mut code_node = common::create_node(
                        node_type,
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

            // Extract structs/classes
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
            warn!("Failed to parse Rust file: {:?}", file_path);
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

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.rs")) {
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

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.rs")) {
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

        if let Some((tree, _)) = common::parse_with_tree_sitter(content, Path::new("temp.rs")) {
            // Use a simpler approach with a basic query
            let query_str = "(use_declaration) @import";

            if let Ok(query) = tree_sitter::Query::new(&tree.language(), query_str) {
                let mut query_cursor = tree_sitter::QueryCursor::new();
                let mut matches =
                    query_cursor.matches(&query, tree.root_node(), content.as_bytes());

                // Use streaming iterator pattern
                matches.advance();
                while let Some(match_) = matches.get() {
                    for capture in match_.captures {
                        let node = capture.node;
                        let import_text = common::get_node_text(node, content);

                        // Extract just the first segment of the path
                        if let Some(first_segment) = import_text
                            .trim_start_matches("use ")
                            .trim_end_matches(';')
                            .split("::")
                            .next()
                        {
                            let module_name = first_segment.trim().to_string();
                            if !module_name.is_empty() {
                                modules.push(module_name);
                            }
                        }
                    }
                    matches.advance();
                }
            }
        }

        modules
    }
}
