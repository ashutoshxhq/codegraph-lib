use crate::code_graph::{CodeGraph, NodeType, Relationship, RelationshipType};
use log::{debug, info, trace, warn};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn identify_relationships(graph: &mut CodeGraph) {
    info!("Identifying precise relationships between code units...");
    let mut relationships_to_add = Vec::new();

    // Group nodes by file for more efficient processing
    let mut nodes_by_file: HashMap<String, Vec<(String, String, NodeType)>> = HashMap::new();

    for node in graph.all_nodes() {
        // Skip very short node names (minimum length 3 characters)
        if node.name.len() < 3 {
            continue;
        }

        nodes_by_file
            .entry(node.file_path.clone())
            .or_insert_with(Vec::new)
            .push((node.id.clone(), node.name.clone(), node.node_type.clone()));
    }

    let file_count = nodes_by_file.len();
    info!("Processing {} files for relationship detection", file_count);

    // Process each file to find relationships
    for (file_idx, (file_path, nodes)) in nodes_by_file.iter().enumerate() {
        if nodes.is_empty() {
            continue;
        }

        if file_idx % 10 == 0 {
            debug!(
                "Processing file {}/{}: {}",
                file_idx + 1,
                file_count,
                file_path
            );
        }

        let file_path_obj = Path::new(file_path);

        // Skip file processing if it can't be read
        let content = match read_file_content(file_path_obj) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read file {}: {}", file_path, e);
                continue;
            }
        };

        // Detect language and process accordingly
        if let Some(language) = crate::parsers::detect_language(file_path_obj) {
            // Find function call relationships
            find_function_call_relationships(
                &language,
                file_path,
                &content,
                &nodes,
                graph,
                &mut relationships_to_add,
            );

            // Find import relationships
            find_import_relationships(
                &language,
                file_path,
                &content,
                &nodes,
                graph,
                &mut relationships_to_add,
            );

            // Find hierarchical relationships
            find_hierarchical_relationships(&nodes, graph, &mut relationships_to_add);
        } else {
            warn!("Could not determine language for file: {}", file_path);
        }
    }

    // Add this new function call
    find_method_class_relationships(graph, &mut relationships_to_add);

    info!(
        "Adding {} precisely identified relationships",
        relationships_to_add.len()
    );

    // Add all unique relationships to the graph
    let mut added_rels = HashSet::new();
    for rel in relationships_to_add {
        let rel_key = (
            rel.from_id.clone(),
            rel.to_id.clone(),
            rel.relationship_type.clone(),
        );
        if !added_rels.contains(&rel_key) {
            graph.add_relationship(rel);
            added_rels.insert(rel_key);
        }
    }

    info!("Relationship identification complete");
}

