use crate::code_graph::CodeGraph;
use log::{error, info};
use std::fs;
use std::io::{self};
use std::path::Path;

pub fn export_graph_to_json(graph: &CodeGraph, output_path: &Path) -> io::Result<()> {
    info!(
        "Exporting graph with {} nodes and {} relationships to JSON: {:?}",
        graph.node_count(),
        graph.relationship_count(),
        output_path
    );

    let json = match serde_json::to_string_pretty(graph) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize graph to JSON: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }
    };

    match fs::write(output_path, json.clone()) {
        Ok(_) => {
            info!(
                "Successfully wrote {} bytes to {:?}",
                json.len(),
                output_path
            );
            Ok(())
        }
        Err(e) => {
            error!("Failed to write JSON to file {:?}: {}", output_path, e);
            Err(e)
        }
    }
}
