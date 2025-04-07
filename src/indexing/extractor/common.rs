use crate::code_graph::{CodeNode, NodeType};
use log::warn;
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};
use uuid::Uuid;

// Helper functions shared by multiple language extractors

pub fn get_node_text(node: Node, source: &str) -> String {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if start_byte >= source.len() || end_byte > source.len() {
        return String::new();
    }

    source[start_byte..end_byte].to_string()
}

pub fn extract_module_name_from_path(path: &str) -> String {
    // Remove quotes
    let path = path.trim_matches(|c| c == '"' || c == '\'' || c == '`');

    // Get the last component of the path
    let components: Vec<&str> = path.split(&['/', '\\', '.', ':', ';']).collect();
    let last_component = components.last().unwrap_or(&path);

    // Remove any remaining non-alphanumeric parts
    let module_name: String = last_component
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    module_name
}

pub fn create_node(
    node_type: NodeType,
    name: String,
    file_path: &str,
    line_range: (usize, usize),
    content: String,
) -> CodeNode {
    CodeNode::new(
        Uuid::new_v4().to_string(),
        node_type,
        name,
        file_path.to_string(),
        line_range,
        content,
    )
}

pub fn parse_with_tree_sitter(content: &str, file_path: &Path) -> Option<(Tree, String)> {
    let mut parser = crate::parsers::treesitter::TreeSitterParser::new();
    parser.parse_file(file_path, content)
}

pub fn execute_query<'a>(
    query_str: &str,
    tree: &'a Tree,
    source: &'a [u8],
    capture_name: &str,
) -> Vec<Node<'a>> {
    let mut result = Vec::new();

    if let Ok(query) = Query::new(&tree.language(), query_str) {
        let mut query_cursor = QueryCursor::new();
        let capture_idx = query.capture_index_for_name(capture_name).unwrap_or(0);

        let mut matches = query_cursor.matches(&query, tree.root_node(), source);

        // Use the streaming iterator properly
        while let Some(match_result) = matches.next() {
            for capture in match_result.captures {
                if capture.index == capture_idx {
                    result.push(capture.node);
                }
            }
        }
    } else {
        warn!("Failed to create query: {}", query_str);
    }

    result
}
