mod node;
mod relationship;

pub use node::{CodeNode, NodeType};
pub use relationship::{Relationship, RelationshipType};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeGraph {
    nodes: HashMap<String, CodeNode>,
    outgoing_edges: HashMap<String, Vec<Relationship>>,
    incoming_edges: HashMap<String, Vec<Relationship>>,

    nodes_by_type: HashMap<NodeType, HashSet<String>>,
    nodes_by_file: HashMap<String, HashSet<String>>,
    nodes_by_name: HashMap<String, HashSet<String>>,
}

impl CodeGraph {
    pub fn new() -> Self {
        CodeGraph {
            nodes: HashMap::new(),
            outgoing_edges: HashMap::new(),
            incoming_edges: HashMap::new(),
            nodes_by_type: HashMap::new(),
            nodes_by_file: HashMap::new(),
            nodes_by_name: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: CodeNode) {
        self.nodes_by_type
            .entry(node.node_type.clone())
            .or_insert_with(HashSet::new)
            .insert(node.id.clone());

        self.nodes_by_file
            .entry(node.file_path.clone())
            .or_insert_with(HashSet::new)
            .insert(node.id.clone());

        self.nodes_by_name
            .entry(node.name.clone())
            .or_insert_with(HashSet::new)
            .insert(node.id.clone());

        self.outgoing_edges
            .entry(node.id.clone())
            .or_insert_with(Vec::new);
        self.incoming_edges
            .entry(node.id.clone())
            .or_insert_with(Vec::new);

        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.outgoing_edges
            .entry(relationship.from_id.clone())
            .or_insert_with(Vec::new)
            .push(relationship.clone());

        self.incoming_edges
            .entry(relationship.to_id.clone())
            .or_insert_with(Vec::new)
            .push(relationship);
    }

    pub fn find_callers(&self, node_id: &str) -> Vec<&CodeNode> {
        if let Some(incoming) = self.incoming_edges.get(node_id) {
            incoming
                .iter()
                .filter(|rel| rel.relationship_type == RelationshipType::Calls)
                .filter_map(|rel| self.nodes.get(&rel.from_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_called_functions(&self, node_id: &str) -> Vec<&CodeNode> {
        if let Some(outgoing) = self.outgoing_edges.get(node_id) {
            outgoing
                .iter()
                .filter(|rel| rel.relationship_type == RelationshipType::Calls)
                .filter_map(|rel| self.nodes.get(&rel.to_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_nodes_by_type(&self, node_type: &NodeType) -> Vec<&CodeNode> {
        if let Some(ids) = self.nodes_by_type.get(node_type) {
            ids.iter().filter_map(|id| self.nodes.get(id)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_nodes_by_name(&self, name: &str) -> Vec<&CodeNode> {
        self.nodes_by_name
            .get(name)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_else(Vec::new)
    }

    pub fn find_nodes_in_file(&self, file_path: &str) -> Vec<&CodeNode> {
        self.nodes_by_file
            .get(file_path)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_else(Vec::new)
    }

    pub fn find_related_nodes(&self, node_id: &str, depth: usize) -> HashSet<&CodeNode> {
        let mut result = HashSet::new();
        let mut to_visit = vec![(node_id.to_string(), 0)];
        let mut visited = HashSet::new();

        while let Some((current_id, current_depth)) = to_visit.pop() {
            if current_depth > depth || visited.contains(&current_id) {
                continue;
            }

            visited.insert(current_id.clone());

            if let Some(node) = self.nodes.get(&current_id) {
                result.insert(node);

                if current_depth < depth {
                    if let Some(outgoing) = self.outgoing_edges.get(&current_id) {
                        for rel in outgoing {
                            to_visit.push((rel.to_id.clone(), current_depth + 1));
                        }
                    }

                    if let Some(incoming) = self.incoming_edges.get(&current_id) {
                        for rel in incoming {
                            to_visit.push((rel.from_id.clone(), current_depth + 1));
                        }
                    }
                }
            }
        }

        result
    }

    pub fn get_node(&self, id: &str) -> Option<&CodeNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut CodeNode> {
        self.nodes.get_mut(id)
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &CodeNode> {
        self.nodes.values()
    }

    pub fn all_nodes_mut(&mut self) -> impl Iterator<Item = &mut CodeNode> {
        self.nodes.values_mut()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn relationship_count(&self) -> usize {
        self.outgoing_edges.values().map(|v| v.len()).sum()
    }
}