// Helper function to read file content
fn read_file_content(file_path: &Path) -> std::io::Result<String> {
    let mut content = String::new();
    let mut file = File::open(file_path)?;
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn find_function_call_relationships(
    language: &str,
    _file_path: &str,
    content: &str,
    nodes: &[(String, String, NodeType)],
    graph: &CodeGraph,
    relationships: &mut Vec<Relationship>,
) {
    // Get function nodes in this file
    let functions_in_file: Vec<_> = nodes
        .iter()
        .filter(|(_, _, node_type)| matches!(node_type, NodeType::Function | NodeType::Method))
        .collect();

    if functions_in_file.is_empty() {
        return;
    }

    // Create a map of function names to their IDs for quick lookup
    let mut function_map: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in graph.all_nodes() {
        if matches!(node.node_type, NodeType::Function | NodeType::Method) && node.name.len() >= 3 {
            function_map
                .entry(node.name.as_str())
                .or_insert_with(Vec::new)
                .push(node.id.as_str());
        }
    }

    // Use language-specific extractor to find function calls
    if let Some(extractor) = crate::indexing::extractor::get_extractor_for_language(language) {
        for (func_id, func_name, _) in &functions_in_file {
            if let Some(func_node) = graph.get_node(func_id) {
                // Get function's line range
                let func_range = func_node.line_range;

                // Find all function calls within this function
                let function_calls =
                    extractor.extract_function_calls(content, func_range, func_name.as_str());

                // Map function calls to relationships
                for called_func_name in function_calls {
                    if called_func_name.len() < 3 {
                        continue;
                    }

                    if let Some(target_ids) = function_map.get(called_func_name.as_str()) {
                        for target_id in target_ids {
                            // Skip self-calls
                            if func_id == *target_id {
                                continue;
                            }

                            trace!("Found function call: {} -> {}", func_name, called_func_name);
                            relationships.push(Relationship::new(
                                RelationshipType::Calls,
                                func_id.clone(),
                                (*target_id).to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn find_import_relationships(
    language: &str,
    file_path: &str,
    content: &str,
    nodes: &[(String, String, NodeType)],
    graph: &CodeGraph,
    relationships: &mut Vec<Relationship>,
) {
    if let Some(extractor) = crate::indexing::extractor::get_extractor_for_language(language) {
        // Extract all imported modules from this file
        let imported_modules = extractor.extract_imported_modules(content);

        if imported_modules.is_empty() {
            return;
        }

        // Get nodes from this file
        let current_file_nodes: Vec<_> = nodes.iter().map(|(id, _, _)| id.clone()).collect();

        // For each imported module, find matching nodes in the graph
        for module_name in imported_modules {
            // Find potential target modules/classes
            for node in graph.all_nodes() {
                if !matches!(
                    node.node_type,
                    NodeType::Module | NodeType::Class | NodeType::Interface
                ) {
                    continue;
                }

                // Skip nodes in the same file
                if current_file_nodes.contains(&node.id) {
                    continue;
                }

                // Check if this node matches the import
                if node.name == module_name
                    || Path::new(&node.file_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s == module_name)
                        .unwrap_or(false)
                {
                    // Add import relationship from each node in current file
                    for (source_id, _, _) in nodes {
                        trace!("Found import from {} to {}", file_path, node.name);
                        relationships.push(Relationship::new(
                            RelationshipType::Imports,
                            source_id.clone(),
                            node.id.clone(),
                        ));
                    }

                    break;
                }
            }
        }
    }
}

fn find_method_class_relationships(graph: &CodeGraph, relationships: &mut Vec<Relationship>) {
    // Find methods with parent_class metadata
    for node in graph.all_nodes() {
        if node.node_type == NodeType::Method {
            if let Some(parent_class) = node.metadata.get("parent_class") {
                // Find all classes with this name
                let potential_classes = graph.find_nodes_by_name(parent_class);

                for class_node in potential_classes {
                    if class_node.node_type == NodeType::Class
                        || class_node.node_type == NodeType::Interface
                    {
                        trace!(
                            "Found method-class relationship: {} belongs to {}",
                            node.name, class_node.name
                        );

                        // Add relationship from class to method (containment)
                        relationships.push(Relationship::new(
                            RelationshipType::Contains,
                            class_node.id.clone(),
                            node.id.clone(),
                        ));
                    }
                }
            }
        }
    }
}

fn find_hierarchical_relationships(
    nodes: &[(String, String, NodeType)],
    graph: &CodeGraph,
    relationships: &mut Vec<Relationship>,
) {
    let classes: Vec<_> = nodes
        .iter()
        .filter(|(_, _, node_type)| matches!(node_type, NodeType::Class | NodeType::Interface))
        .filter_map(|(id, _, _)| graph.get_node(id))
        .collect();

    if classes.len() <= 1 {
        return;
    }

    // Sort classes by line number
    let mut sorted_classes = classes;
    sorted_classes.sort_by_key(|node| node.line_range.0);

    // Check for containment/nesting
    for i in 0..sorted_classes.len() {
        let outer = &sorted_classes[i];
        let outer_range = outer.line_range;

        for j in 0..sorted_classes.len() {
            if i == j {
                continue;
            }

            let inner = &sorted_classes[j];
            let inner_range = inner.line_range;

            // If inner class is contained within outer class
            if inner_range.0 > outer_range.0 && inner_range.1 < outer_range.1 {
                trace!(
                    "Found class containment: {} contains {}",
                    outer.name, inner.name
                );
                relationships.push(Relationship::new(
                    RelationshipType::Contains,
                    outer.id.clone(),
                    inner.id.clone(),
                ));
            }
        }
    }
}

pub fn generate_summaries(graph: &mut CodeGraph) {
    info!("Generating summaries for {} nodes", graph.node_count());

    let mut summary_counts = HashMap::new();

    for node in graph.all_nodes_mut() {
        let node_type = &node.node_type;

        let summary = match node_type {
            NodeType::Function => format!("Function that handles {}", node.name),
            NodeType::Method => format!("Method that implements {}", node.name),
            NodeType::Class => format!("Class that represents {}", node.name),
            NodeType::Interface => format!("Interface for {}", node.name),
            NodeType::Module => format!("Module containing {}", node.name),
            NodeType::TypeDefinition => format!("Type definition for {}", node.name),
            _ => format!("Code unit: {}", node.name),
        };

        node.summary = Some(summary);

        *summary_counts.entry(node_type.clone()).or_insert(0) += 1;
    }

    for (node_type, count) in summary_counts {
        debug!("Generated {:?} summaries for {} nodes", node_type, count);
    }

    info!("Summary generation complete");
}

pub fn enhance_method_names(graph: &mut CodeGraph) {
    info!("Enhancing method names with parent class information...");
    let mut methods_to_update = Vec::new();

    // First collect all methods that need updating
    for node in graph.all_nodes() {
        if node.node_type == NodeType::Method {
            if let Some(parent_class) = node.metadata.get("parent_class") {
                methods_to_update
                    .push((node.id.clone(), format!("{}::{}", parent_class, node.name)));
            }
        }
    }

    // Now update the methods with enhanced names
    for (id, enhanced_name) in methods_to_update {
        if let Some(node) = graph.get_node_mut(&id) {
            debug!(
                "Updating method name from '{}' to '{}'",
                node.name, enhanced_name
            );
            node.name = enhanced_name;
        }
    }

    info!("Method names enhancement complete");
}
